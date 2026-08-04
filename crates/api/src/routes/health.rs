use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(liveness))
        .route("/ready", get(readiness))
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
}

/// Liveness: risponde sempre, senza toccare nessuna dipendenza.
///
/// Se controllasse il database, un'indisponibilità di PostgreSQL farebbe
/// riavviare in loop tutti i pod dell'API — che è esattamente il contrario di
/// quello che serve mentre il database torna su.
#[utoipa::path(
    get, path = "/health", tag = "sistema",
    responses((status = 200, description = "Il processo è vivo", body = HealthResponse))
)]
async fn liveness() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

/// Readiness: verifica che il database sia raggiungibile.
///
/// Qui il controllo ci va: un pod che non riesce a parlare col database non deve
/// ricevere traffico, ma non deve nemmeno essere riavviato.
#[utoipa::path(
    get, path = "/ready", tag = "sistema",
    responses(
        (status = 200, description = "Pronto a servire traffico", body = HealthResponse),
        (status = 503, description = "Dipendenze non raggiungibili", body = HealthResponse),
    )
)]
async fn readiness(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    match state.pools.ping().await {
        Ok(()) => (StatusCode::OK, Json(HealthResponse { status: "ready" })),
        Err(error) => {
            tracing::warn!(error = %error, "controllo di prontezza fallito");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(HealthResponse {
                    status: "database non raggiungibile",
                }),
            )
        }
    }
}
