//! Supporto ai test dei bordi HTTP.
//!
//! Il router viene montato in-process e interrogato con `tower::ServiceExt::oneshot`:
//! nessuna porta aperta, nessun client HTTP, nessuna attesa di avvio. Il database
//! però è reale, perché la Row Level Security fa parte del comportamento che questi
//! test verificano.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use chrono::Duration;
use propflow_api::state::{AppState, Dependencies};
use propflow_infrastructure::db::PoolSettings;
use propflow_infrastructure::security::{
    Argon2PasswordHasher, EdDsaTokenIssuer, JwtSettings, OpaqueRefreshTokenFactory,
    OpaqueTokenGenerator,
};
use propflow_infrastructure::{DatabasePools, PgUnitOfWorkFactory, SystemClock};
use tower::ServiceExt as _;

pub struct TestApp {
    pub router: Router,
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
            pools: pools.clone(),
        };

        Some(Self {
            router: propflow_api::router(
                AppState::new(dependencies),
                1024 * 1024,
                std::time::Duration::from_secs(10),
            ),
        })
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
