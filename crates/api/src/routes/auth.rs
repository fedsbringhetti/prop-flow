use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use propflow_application::use_cases::auth::{
    AuthenticatedSession, LoginCommand, LogoutCommand, RefreshSessionCommand, RegisterOwnerCommand,
};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::{Anonymous, Authenticated};
use crate::state::AppState;

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

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Risposta di autenticazione, nella forma attesa da un client OAuth2.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct SessionResponse {
    pub access_token: String,
    pub token_type: &'static str,
    /// Secondi residui dell'access token.
    pub expires_in: i64,
    pub refresh_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<&'static str>,
    pub user_id: Uuid,
}

impl SessionResponse {
    fn from_session(session: AuthenticatedSession) -> Self {
        let now = chrono::Utc::now();
        Self {
            expires_in: session.expires_in_seconds(now),
            token_type: "Bearer",
            organization_id: session.org_id.map(|id| id.as_uuid()),
            role: session
                .org_role
                .map(propflow_domain::identity::OrgRole::as_str),
            user_id: session.user_id.as_uuid(),
            access_token: session.access_token,
            refresh_token: session.refresh_token,
        }
    }
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
        (status = 200, description = "Sessione aperta", body = SessionResponse),
        (status = 401, description = "Credenziali non valide", body = crate::error::ProblemDetails),
    )
)]
async fn login(
    State(state): State<AppState>,
    Anonymous(context): Anonymous,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<SessionResponse>> {
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
    Ok(Json(SessionResponse::from_session(session)))
}

#[utoipa::path(
    post, path = "/api/v1/auth/refresh", tag = "autenticazione",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Sessione rinnovata e token ruotato", body = SessionResponse),
        (status = 401, description = "Refresh token non valido, scaduto o già usato", body = crate::error::ProblemDetails),
    )
)]
async fn refresh(
    State(state): State<AppState>,
    Anonymous(context): Anonymous,
    Json(request): Json<RefreshRequest>,
) -> ApiResult<Json<SessionResponse>> {
    let session = state
        .refresh_session
        .execute(
            &context,
            RefreshSessionCommand {
                refresh_token: request.refresh_token,
            },
        )
        .await
        .map_err(|error| {
            ApiError::from(error).with_request_id(context.request_id().map(str::to_owned))
        })?;
    Ok(Json(SessionResponse::from_session(session)))
}

#[utoipa::path(
    post, path = "/api/v1/auth/logout", tag = "autenticazione",
    request_body = RefreshRequest,
    security(("bearer" = [])),
    responses(
        (status = 204, description = "Sessione revocata"),
        (status = 401, description = "Autenticazione richiesta", body = crate::error::ProblemDetails),
    )
)]
async fn logout(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Json(request): Json<RefreshRequest>,
) -> ApiResult<StatusCode> {
    state
        .logout
        .execute(
            &context,
            LogoutCommand {
                refresh_token: request.refresh_token,
            },
        )
        .await
        .map_err(|error| {
            ApiError::from(error).with_request_id(context.request_id().map(str::to_owned))
        })?;
    Ok(StatusCode::NO_CONTENT)
}
