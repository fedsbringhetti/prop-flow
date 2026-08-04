use std::net::{IpAddr, SocketAddr};

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{HeaderMap, header};
use propflow_application::ports::CryptoError;
use propflow_domain::RequestContext;
use propflow_domain::identity::PlatformRole;
use propflow_domain::shared::OrgId;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

pub const REQUEST_ID_HEADER: &str = "x-request-id";
const ORG_HEADER: &str = "x-org-id";

/// Contesto di una richiesta autenticata.
///
/// L'estrattore fa tre cose: verifica la firma dell'access token, decide
/// l'organizzazione attiva e — solo se il client ne chiede una diversa da quella
/// nel token — rilegge la membership dal database.
///
/// Il ruolo presente nel token serve a rispondere `403` senza interrogare il
/// database. Non è l'autorità: la Row Level Security rilegge sempre la membership,
/// quindi un token con un ruolo gonfiato non ottiene righe in più.
pub struct Authenticated(pub RequestContext);

impl FromRequestParts<AppState> for Authenticated {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let request_id = extract_request_id(&parts.headers);
        let token = bearer_token(&parts.headers)
            .ok_or_else(|| ApiError::unauthorized().with_request_id(request_id.clone()))?;

        let claims = state.tokens.verify(token).map_err(|error| match error {
            CryptoError::ExpiredToken | CryptoError::InvalidToken => {
                ApiError::unauthorized().with_request_id(request_id.clone())
            }
            CryptoError::Backend { details } => {
                ApiError::from(propflow_application::AppError::Internal { details })
                    .with_request_id(request_id.clone())
            }
        })?;

        let requested_org = requested_org_id(&parts.headers)
            .map_err(|error| error.with_request_id(request_id.clone()))?;

        let mut context = RequestContext::authenticated(claims.user_id)
            .with_platform_admin(claims.platform_role.is_admin())
            .with_transport(
                request_id.clone(),
                client_ip(parts),
                user_agent(&parts.headers),
            );

        context = match (requested_org, claims.org_id, claims.org_role) {
            // Nessuna organizzazione richiesta: vale quella del token.
            (None, Some(org_id), Some(role)) => context.with_org(org_id, role),
            (None, _, _) => context,
            // Organizzazione richiesta uguale a quella del token: ruolo già noto.
            (Some(requested), Some(token_org), Some(role)) if requested == token_org => {
                context.with_org(requested, role)
            }
            // Organizzazione diversa: il ruolo nel token non vale qui, va riletto.
            (Some(requested), _, _) => {
                let role = resolve_role(state, &context, requested)
                    .await
                    .map_err(|error| error.with_request_id(request_id.clone()))?;
                context.with_org(requested, role)
            }
        };

        Ok(Self(context))
    }
}

/// Contesto di una richiesta non autenticata: registrazione, login, refresh.
/// Trasporta solo i dati di correlazione, nessuna identità.
pub struct Anonymous(pub RequestContext);

impl<S> FromRequestParts<S> for Anonymous
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let context = RequestContext::anonymous().with_transport(
            extract_request_id(&parts.headers),
            client_ip(parts),
            user_agent(&parts.headers),
        );
        Ok(Self(context))
    }
}

async fn resolve_role(
    state: &AppState,
    context: &RequestContext,
    org_id: OrgId,
) -> Result<propflow_domain::identity::OrgRole, ApiError> {
    let user_id = context.user_id().ok_or_else(ApiError::unauthorized)?;

    // Il contesto passato qui ha solo l'utente: basta, perché la policy su
    // `org_members` mostra a ciascuno le proprie appartenenze
    // (`user_id = app.current_user_id()`) indipendentemente dall'organizzazione.
    let mut uow = state
        .unit_of_work
        .begin(context)
        .await
        .map_err(|error| ApiError::from(propflow_application::AppError::from(error)))?;
    let membership = uow
        .members()
        .find(org_id, user_id)
        .await
        .map_err(|error| ApiError::from(propflow_application::AppError::from(error)))?;

    match membership {
        Some(member) if member.is_active() => Ok(member.role()),
        // Non si distingue "organizzazione inesistente" da "non ne fai parte":
        // la differenza rivelerebbe l'esistenza di organizzazioni altrui.
        _ => Err(ApiError::from(propflow_application::AppError::Forbidden(
            "non appartieni a questa organizzazione",
        ))),
    }
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

fn requested_org_id(headers: &HeaderMap) -> Result<Option<OrgId>, ApiError> {
    let Some(raw) = headers.get(ORG_HEADER) else {
        return Ok(None);
    };
    let value = raw
        .to_str()
        .map_err(|_| ApiError::bad_request("header X-Org-Id non è testo valido"))?;
    let uuid = Uuid::parse_str(value.trim())
        .map_err(|_| ApiError::bad_request("header X-Org-Id non è un UUID"))?;
    Ok(Some(OrgId::from_uuid(uuid)))
}

fn extract_request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.chars().take(128).collect())
}

fn user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

/// IP del chiamante. Dietro un ingress il socket è quello del proxy, quindi si
/// legge `X-Forwarded-For` — solo il primo elemento, gli altri sono aggiunti a
/// valle e non affidabili.
fn client_ip(parts: &Parts) -> Option<IpAddr> {
    if let Some(forwarded) = parts.headers.get("x-forwarded-for")
        && let Some(first) = forwarded
            .to_str()
            .ok()
            .and_then(|value| value.split(',').next())
        && let Ok(ip) = first.trim().parse::<IpAddr>()
    {
        return Some(ip);
    }
    parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(address)| address.ip())
}

/// Riesportato per i test dei bordi.
pub const fn platform_admin_role() -> PlatformRole {
    PlatformRole::Admin
}
