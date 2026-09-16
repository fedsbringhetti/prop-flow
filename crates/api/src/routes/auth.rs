use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::routing::post;
use axum::{Json, Router};
use propflow_application::use_cases::auth::{
    AcceptOwnerInvitationCommand, AuthenticatedSession, ConfirmPasswordResetCommand, LoginCommand,
    LogoutCommand, RefreshSessionCommand, RegisterTenantFromInvitationCommand,
    RequestPasswordResetCommand,
};
use propflow_domain::shared::Email;
use propflow_domain::{DomainError, DomainResult};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::{Anonymous, Authenticated, clear_refresh_cookie, set_refresh_cookie};
use crate::state::AppState;

/// Le rotte dell'autenticazione, e le sole che il cookie di sessione
/// raggiunge: `Path=/api/v1/auth` lo limita a questo prefisso.
///
/// Login e le due registrazioni da invito restano `Anonymous` perché non c'è
/// ancora nessuno da autenticare; **anche il refresh lo è**, e non è una svista
/// — chi lo chiama presenta un token, non un'identità, e pretendere un
/// `Authorization` valido renderebbe impossibile rinnovare proprio quando
/// serve, cioè con l'access token già scaduto.
///
/// **Nessun `/register`** (ADR-053, `F6`): si entra solo da un invito, da
/// proprietario o da inquilino. La rotta non risponde più a nessun verbo —
/// vedi `docs/05-api.md` — invece di accettare un corpo con un token in più,
/// che su una rotta chiamata ancora `register` insegnerebbe il contrario di
/// ciò che questa decisione stabilisce.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/refresh", post(refresh))
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/password-reset", post(request_password_reset))
        .route(
            "/api/v1/auth/password-reset/confirm",
            post(confirm_password_reset),
        )
        .route(
            "/api/v1/auth/accept-owner-invitation",
            post(accept_owner_invitation),
        )
        .route(
            "/api/v1/auth/register-from-invitation",
            post(register_from_invitation),
        )
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct PasswordResetRequest {
    #[schema(example = "mario.rossi@example.it")]
    pub email: String,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct PasswordResetConfirmRequest {
    /// Il codice ricevuto per email, da incollare.
    ///
    /// **Un codice e non un link**, come per l'invito: un token nell'URL finisce
    /// nella cronologia, nel `Referer` e nei log di ogni proxy davanti al client
    /// (ADR-045, ADR-052).
    pub code: String,
    /// Almeno 12 caratteri, come alla registrazione.
    #[schema(example = "cavallo batteria graffetta")]
    pub new_password: String,
}

/// Accetta un invito da proprietario: crea utente e organizzazione insieme,
/// dietro un token emesso da `ops/dev/issue-owner-invitation.sh` invece che su
/// una rotta aperta a chiunque (ADR-053).
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct AcceptOwnerInvitationRequest {
    /// Il token in chiaro consegnato fuori banda da chi lo ha emesso.
    pub token: String,
    #[schema(example = "Mario Rossi")]
    pub full_name: String,
    /// Almeno 12 caratteri.
    #[schema(example = "cavallo batteria graffetta")]
    pub password: String,
    /// Nome dell'organizzazione personale. Se omesso si usa il nome dell'utente.
    #[serde(default)]
    pub organization_name: Option<String>,
}

/// Registra un inquilino a partire dall'invito.
///
/// **Nessun campo `email`**: è quella dell'invito, non del corpo — un invito è
/// per una persona, e chi vuole un altro indirizzo chiede un altro invito
/// (ADR-053, decisione 6). Il campo si riconosce e si rifiuta se presente, con
/// la stessa forma di `email` in `PATCH /me`: un campo sconosciuto si ignora
/// ovunque nell'API, ma questo non è un campo sconosciuto — è quello che
/// chiunque proverà a mandare per primo, e ignorarlo darebbe `201` con un
/// account collegato a un indirizzo diverso da quello scritto nel corpo, senza
/// alcun indizio.
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct RegisterFromInvitationRequest {
    /// Il token in chiaro dell'invito da inquilino.
    pub token: String,
    #[schema(example = "Mario Rossi")]
    pub full_name: String,
    #[schema(example = "cavallo batteria graffetta")]
    pub password: String,
    /// Si riconosce e si rifiuta con `422` se presente: vedi il commento del tipo.
    #[serde(default)]
    pub email: Option<String>,
}

impl RegisterFromInvitationRequest {
    fn into_command(self) -> DomainResult<RegisterTenantFromInvitationCommand> {
        if self.email.is_some() {
            return Err(DomainError::validation(
                "email",
                "cannot be set here: it is the invitation's email, not a field of this request",
            ));
        }
        Ok(RegisterTenantFromInvitationCommand {
            token: self.token,
            full_name: self.full_name,
            password: self.password,
        })
    }
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

#[utoipa::path(
    post, path = "/api/v1/auth/password-reset", tag = "autenticazione",
    request_body = PasswordResetRequest,
    responses(
        (status = 202, description = "Se quell'indirizzo ha un account, riceverà un codice. **La risposta è la stessa in ogni caso**"),
        (status = 422, description = "Indirizzo non valido", body = crate::error::ProblemDetails),
    )
)]
async fn request_password_reset(
    State(state): State<AppState>,
    Anonymous(context): Anonymous,
    Json(request): Json<PasswordResetRequest>,
) -> ApiResult<StatusCode> {
    let request_id = context.request_id().map(str::to_owned);
    let email = Email::parse(&request.email).map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;

    state
        .request_password_reset
        .execute(RequestPasswordResetCommand { email })
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    // **`202` e nient'altro, sempre.** Un indirizzo noto e uno sconosciuto ricevono
    // la stessa cosa: distinguerli sarebbe un oracolo per sapere chi ha un account
    // (ADR-017). Nemmeno un corpo, che sarebbe un posto in cui far entrare una
    // differenza per distrazione.
    Ok(StatusCode::ACCEPTED)
}

#[utoipa::path(
    post, path = "/api/v1/auth/password-reset/confirm", tag = "autenticazione",
    request_body = PasswordResetConfirmRequest,
    responses(
        (status = 204, description = "Password sostituita. **Tutte le sessioni aperte sono revocate**, compresa quella di chi recupera"),
        (status = 401, description = "Codice sconosciuto, scaduto o già usato: i tre casi non si distinguono", body = crate::error::ProblemDetails),
        (status = 422, description = "Password troppo debole", body = crate::error::ProblemDetails),
    )
)]
async fn confirm_password_reset(
    State(state): State<AppState>,
    Anonymous(context): Anonymous,
    Json(request): Json<PasswordResetConfirmRequest>,
) -> ApiResult<StatusCode> {
    let request_id = context.request_id().map(str::to_owned);
    state
        .confirm_password_reset
        .execute(ConfirmPasswordResetCommand {
            code: request.code,
            new_password: request.new_password,
        })
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    // Nessun cookie da ripulire: chi recupera non aveva una sessione, e quelle che
    // esistevano sono state revocate in database — un cookie che sopravvivesse
    // presenterebbe un token che non vale più.
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post, path = "/api/v1/auth/accept-owner-invitation", tag = "autenticazione",
    request_body = AcceptOwnerInvitationRequest,
    responses(
        (status = 201, description = "Proprietario e organizzazione personale creati. Sessione e cookie come il login", body = SessionResponse),
        (status = 401, description = "Token sconosciuto, scaduto, revocato o già accettato: i quattro casi non si distinguono", body = crate::error::ProblemDetails),
        (status = 409, description = "Email già registrata", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi", body = crate::error::ProblemDetails),
    )
)]
async fn accept_owner_invitation(
    State(state): State<AppState>,
    Anonymous(context): Anonymous,
    Json(request): Json<AcceptOwnerInvitationRequest>,
) -> ApiResult<(StatusCode, HeaderMap, Json<SessionResponse>)> {
    let session = state
        .accept_owner_invitation
        .execute(
            &context,
            AcceptOwnerInvitationCommand {
                token: request.token,
                full_name: request.full_name,
                password: request.password,
                organization_name: request.organization_name,
            },
        )
        .await
        .map_err(|error| {
            ApiError::from(error).with_request_id(context.request_id().map(str::to_owned))
        })?;

    let (headers, body) = session_reply(&state, &session)?;
    Ok((StatusCode::CREATED, headers, body))
}

#[utoipa::path(
    post, path = "/api/v1/auth/register-from-invitation", tag = "autenticazione",
    request_body = RegisterFromInvitationRequest,
    responses(
        (status = 201, description = "Inquilino registrato **dentro l'organizzazione di chi lo ha invitato**. Sessione e cookie come il login", body = SessionResponse),
        (status = 401, description = "Token sconosciuto, scaduto, usato o revocato: i quattro casi non si distinguono", body = crate::error::ProblemDetails),
        (status = 409, description = "Email già registrata, oppure il token è di un invito da **proprietario** valido — rimandare a `accept-owner-invitation` con il nome dell'organizzazione", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi, oppure `email` presente nel corpo", body = crate::error::ProblemDetails),
    )
)]
async fn register_from_invitation(
    State(state): State<AppState>,
    Anonymous(context): Anonymous,
    Json(request): Json<RegisterFromInvitationRequest>,
) -> ApiResult<(StatusCode, HeaderMap, Json<SessionResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let command = request.into_command().map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;

    let session = state
        .register_tenant_from_invitation
        .execute(&context, command)
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    let (headers, body) = session_reply(&state, &session)?;
    Ok((StatusCode::CREATED, headers, body))
}
