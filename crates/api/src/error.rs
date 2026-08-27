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
    /// URI che identifica la **classe** del problema.
    #[schema(example = "https://propflow.dev/problems/validation")]
    pub r#type: String,
    /// Riassunto leggibile, stabile per classe di problema.
    pub title: String,
    pub status: u16,
    /// Codice stabile della **condizione**, sempre presente (ADR-022).
    ///
    /// È il campo su cui un client instrada e localizza: `title` e `detail` sono
    /// per un umano che legge i log e possono essere riscritti, `code` no.
    #[schema(example = "lease.period_overlap")]
    pub code: String,
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
    code: &'static str,
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

    fn new(
        status: StatusCode,
        problem_type: &'static str,
        title: &'static str,
        code: &'static str,
    ) -> Self {
        Self {
            status,
            problem_type,
            title,
            code,
            detail: None,
            errors: Vec::new(),
            request_id: None,
            internal: None,
        }
    }

    fn unauthorized(code: &'static str) -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Authentication required",
            code,
        )
    }

    /// Nessun header `Authorization`.
    #[must_use]
    pub fn token_missing() -> Self {
        Self::unauthorized("auth.token_missing")
    }

    /// Firma o forma del token rifiutate.
    #[must_use]
    pub fn token_invalid() -> Self {
        Self::unauthorized("auth.token_invalid")
    }

    /// Token nostro, firma valida, `exp` superato.
    ///
    /// Distinto da [`Self::token_invalid`] di proposito: dice al client di
    /// rinnovare invece di rimandare l'utente al login. Non è un oracolo — per
    /// arrivare qui bisogna già presentare un token che abbiamo firmato noi,
    /// quindi non si rivela nulla che il chiamante non sapesse. Il *refresh*
    /// invece resta indistinto (ADR-022): lì "scaduto" contro "riusato" direbbe a
    /// chi ha rubato un token che è ancora vivo.
    #[must_use]
    pub fn token_expired() -> Self {
        Self::unauthorized("auth.token_expired")
    }

    /// Corpo oltre il limite consentito dalla rotta.
    ///
    /// Nasce qui e non dal service layer perché è una condizione del **trasporto**:
    /// il dominio non sa quanti byte l'API accetti, e non deve saperlo.
    #[must_use]
    pub fn payload_too_large() -> Self {
        let mut error = Self::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload-too-large",
            "Payload too large",
            "request.payload_too_large",
        );
        error.detail = Some("the request body exceeds the limit for this route".to_owned());
        error
    }

    #[must_use]
    pub fn bad_request(code: &'static str, detail: impl Into<String>) -> Self {
        let mut error = Self::new(StatusCode::BAD_REQUEST, "bad-request", "Bad request", code);
        error.detail = Some(detail.into());
        error
    }
}

impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        // Il codice si legge **una volta sola** dal service layer. Nessun ramo qui
        // sotto può inventarne uno o dimenticarsene: il bordo trascrive (ADR-022).
        let code = error.code();

        match error {
            AppError::Validation(DomainError::Validation { field, message }) => {
                let mut api = Self::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "validation",
                    "Validation failed",
                    code,
                );
                api.errors.push(FieldError {
                    field: field.to_owned(),
                    message,
                });
                api
            }
            AppError::Validation(DomainError::Invariant(invariant)) => {
                let mut api = Self::new(
                    StatusCode::CONFLICT,
                    "invariant",
                    "Operation not allowed in the current state",
                    code,
                );
                api.detail = Some(invariant.to_string());
                api
            }
            AppError::Validation(DomainError::NotFound(entity)) | AppError::NotFound(entity) => {
                let mut api = Self::new(
                    StatusCode::NOT_FOUND,
                    "not-found",
                    "Resource not found",
                    code,
                );
                api.detail = Some(entity.to_string());
                api
            }
            AppError::Validation(DomainError::Forbidden(denial)) | AppError::Forbidden(denial) => {
                let mut api = Self::new(
                    StatusCode::FORBIDDEN,
                    "forbidden",
                    "Operation not allowed",
                    code,
                );
                api.detail = Some(denial.to_string());
                api
            }
            AppError::Unauthenticated => Self::unauthorized(code),
            // Non si dice al chiamante che è stata una policy a fermarlo: la
            // diagnosi è nostra e sta nei log, correlata dal request_id.
            AppError::DeniedByPolicy => {
                let mut api = Self::new(
                    StatusCode::FORBIDDEN,
                    "forbidden",
                    "Operation not allowed",
                    code,
                );
                api.internal = Some("denied by a row level security policy".to_owned());
                api
            }
            AppError::Conflict(conflict) => {
                let mut api = Self::new(StatusCode::CONFLICT, "conflict", "Conflict", code);
                api.detail = Some(conflict.to_string());
                api
            }
            // La forma resta quella di una validazione — stesso `errors[]` di
            // sempre — ma il codice dice *quale* rifiuto è, che con
            // `validation.failed` si sarebbe perso.
            AppError::UnsupportedQuery(query) => {
                let mut api = Self::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "validation",
                    "Validation failed",
                    code,
                );
                api.errors.push(FieldError {
                    field: query.field().to_owned(),
                    message: query.to_string(),
                });
                api
            }
            // Il vincolo di esclusione è l'autorità su questa invariante, ma il
            // suo nome non dice niente a un client: la risposta nomina l'oggetto
            // occupato, non `leases_no_overlap_room`.
            AppError::OverlappingLease(target) => {
                let mut api = Self::new(
                    StatusCode::CONFLICT,
                    "conflict",
                    "Period not available",
                    code,
                );
                api.detail = Some(format!(
                    "an active lease already covers {target} for the given period"
                ));
                api
            }
            AppError::Integrity { details } => {
                let mut api = Self::new(
                    StatusCode::CONFLICT,
                    "conflict",
                    "Constraint violated",
                    code,
                );
                api.internal = Some(details);
                api
            }
            AppError::Internal { details } => {
                let mut api = Self::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "Internal error",
                    code,
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
                code = self.code,
                details = %internal,
                "richiesta terminata con errore"
            );
        }

        let body = ProblemDetails {
            r#type: format!("https://propflow.dev/problems/{}", self.problem_type),
            title: self.title.to_owned(),
            status: self.status.as_u16(),
            code: self.code.to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use propflow_application::Conflict;
    use propflow_domain::{Denial, Entity, Invariant};

    /// Il bordo trascrive il codice del service layer. Se un ramo lo riscrivesse
    /// a mano, questo test lo prende.
    #[test]
    fn the_edge_transcribes_the_service_layer_code() {
        let cases: Vec<AppError> = vec![
            AppError::Unauthenticated,
            AppError::Forbidden(Denial::StaffRoleRequired),
            AppError::DeniedByPolicy,
            AppError::NotFound(Entity::Property),
            AppError::Conflict(Conflict::EmailTaken),
            AppError::Validation(DomainError::Invariant(Invariant::PropertyArchived)),
            AppError::Validation(DomainError::validation("email", "invalid domain")),
            AppError::Integrity {
                details: "vincolo `x`".to_owned(),
            },
            AppError::internal("boom"),
        ];

        for error in cases {
            let expected = error.code();
            let api = ApiError::from(error);
            assert_eq!(api.code, expected);
        }
    }

    /// Un dettaglio tecnico non deve mai finire nel corpo: `Integrity` e
    /// `DeniedByPolicy` parlano ai log, non al chiamante.
    #[test]
    fn technical_details_never_reach_the_body() {
        for error in [
            AppError::Integrity {
                details: "vincolo `users_email`".to_owned(),
            },
            AppError::internal("connection refused"),
            AppError::DeniedByPolicy,
        ] {
            let api = ApiError::from(error);
            assert!(api.detail.is_none(), "code {}", api.code);
            assert!(api.internal.is_some(), "code {}", api.code);
        }
    }

    #[test]
    fn an_unsupported_filter_keeps_the_validation_shape_with_its_own_code() {
        let api = ApiError::from(AppError::UnsupportedQuery(
            propflow_application::UnsupportedQuery::LeaseDerivedStatusFilter,
        ));
        assert_eq!(api.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(api.code, "lease.derived_status_filter");
        assert_eq!(api.errors.first().map(|e| e.field.as_str()), Some("status"));
    }

    /// Il `413` è una condizione del trasporto e non del dominio: deve avere un
    /// codice proprio, altrimenti un client non distingue "file troppo grande" da
    /// "file non valido".
    #[test]
    fn an_oversized_body_has_its_own_code() {
        let error = ApiError::payload_too_large();
        assert_eq!(error.status, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(error.code, "request.payload_too_large");
        assert!(error.internal.is_none());
    }

    #[test]
    fn an_expired_token_is_distinguishable_from_an_invalid_one() {
        assert_eq!(ApiError::token_expired().code, "auth.token_expired");
        assert_eq!(ApiError::token_invalid().code, "auth.token_invalid");
        assert_eq!(ApiError::token_missing().code, "auth.token_missing");
        // Le credenziali sbagliate restano indistinte da un account inesistente.
        assert_eq!(
            ApiError::from(AppError::Unauthenticated).code,
            "auth.invalid_credentials"
        );
    }
}
