use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::routing::post;
use axum::{Json, Router};
use propflow_application::use_cases::auth::{
    AuthenticatedSession, LoginCommand, LogoutCommand, RefreshSessionCommand, RegisterOwnerCommand,
};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::{Anonymous, Authenticated, clear_refresh_cookie, set_refresh_cookie};
use crate::state::AppState;

/// Le quattro rotte dell'autenticazione, e le sole che il cookie di sessione
/// raggiunge: `Path=/api/v1/auth` lo limita a questo prefisso.
///
/// Registrazione e login restano `Anonymous` perché non c'è ancora nessuno da
/// autenticare; **anche il refresh lo è**, e non è una svista — chi lo chiama
/// presenta un token, non un'identità, e pretendere un `Authorization` valido
/// renderebbe impossibile rinnovare proprio quando serve, cioè con l'access token
/// già scaduto.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/refresh", post(refresh))
        .route("/api/v1/auth/logout", post(logout))
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct RegisterRequest {
    #[schema(example = "mario.rossi@example.it")]
    pub email: String,
    #[schema(example = "Mario Rossi")]
    pub full_name: String,
    /// Almeno 12 caratteri.
    #[schema(example = "cavallo batteria graffetta")]
    pub password: String,
    /// Nome dell'organizzazione personale. Se omesso si usa il nome dell'utente.
    #[serde(default)]
    pub organization_name: Option<String>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct RegisterResponse {
    pub user_id: Uuid,
    pub organization_id: Uuid,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Risposta di autenticazione, nella forma attesa da un client OAuth2 **meno il
/// refresh token**.
///
/// Il refresh token esce solo come cookie `httpOnly`. Metterlo anche qui
/// annullerebbe il cookie: un XSS lo leggerebbe dalla risposta del login senza
/// dover toccare `document.cookie`, e trenta giorni di sessione sarebbero
/// leggibili da uno script. Vedi `crate::middleware::session_cookie`.
///
/// **È una rottura del contratto**, non un'aggiunta: un client che leggeva
/// `refresh_token` dal corpo smette di trovarlo, e chi usa `curl` passa a un
/// cookie jar (`-c`/`-b`).
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct SessionResponse {
    pub access_token: String,
    pub token_type: &'static str,
    /// Secondi residui dell'access token.
    pub expires_in: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<&'static str>,
    pub user_id: Uuid,
}

impl SessionResponse {
    fn from_session(session: &AuthenticatedSession, now: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            expires_in: session.expires_in_seconds(now),
            token_type: "Bearer",
            organization_id: session.org_id.map(|id| id.as_uuid()),
            role: session
                .org_role
                .map(propflow_domain::identity::OrgRole::as_str),
            user_id: session.user_id.as_uuid(),
            access_token: session.access_token.clone(),
        }
    }
}

/// Corpo e cookie di una sessione appena aperta o rinnovata.
///
/// Una funzione sola per login e refresh: le due rotte devono posare **lo stesso**
/// cookie con gli stessi attributi, e scriverlo due volte è il modo in cui uno dei
/// due finisce per divergere.
fn session_reply(
    state: &AppState,
    session: &AuthenticatedSession,
) -> ApiResult<(HeaderMap, Json<SessionResponse>)> {
    let now = state.clock.now();
    // La vita del cookie è quella **residua** del token consegnato, letta dalla
    // sessione: derivarla di nuovo dal TTL configurato la farebbe divergere dalla
    // scadenza che il database ha memorizzato.
    let max_age = (session.refresh_token_expires_at - now).num_seconds();
    let cookie = set_refresh_cookie(&session.refresh_token, max_age, state.secure_cookies)?;

    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, cookie);
    Ok((headers, Json(SessionResponse::from_session(session, now))))
}

#[utoipa::path(
    post, path = "/api/v1/auth/register", tag = "autenticazione",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Proprietario e organizzazione personale creati", body = RegisterResponse),
        (status = 409, description = "Email già registrata", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi", body = crate::error::ProblemDetails),
    )
)]
async fn register(
    State(state): State<AppState>,
    Anonymous(context): Anonymous,
    Json(request): Json<RegisterRequest>,
) -> ApiResult<(StatusCode, Json<RegisterResponse>)> {
    let result = state
        .register_owner
        .execute(
            &context,
            RegisterOwnerCommand {
                email: request.email,
                full_name: request.full_name,
                password: request.password,
                organization_name: request.organization_name,
            },
        )
        .await
        .map_err(|error| {
            ApiError::from(error).with_request_id(context.request_id().map(str::to_owned))
        })?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            user_id: result.user_id.as_uuid(),
            organization_id: result.org_id.as_uuid(),
        }),
    ))
}

#[utoipa::path(
    post, path = "/api/v1/auth/login", tag = "autenticazione",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Sessione aperta. Il refresh token esce nel cookie `propflow_refresh`, non nel corpo", body = SessionResponse),
        (status = 401, description = "Credenziali non valide", body = crate::error::ProblemDetails),
    )
)]
async fn login(
    State(state): State<AppState>,
    Anonymous(context): Anonymous,
    Json(request): Json<LoginRequest>,
) -> ApiResult<(HeaderMap, Json<SessionResponse>)> {
    let session = state
        .login
        .execute(
            &context,
            LoginCommand {
                email: request.email,
                password: request.password,
            },
        )
        .await
        .map_err(|error| {
            ApiError::from(error).with_request_id(context.request_id().map(str::to_owned))
        })?;
    session_reply(&state, &session)
}

#[utoipa::path(
    post, path = "/api/v1/auth/refresh", tag = "autenticazione",
    responses(
        (status = 200, description = "Sessione rinnovata e token ruotato. Il token nuovo esce nel cookie, il vecchio è consumato", body = SessionResponse),
        (status = 401, description = "Cookie assente, oppure refresh token non valido, scaduto o già usato", body = crate::error::ProblemDetails),
    )
)]
async fn refresh(
    State(state): State<AppState>,
    Anonymous(context): Anonymous,
    headers: HeaderMap,
) -> ApiResult<(HeaderMap, Json<SessionResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    // Cookie assente e cookie rifiutato danno la **stessa** risposta. Non è
    // pigrizia: distinguerle direbbe a chi ha rubato un token se è ancora vivo, ed
    // è la ragione per cui il refresh non separa "scaduto" da "riusato"
    // (ADR-022). Il rimedio è comunque uno solo, rifare il login.
    let presented = crate::middleware::refresh_cookie(&headers).ok_or_else(|| {
        ApiError::from(propflow_application::AppError::Unauthenticated)
            .with_request_id(request_id.clone())
    })?;

    let session = state
        .refresh_session
        .execute(
            &context,
            RefreshSessionCommand {
                refresh_token: presented,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    session_reply(&state, &session)
}

#[utoipa::path(
    post, path = "/api/v1/auth/logout", tag = "autenticazione",
    security(("bearer" = [])),
    responses(
        (status = 204, description = "Sessione revocata e cookie cancellato. Idempotente"),
        (status = 401, description = "Autenticazione richiesta", body = crate::error::ProblemDetails),
    )
)]
async fn logout(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    headers: HeaderMap,
) -> ApiResult<(StatusCode, HeaderMap)> {
    // Senza cookie non c'è nulla da revocare, e la risposta resta `204`: il logout
    // è idempotente, e un `400` costringerebbe un client che ha già perso il
    // cookie a distinguere due esiti che per lui sono lo stesso. Il cookie si
    // cancella comunque — è il solo effetto che rimane da produrre.
    if let Some(presented) = crate::middleware::refresh_cookie(&headers) {
        state
            .logout
            .execute(
                &context,
                LogoutCommand {
                    refresh_token: presented,
                },
            )
            .await
            .map_err(|error| {
                ApiError::from(error).with_request_id(context.request_id().map(str::to_owned))
            })?;
    }

    let mut response = HeaderMap::new();
    response.insert(
        header::SET_COOKIE,
        clear_refresh_cookie(state.secure_cookies)?,
    );
    Ok((StatusCode::NO_CONTENT, response))
}
