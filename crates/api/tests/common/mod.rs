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
        };

        Some(Self {
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

    pub async fn post(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let request = Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("richiesta costruita");
        let (status, json, _) = self.request(request).await;
        (status, json)
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

    let (status, body) = app
        .post(
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
