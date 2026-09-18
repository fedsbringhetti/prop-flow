use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};

use crate::state::AppState;

/// Le sonde, in due forme che rispondono allo stesso modo.
///
/// Alla **radice** perché l'`healthcheck` del container parla all'API direttamente,
/// senza proxy. **Sotto `/api/v1`** perché dal dominio pubblico il reverse proxy
/// inoltra all'API solo quel prefisso (`ops/web/Caddyfile`): `…/ready` alla radice
/// non arriva mai qui — risponde il file server, con `index.html` e `200`, una
/// sonda che «riesce» senza aver provato niente — e il runbook del primo deploy si
/// dichiara riuscito su `…/api/v1/ready`. Un `handle /ready` nel proxy avrebbe
/// spostato là la conoscenza delle sonde, che oggi conosce solo il prefisso.
///
/// Nessun estrattore `Authenticated` su nessuna delle quattro: una sonda che
/// chieda un token non è una sonda.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(liveness))
        .route("/ready", get(readiness))
        .route("/api/v1/health", get(liveness))
        .route("/api/v1/ready", get(readiness))
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
    if let Err(error) = state.pools.ping().await {
        tracing::warn!(error = %error, "controllo di prontezza fallito");
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse {
                status: "database non raggiungibile",
            }),
        );
    }
    // **La posta sta qui accanto al database**, e non è completezza. L'invio è
    // best-effort dopo il commit: un relay caduto non produce nessun errore visibile
    // a nessuno, e il recupero della password risponde `202` senza mandare niente —
    // su chi non ha nessun altro modo di rientrare nel proprio account. Dirlo al
    // chiamante costruirebbe un oracolo sugli account (ADR-017), quindi lo si dice
    // **qui**, prima che qualcuno lo incontri.
    if let Err(error) = state.notifier.probe().await {
        tracing::warn!(error = %error, "controllo di prontezza fallito");
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse {
                status: "posta non raggiungibile",
            }),
        );
    }
    (StatusCode::OK, Json(HealthResponse { status: "ready" }))
}
