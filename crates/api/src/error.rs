use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use propflow_application::AppError;
use propflow_domain::DomainError;

/// Errore HTTP serializzato secondo RFC 9457 (Problem Details).
///
/// Un formato unico per ogni errore dell'API: il client ha un solo contratto da
/// gestire invece di indovinare la forma della risposta endpoint per endpoint.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ProblemDetails {
    /// URI che identifica il tipo di problema.
    #[schema(example = "https://propflow.dev/problems/validation")]
    pub r#type: String,
    /// Riassunto leggibile, stabile per tipo di problema.
    pub title: String,
    pub status: u16,
    /// Dettaglio specifico di questa occorrenza.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Identificatore della richiesta, da citare in caso di segnalazione.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Errori per campo, presenti solo sulle validazioni.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<FieldError>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

/// Errore restituito dagli handler.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    problem_type: &'static str,
    title: &'static str,
    detail: Option<String>,
    errors: Vec<FieldError>,
    request_id: Option<String>,
    /// Dettaglio tecnico da registrare nei log e **non** da restituire: un
    /// messaggio di errore del database in una risposta HTTP è ricognizione
    /// gratuita per chi attacca.
    internal: Option<String>,
}

impl ApiError {
    #[must_use]
    pub fn with_request_id(mut self, request_id: Option<String>) -> Self {
        self.request_id = request_id;
        self
    }

    fn new(status: StatusCode, problem_type: &'static str, title: &'static str) -> Self {
        Self {
            status,
            problem_type,
            title,
            detail: None,
            errors: Vec::new(),
            request_id: None,
            internal: None,
        }
    }

    #[must_use]
    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Autenticazione richiesta",
        )
    }

    #[must_use]
    pub fn bad_request(detail: impl Into<String>) -> Self {
        let mut error = Self::new(
            StatusCode::BAD_REQUEST,
            "bad-request",
            "Richiesta non valida",
        );
        error.detail = Some(detail.into());
        error
    }
}

impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        match error {
            AppError::Validation(DomainError::Validation { field, message }) => {
                let mut api = Self::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "validation",
                    "Dati non validi",
                );
                api.errors.push(FieldError {
                    field: field.to_owned(),
                    message,
                });
                api
            }
            AppError::Validation(DomainError::Invariant(message)) => {
                let mut api = Self::new(
                    StatusCode::CONFLICT,
                    "invariant",
                    "Operazione non ammessa nello stato attuale",
                );
                api.detail = Some(message);
                api
            }
            AppError::Validation(DomainError::NotFound { entity }) => {
                let mut api = Self::new(StatusCode::NOT_FOUND, "not-found", "Risorsa non trovata");
                api.detail = Some(format!("{entity} non trovato"));
                api
            }
            AppError::Validation(DomainError::Forbidden(reason)) | AppError::Forbidden(reason) => {
                let mut api = Self::new(
                    StatusCode::FORBIDDEN,
                    "forbidden",
                    "Operazione non consentita",
                );
                api.detail = Some(reason.to_owned());
                api
            }
            AppError::Unauthenticated => Self::unauthorized(),
            AppError::NotFound { entity } => {
                let mut api = Self::new(StatusCode::NOT_FOUND, "not-found", "Risorsa non trovata");
                api.detail = Some(format!("{entity} non trovato"));
                api
            }
            AppError::Conflict { resource } => {
                let mut api = Self::new(StatusCode::CONFLICT, "conflict", "Risorsa già esistente");
                api.detail = Some(format!("{resource} già esistente"));
                api
            }
            // Il vincolo di esclusione è l'autorità su questa invariante, ma il
            // suo nome non dice niente a un client: la risposta nomina l'oggetto
            // occupato, non `leases_no_overlap_room`.
            AppError::OverlappingLease(target) => {
                let mut api =
                    Self::new(StatusCode::CONFLICT, "conflict", "Periodo non disponibile");
                api.detail = Some(format!(
                    "esiste già una locazione attiva per {target} nel periodo indicato"
                ));
                api
            }
            AppError::Integrity { details } => {
                let mut api = Self::new(StatusCode::CONFLICT, "conflict", "Vincolo violato");
                api.internal = Some(details);
                api
            }
            AppError::Internal { details } => {
                let mut api = Self::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "Errore interno",
                );
                api.internal = Some(details);
                api
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let Some(internal) = &self.internal {
            // Il dettaglio tecnico resta nei log, correlato dal request_id.
            tracing::error!(
                status = self.status.as_u16(),
                request_id = self.request_id.as_deref().unwrap_or("-"),
                problem = self.problem_type,
                details = %internal,
                "richiesta terminata con errore"
            );
        }

        let body = ProblemDetails {
            r#type: format!("https://propflow.dev/problems/{}", self.problem_type),
            title: self.title.to_owned(),
            status: self.status.as_u16(),
            detail: self.detail,
            request_id: self.request_id,
            errors: self.errors,
        };

        let mut response = (self.status, Json(body)).into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        if self.status == StatusCode::UNAUTHORIZED {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        response
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
