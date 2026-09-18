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
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use utoipa::OpenApi as _;
use utoipa_swagger_ui::SwaggerUi;

use crate::middleware::REQUEST_ID_HEADER;
use crate::state::AppState;

/// Compone il router completo.
///
/// L'ordine dei layer conta: il request id viene generato prima del tracing, così
/// ogni riga di log della richiesta lo riporta, ed è propagato in risposta perché
/// un utente che segnala un errore possa citarlo.
///
/// Lo span della richiesta è costruito a mano e non con `DefaultMakeSpan`: quello
/// predefinito registra metodo, URI e versione, e **non** il request id — che è la
/// sola chiave con cui si ritrova una richiesta segnalata da un utente. La risposta
/// si logga a `INFO` per lo stesso motivo: a `DEBUG`, che è il default di
/// `TraceLayer`, con il filtro che il progetto spedisce non compare alcuna riga per
/// nessuna richiesta.
///
/// # Perché due gruppi di rotte
///
/// `RequestBodyLimitLayer` non ammette deroghe per rotta: montato sull'intero
/// router rifiuta **per primo**, e un limite più largo dichiarato sopra la rotta di
/// caricamento non verrebbe mai raggiunto — il sintomo è un `413` che nessun limite
/// per rotta sembra spiegare. Alzare quello generale funzionerebbe ed è la via da
/// non prendere: indebolisce ogni endpoint per servirne uno.
///
/// Quindi il limite generale si monta sul gruppo che **non** comprende le rotte di
/// caricamento, e quelle portano il proprio (`routes::documents::upload_router`).
pub fn router(
    state: AppState,
    body_limit_bytes: usize,
    upload_body_limit_bytes: usize,
    request_timeout: Duration,
) -> Router {
    let request_id_header = HeaderName::from_static(REQUEST_ID_HEADER);

    let limited = Router::new()
        .merge(routes::health::router())
        .merge(routes::auth::router())
        .merge(routes::organizations::router())
        .merge(routes::me::router())
        .merge(routes::dashboard::router())
        .merge(routes::properties::router())
        .merge(routes::rooms::router())
        .merge(routes::leases::router())
        .merge(routes::invitations::router())
        .merge(routes::documents::router())
        .merge(routes::tickets::router())
        .merge(routes::billing::router())
        .merge(routes::payments::router())
        .merge(routes::expenses::router())
        .merge(SwaggerUi::new("/docs").url("/openapi.json", openapi::ApiDoc::openapi()))
        .layer(RequestBodyLimitLayer::new(body_limit_bytes));

    limited
        .merge(routes::documents::upload_router(upload_body_limit_bytes))
        .fallback(not_found)
        // 503 e non 408: il timeout scade per lentezza del server, non del client.
        .layer(TimeoutLayer::with_status_code(
            StatusCode::SERVICE_UNAVAILABLE,
            request_timeout,
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(request_span)
                .on_response(DefaultOnResponse::new().level(tracing::Level::INFO)),
        )
        .layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(SetRequestIdLayer::new(request_id_header, MakeRequestUuid))
        .with_state(state)
}

/// Span di una richiesta.
///
/// Funzione con un nome invece di una chiusura in mezzo ai layer perché così è
/// verificabile: il test qui sotto fallisce se il campo `request_id` sparisce, che
/// è ciò che accade tornando a `DefaultMakeSpan`.
fn request_span(request: &axum::http::Request<axum::body::Body>) -> tracing::Span {
    tracing::info_span!(
        "request",
        method = %request.method(),
        uri = %request.uri(),
        request_id = request
            .headers()
            .get(REQUEST_ID_HEADER)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("-"),
    )
}

/// Anche il 404 esce come Problem Details: un client non deve incontrare due
/// formati di errore diversi a seconda di quale rotta ha sbagliato.
async fn not_found() -> crate::error::ApiError {
    crate::error::ApiError::from(propflow_application::AppError::NotFound(
        propflow_domain::Entity::Endpoint,
    ))
}

/// Riesportato per i test dei bordi.
pub const NOT_FOUND: StatusCode = StatusCode::NOT_FOUND;

#[cfg(test)]
mod tests {
    use super::*;

    /// Lo span della richiesta deve portare il request id.
    ///
    /// Senza, l'id resta nella risposta ma non nei log, e chi riceve una
    /// segnalazione con un id in mano non ha nulla da cercare — che è la ragione
    /// per cui l'id esiste.
    #[test]
    fn the_request_span_carries_the_request_id() {
        let request = axum::http::Request::builder()
            .uri("/api/v1/properties")
            .header(REQUEST_ID_HEADER, "id-di-prova")
            .body(axum::body::Body::empty())
            .expect("richiesta di prova");

        let fields = request_span(&request).metadata().map(|m| {
            m.fields()
                .iter()
                .map(|f| f.name().to_owned())
                .collect::<Vec<_>>()
        });

        assert_eq!(
            fields,
            Some(vec![
                "method".to_owned(),
                "uri".to_owned(),
                "request_id".to_owned()
            ]),
            "lo span della richiesta ha perso un campo"
        );
    }
}
