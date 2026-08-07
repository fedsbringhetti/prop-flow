// `ApiError` porta il contenuto di un Problem Details (titolo, dettaglio, errori
// per campo): è volutamente ricco. Incapsularlo in un Box a ogni handler
// aggiungerebbe un'allocazione sul percorso di errore per risparmiare stack su un
// ramo che comunque termina la richiesta.
#![allow(clippy::result_large_err)]
// `unwrap`/`expect` restano vietati nel codice di produzione. Nei test sono
// invece lo strumento corretto: un panic segnala una precondizione di test non
// soddisfatta, e mascherarla con un `if let` renderebbe il test verde a vuoto.
#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used, clippy::panic))]

//! Bordo HTTP: router, estrattori, modello di errore, documento OpenAPI.
//!
//! Gli handler non contengono logica di business. Deserializzano, chiamano uno use
//! case, serializzano. È esposto come libreria oltre che come binario perché i
//! test dei bordi montano il router in-process, senza rete.

pub mod config;
pub mod error;
pub mod middleware;
pub mod openapi;
pub mod routes;
pub mod state;

use std::time::Duration;

use axum::Router;
use axum::http::{HeaderName, StatusCode};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi as _;
use utoipa_swagger_ui::SwaggerUi;

use crate::middleware::REQUEST_ID_HEADER;
use crate::state::AppState;

/// Compone il router completo.
///
/// L'ordine dei layer conta: il request id viene generato prima del tracing, così
/// ogni riga di log della richiesta lo riporta, ed è propagato in risposta perché
/// un utente che segnala un errore possa citarlo.
pub fn router(state: AppState, body_limit_bytes: usize, request_timeout: Duration) -> Router {
    let request_id_header = HeaderName::from_static(REQUEST_ID_HEADER);

    Router::new()
        .merge(routes::health::router())
        .merge(routes::auth::router())
        .merge(routes::properties::router())
        .merge(routes::rooms::router())
        .merge(routes::leases::router())
        .merge(routes::invitations::router())
        .merge(SwaggerUi::new("/docs").url("/openapi.json", openapi::ApiDoc::openapi()))
        .fallback(not_found)
        // 503 e non 408: il timeout scade per lentezza del server, non del client.
        .layer(TimeoutLayer::with_status_code(
            StatusCode::SERVICE_UNAVAILABLE,
            request_timeout,
        ))
        .layer(RequestBodyLimitLayer::new(body_limit_bytes))
        .layer(TraceLayer::new_for_http())
        .layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(SetRequestIdLayer::new(request_id_header, MakeRequestUuid))
        .with_state(state)
}

/// Anche il 404 esce come Problem Details: un client non deve incontrare due
/// formati di errore diversi a seconda di quale rotta ha sbagliato.
async fn not_found() -> crate::error::ApiError {
    crate::error::ApiError::from(propflow_application::AppError::NotFound { entity: "endpoint" })
}

/// Riesportato per i test dei bordi.
pub const NOT_FOUND: StatusCode = StatusCode::NOT_FOUND;
