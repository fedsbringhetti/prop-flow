//! Supporto ai test dei bordi HTTP.
//!
//! Il router viene montato in-process e interrogato con `tower::ServiceExt::oneshot`:
//! nessuna porta aperta, nessun client HTTP, nessuna attesa di avvio. Il database
//! però è reale, perché la Row Level Security fa parte del comportamento che questi
//! test verificano.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use chrono::Duration;
use propflow_api::state::{AppState, Dependencies};
use propflow_application::ports::{ObjectStorage, StorageError, StoredObject};
use propflow_infrastructure::db::PoolSettings;
use propflow_infrastructure::security::{
    Argon2PasswordHasher, EdDsaTokenIssuer, JwtSettings, OpaqueRefreshTokenFactory,
    OpaqueTokenGenerator,
};
use propflow_infrastructure::{DatabasePools, PgUnitOfWorkFactory, SystemClock};
use tower::ServiceExt as _;

/// Doppio in memoria di [`ObjectStorage`].
///
/// I test dei bordi verificano il **contratto HTTP**, non S3: un MinIO acceso
/// renderebbe la suite dipendente da un secondo servizio per verificare codici di
/// stato e autorizzazione, che con S3 non hanno nulla a che vedere. L'adattatore
/// vero ha i propri test, e uno contro MinIO gira solo se l'ambiente c'è.
///
/// Riproduce le due proprietà su cui il resto del codice fa affidamento: il
/// checksum lo calcola **chi scrive**, e l'URL pre-firmato è opaco e a scadenza.
#[derive(Default)]
pub struct InMemoryObjectStorage {
    objects: Mutex<HashMap<String, Vec<u8>>>,
}

impl InMemoryObjectStorage {
    /// Numero di oggetti presenti: serve a verificare che un caricamento rifiutato
    /// non lasci byte orfani nel bucket.
    pub fn len(&self) -> usize {
        self.objects.lock().map(|map| map.len()).unwrap_or_default()
    }

    /// Chiavi presenti: servono a verificare che il nome del file non ci finisca.
    pub fn keys(&self) -> Vec<String> {
        self.objects
            .lock()
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default()
    }
}

#[async_trait::async_trait]
impl ObjectStorage for InMemoryObjectStorage {
    async fn put(
        &self,
        key: &str,
        _content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<StoredObject, StorageError> {
        use sha2::{Digest as _, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let checksum: [u8; 32] = hasher.finalize().into();
        let size_bytes = bytes.len() as u64;

        self.objects
            .lock()
            .map_err(|_| StorageError::Backend {
                details: "lock avvelenato".to_owned(),
            })?
            .insert(key.to_owned(), bytes);

        Ok(StoredObject {
            provider: "memory".to_owned(),
            bucket: "test".to_owned(),
            key: key.to_owned(),
            size_bytes,
            checksum_sha256: checksum,
            // Nessuna cifratura: il doppio dice il vero, come deve fare
            // l'adattatore vero su un bucket non configurato.
            encryption: None,
        })
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        self.objects
            .lock()
            .map_err(|_| StorageError::Backend {
                details: "lock avvelenato".to_owned(),
            })?
            .get(key)
            .cloned()
            .ok_or(StorageError::NotFound)
    }

    async fn presigned_get_url(&self, key: &str, ttl_seconds: u32) -> Result<String, StorageError> {
        Ok(format!(
            "https://storage.test/test/{key}?X-Amz-Expires={ttl_seconds}&X-Amz-Signature=finto"
        ))
    }
}

/// Parametri di un caricamento multipart.
///
/// Una struct e non sette argomenti: sarebbero quattro `&str` di fila, e due
/// scambiati compilerebbero senza che nulla se ne accorga.
pub struct Upload<'a> {
    pub path: &'a str,
    pub token: &'a str,
    /// `X-Org-Id`, quando serve: l'inquilino ne ha sempre bisogno.
    pub org: Option<&'a str>,
    pub filename: &'a str,
    pub content_type: &'a str,
    pub content: &'a [u8],
    /// Campo `kind` del form. `None` lo omette del tutto, per verificare il
    /// default.
    pub kind: Option<&'a str>,
}

pub struct TestApp {
    pub router: Router,
    /// Lo stesso storage iniettato nell'app: i test lo ispezionano.
    pub storage: Arc<InMemoryObjectStorage>,
    /// I due pool, per i test che devono far girare anche il **worker**.
    ///
    /// Il worker non è montato nel router — non è un bordo HTTP — quindi un test
    /// che voglia vedere il giro completo dell'OCR deve costruirsi il ciclo con il
    /// pool privilegiato. Senza questo campo, il contratto HTTP potrebbe verificare
    /// solo metà del percorso, cioè l'accodamento.
    pub pools: DatabasePools,
}

impl TestApp {
    /// Costruisce l'app, oppure `None` se l'ambiente di test non è configurato.
    pub async fn spawn() -> Option<Self> {
        let (Ok(app_url), Ok(privileged_url), Ok(private_key), Ok(public_key)) = (
            std::env::var("PROPFLOW_TEST_APP_URL"),
            std::env::var("PROPFLOW_TEST_PRIVILEGED_URL"),
            std::env::var("PROPFLOW_TEST_JWT_PRIVATE_KEY"),
            std::env::var("PROPFLOW_TEST_JWT_PUBLIC_KEY"),
        ) else {
            eprintln!("SALTATO: ambiente di test non configurato (ops/ci/test-integration.sh)");
            return None;
        };

        let pools = DatabasePools::connect(&app_url, &privileged_url, &PoolSettings::default())
            .await
            .expect("connessione al database di test");

        let storage = Arc::new(InMemoryObjectStorage::default());
        let dependencies = Dependencies {
            unit_of_work: Arc::new(PgUnitOfWorkFactory::new(pools.clone())),
            hasher: Arc::new(Argon2PasswordHasher::new().expect("Argon2")),
            tokens: Arc::new(
                EdDsaTokenIssuer::new(&JwtSettings {
                    private_key_pem: std::fs::read_to_string(private_key).expect("chiave privata"),
                    public_key_pem: std::fs::read_to_string(public_key).expect("chiave pubblica"),
                    issuer: "propflow".to_owned(),
                    audience: "propflow-api".to_owned(),
                    access_token_lifetime: Duration::minutes(15),
                })
                .expect("chiavi valide"),
            ),
            refresh_tokens: Arc::new(OpaqueRefreshTokenFactory::new(Duration::days(30))),
            invitation_tokens: Arc::new(OpaqueTokenGenerator),
            clock: Arc::new(SystemClock),
            object_storage: Arc::clone(&storage) as Arc<dyn ObjectStorage>,
            presign_ttl_seconds: PRESIGN_TTL_SECONDS,
            ocr_provider_name: propflow_infrastructure::DeterministicOcrProvider::NAME.to_owned(),
            pools: pools.clone(),
            // `false` come in sviluppo: i test dei bordi non parlano HTTPS, e con
            // `Secure` un client vero non rimanderebbe il cookie — cioè il test
            // verificherebbe una configurazione che nessuno usa in quel contesto.
            secure_cookies: false,
        };

        Some(Self {
            pools: pools.clone(),
            router: propflow_api::router(
                AppState::new(dependencies),
                BODY_LIMIT_BYTES,
                UPLOAD_BODY_LIMIT_BYTES,
                std::time::Duration::from_secs(10),
            ),
            storage,
        })
    }

    /// Invia un corpo `multipart/form-data` con un solo file.
    ///
    /// Costruito a mano e non con un client HTTP: il router è montato in-process,
    /// e tirarsi dentro un client per comporre tre righe di corpo aggiungerebbe
    /// una dipendenza per nulla.
    pub async fn upload(&self, request: Upload<'_>) -> (StatusCode, serde_json::Value) {
        let Upload {
            path,
            token,
            org,
            filename,
            content_type,
            content,
            kind,
        } = request;
        const BOUNDARY: &str = "----propflowtestboundary";
        let mut body: Vec<u8> = Vec::new();
        if let Some(kind) = kind {
            body.extend_from_slice(
                format!(
                    "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"kind\"\r\n\r\n{kind}\r\n"
                )
                .as_bytes(),
            );
        }
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; \
                 filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(content);
        body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());

        let mut builder = Request::builder()
            .method("POST")
            .uri(path)
            .header(
                "content-type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .header("authorization", format!("Bearer {token}"));
        if let Some(org) = org {
            builder = builder.header("x-org-id", org);
        }
        let request = builder.body(Body::from(body)).expect("richiesta costruita");
        let (status, json, _) = self.request(request).await;
        (status, json)
    }

    /// Come [`TestApp::send`] ma restituisce anche le intestazioni: serve al
    /// download, dove la risposta interessante è `Location` e non il corpo.
    pub async fn send_raw(
        &self,
        method: &str,
        path: &str,
        token: &str,
        org: Option<&str>,
    ) -> (StatusCode, Response<Body>) {
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header("authorization", format!("Bearer {token}"));
        if let Some(org) = org {
            builder = builder.header("x-org-id", org);
        }
        let request = builder.body(Body::empty()).expect("richiesta costruita");
        let (status, _, response) = self.request(request).await;
        (status, response)
    }

    pub async fn request(
        &self,
        request: Request<Body>,
    ) -> (StatusCode, serde_json::Value, Response<Body>) {
        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .expect("il router ha risposto");
        let status = response.status();
        let (parts, body) = response.into_parts();
        let bytes = axum::body::to_bytes(body, 1024 * 1024)
            .await
            .expect("corpo leggibile");
        let json = if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
        };
        (status, json, Response::from_parts(parts, Body::empty()))
    }

    /// `POST` che porta il cookie di sessione e ne restituisce quello nuovo.
    ///
    /// Il refresh token **non è più nel corpo**: sta in un cookie `httpOnly`, e i
    /// test che verificano la rotazione devono comportarsi come un browser — cioè
    /// rimandare ciò che il `Set-Cookie` precedente ha consegnato. Il terzo
    /// elemento della tupla è quel valore, così una catena di refresh si scrive
    /// senza rileggere gli header a mano ogni volta.
    pub async fn post_with_cookie(
        &self,
        path: &str,
        cookie: Option<&str>,
        token: Option<&str>,
    ) -> (StatusCode, serde_json::Value, Option<String>) {
        let mut builder = Request::builder().method("POST").uri(path);
        if let Some(cookie) = cookie {
            builder = builder.header("cookie", format!("propflow_refresh={cookie}"));
        }
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        let request = builder.body(Body::empty()).expect("richiesta costruita");
        let (status, json, response) = self.request(request).await;
        (status, json, refresh_cookie_of(&response))
    }

    /// Una richiesta con il **solo cookie** e senza `Authorization`.
    ///
    /// Serve a una cosa sola, ed è la ragione per cui esiste: verificare che una
    /// rotta di business non accetti il cookie. Vedi
    /// `the_session_cookie_is_refused_by_business_routes`.
    pub async fn send_with_cookie_only(
        &self,
        method: &str,
        path: &str,
        cookie: &str,
    ) -> (StatusCode, serde_json::Value) {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header("cookie", format!("propflow_refresh={cookie}"))
            .body(Body::empty())
            .expect("richiesta costruita");
        let (status, json, _) = self.request(request).await;
        (status, json)
    }

    pub async fn post(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let (status, json, _) = self.post_returning_cookie(path, body).await;
        (status, json)
    }

    /// Come [`TestApp::post`] ma restituisce anche il cookie di sessione posato
    /// dalla risposta: è così che un test ottiene un refresh token, che dal
    /// corpo non esce più.
    pub async fn post_returning_cookie(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value, Option<String>) {
        let request = Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("richiesta costruita");
        let (status, json, response) = self.request(request).await;
        (status, json, refresh_cookie_of(&response))
    }

    pub async fn post_auth(
        &self,
        path: &str,
        token: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let request = Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(body.to_string()))
            .expect("richiesta costruita");
        let (status, json, _) = self.request(request).await;
        (status, json)
    }

    /// `POST` con l'header `Idempotency-Key`.
    ///
    /// La chiave è `Option` perché una delle cose da verificare è che **assente
    /// dia `400`**: sulle rotte che creano un pagamento l'header è obbligatorio, e
    /// un test che potesse solo mandarlo non proverebbe l'obbligo.
    pub async fn post_idempotent(
        &self,
        path: &str,
        token: &str,
        org: Option<&str>,
        key: Option<&str>,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let mut builder = Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"));
        if let Some(org) = org {
            builder = builder.header("x-org-id", org);
        }
        if let Some(key) = key {
            builder = builder.header("idempotency-key", key);
        }
        let request = builder
            .body(Body::from(body.to_string()))
            .expect("richiesta costruita");
        let (status, json, _) = self.request(request).await;
        (status, json)
    }

    pub async fn send(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        self.send_in_org(method, path, token, None, body).await
    }

    /// Come [`TestApp::send`] ma con l'header `X-Org-Id`.
    ///
    /// Serve all'inquilino: il suo access token porta l'organizzazione personale,
    /// e per leggere i dati dell'organizzazione del proprietario deve dichiararla.
    pub async fn send_in_org(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        org: Option<&str>,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let mut builder = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        if let Some(org) = org {
            builder = builder.header("x-org-id", org);
        }
        let request = match body {
            Some(json) => builder
                .header("content-type", "application/json")
                .body(Body::from(json.to_string())),
            None => builder.body(Body::empty()),
        }
        .expect("richiesta costruita");
        let (status, json, _) = self.request(request).await;
        (status, json)
    }
}

/// Gli stessi limiti che il progetto spedisce in `config/default.toml`: un test
/// che ne usasse di propri verificherebbe una configurazione che non esiste.
pub const BODY_LIMIT_BYTES: usize = 1024 * 1024;
pub const UPLOAD_BODY_LIMIT_BYTES: usize = 25 * 1024 * 1024;
pub const PRESIGN_TTL_SECONDS: u32 = 300;

/// Il valore del cookie `propflow_refresh` posato da una risposta.
///
/// `None` sia quando il `Set-Cookie` manca sia quando cancella il cookie
/// (`Max-Age=0`, valore vuoto): per chi chiama sono la stessa cosa — non c'è una
/// sessione da presentare.
pub fn refresh_cookie_of(response: &Response<Body>) -> Option<String> {
    response
        .headers()
        .get_all(axum::http::header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .filter_map(|value| value.split(';').next())
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| name.trim() == "propflow_refresh")
        .map(|(_, value)| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// Email irripetibile fra esecuzioni.
pub fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@example.test", uuid::Uuid::now_v7().simple())
}

/// Registra un proprietario e restituisce (access_token, org_id).
pub async fn register_and_login(app: &TestApp, prefix: &str) -> (String, String) {
    let (token, org, _) = register_and_login_with_email(app, &unique_email(prefix)).await;
    (token, org)
}

/// Come [`register_and_login`] ma con un'email scelta dal chiamante: serve
/// all'inquilino, che deve registrarsi con **la stessa** email dell'invito.
/// Restituisce (access_token, org_id, email).
pub async fn register_and_login_with_email(app: &TestApp, email: &str) -> (String, String, String) {
    let email = email.to_owned();
    let password = "cavallo batteria graffetta";

    let (status, _) = app
        .post(
            "/api/v1/auth/register",
            serde_json::json!({
                "email": email,
                "full_name": "Proprietario Test",
                "password": password,
            }),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "registrazione non riuscita");

    let (status, body, _) = app
        .post_returning_cookie(
            "/api/v1/auth/login",
            serde_json::json!({ "email": email, "password": password }),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "login non riuscito: {body}");

    let token = body["access_token"]
        .as_str()
        .expect("access token")
        .to_owned();
    let org = body["organization_id"]
        .as_str()
        .expect("organizzazione")
        .to_owned();
    (token, org, email)
}

/// Chiave dell'advisory lock che serializza i test dell'OCR.
///
/// Arbitraria e condivisa fra i due binari di test: deve solo non collidere con
/// altri lock nello stesso database.
const OCR_QUEUE_LOCK: i64 = 918_273_645;

/// Prende in esclusiva la coda dei job AI per la durata del test.
///
/// **Serve perché la coda è globale, e lo è di proposito.** `claim_next` non
/// filtra per organizzazione — il worker serve tutte, e deve — mentre ogni test
/// ha il **proprio** doppio dello storage oggetti. Due test dell'OCR in parallelo
/// sullo stesso database producono quindi il difetto peggiore possibile: il worker
/// dell'uno prende il job dell'altro, non trova l'oggetto nel proprio bucket, e lo
/// chiude `failed` con `document_unavailable`. Non è un difetto del codice — in
/// produzione c'è un solo storage — ma della suite, che simula una configurazione
/// che non può esistere.
///
/// L'invariante da rispettare è: **accodare e smaltire dentro la stessa sezione
/// critica**, così quando il lock si libera non restano job in coda per il worker
/// del test successivo.
///
/// `pg_advisory_xact_lock` e non quello di sessione: si rilascia con la
/// transazione, quindi anche se il test va in panico. Un lock di sessione su una
/// connessione che torna al pool resterebbe preso, e la suite si bloccherebbe.
pub struct OcrQueueGate(#[allow(dead_code)] sqlx::Transaction<'static, sqlx::Postgres>);

pub async fn lock_ocr_queue(pools: &DatabasePools) -> OcrQueueGate {
    let mut tx = pools
        .privileged()
        .begin()
        .await
        .expect("transazione per il lock");
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(OCR_QUEUE_LOCK)
        .execute(&mut *tx)
        .await
        .expect("lock sulla coda dei job");
    OcrQueueGate(tx)
}
