use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use propflow_domain::payment::IdempotencyKey;

use crate::error::ApiError;
use crate::middleware::REQUEST_ID_HEADER;

pub const IDEMPOTENCY_HEADER: &str = "idempotency-key";
/// Un header assente è una condizione del **trasporto**, non del dominio: il
/// rimedio è rimandare la richiesta con l'header, e il codice lo dice.
const MISSING_HEADER: &str = "request.missing_header";
const INVALID_HEADER: &str = "request.invalid_header";

/// L'header `Idempotency-Key`, **obbligatorio** sulle rotte che creano un
/// pagamento.
///
/// La chiave la fornisce il client e non il server, ed è l'unica decisione di
/// questa fetta in cui sbagliare costa denaro: se a generarla fosse il server, ogni
/// ritentativo produrrebbe una chiave nuova, quindi un secondo pagamento, quindi un
/// saldo che scende **due volte**. Vedi ADR-033.
///
/// **È un estrattore e non un campo del corpo** perché è una proprietà della
/// *richiesta*, non del pagamento: la stessa chiave deve poter accompagnare il
/// medesimo invio ripetuto, e un campo del corpo inviterebbe a rigenerarla insieme
/// al resto. È anche la convenzione con cui i client la trattano già.
///
/// Assente dà `400` e non `422`: non c'è nulla di malformato nel corpo, manca un
/// pezzo del trasporto.
pub struct IdempotencyKeyHeader(pub IdempotencyKey);

impl<S> FromRequestParts<S> for IdempotencyKeyHeader
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let request_id = parts
            .headers
            .get(REQUEST_ID_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.chars().take(128).collect::<String>());

        let raw = parts.headers.get(IDEMPOTENCY_HEADER).ok_or_else(|| {
            ApiError::bad_request(
                MISSING_HEADER,
                "The Idempotency-Key header is required on this route",
            )
            .with_request_id(request_id.clone())
        })?;

        let value = raw.to_str().map_err(|_| {
            ApiError::bad_request(
                INVALID_HEADER,
                "The Idempotency-Key header is not valid text",
            )
            .with_request_id(request_id.clone())
        })?;

        let key = IdempotencyKey::parse(value).map_err(|error| {
            ApiError::from(propflow_application::AppError::Validation(error))
                .with_request_id(request_id)
        })?;

        Ok(Self(key))
    }
}
