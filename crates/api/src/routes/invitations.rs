use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, post};
use axum::{Json, Router};
use propflow_application::use_cases::invitation::{
    AcceptInvitationCommand, InviteTenantCommand, IssuedInvitation, RevokeInvitationCommand,
};
use propflow_domain::identity::Invitation;
use propflow_domain::shared::{Email, InvitationId, LeaseId};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::Authenticated;
use crate::state::AppState;

/// L'accettazione sta sotto `/auth` e non sotto `/invitations/{id}`: chi accetta
/// non conosce l'identificatore dell'invito, conosce il token — e cercare l'invito
/// per id significherebbe esporne l'esistenza a chi ne indovina uno.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/leases/{id}/invitations", post(invite))
        .route("/api/v1/invitations/{id}", delete(revoke))
        .route("/api/v1/auth/accept-invitation", post(accept))
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct InviteTenantRequest {
    #[schema(example = "inquilino@example.it")]
    pub email: String,
    /// Nome da usare se la parte contrattuale con questa email non esiste ancora.
    #[schema(example = "Mario Rossi")]
    pub full_name: String,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct InvitationResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease_id: Option<Uuid>,
    pub email: String,
    pub role: &'static str,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// Token di accettazione **in chiaro**, presente solo in questa risposta: in
    /// database ne resta lo hash, come per i refresh token. Finché la porta
    /// `Notifier` non è implementata, è il proprietario a consegnarlo.
    pub accept_token: String,
}

impl InvitationResponse {
    fn from_issued(issued: &IssuedInvitation) -> Self {
        let invitation: &Invitation = &issued.invitation;
        Self {
            id: invitation.id().as_uuid(),
            organization_id: invitation.org_id().as_uuid(),
            lease_id: invitation.lease_id().map(|id| id.as_uuid()),
            email: invitation.email().as_str().to_owned(),
            role: invitation.role().as_str(),
            expires_at: invitation.expires_at(),
            accept_token: issued.token.clone(),
        }
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AcceptInvitationRequest {
    pub token: String,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct AcceptInvitationResponse {
    /// Organizzazione di cui si è appena diventati membri. Da passare come
    /// `X-Org-Id`, oppure da ottenere nel token al prossimo login.
    pub organization_id: Uuid,
    pub role: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease_id: Option<Uuid>,
}

#[utoipa::path(
    post, path = "/api/v1/leases/{id}/invitations", tag = "inviti",
    params(("id" = Uuid, Path, description = "Identificatore del contratto")),
    request_body = InviteTenantRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Invito emesso, con il token da consegnare", body = InvitationResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Contratto non trovato o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Esiste già un invito pendente per questa email", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi", body = crate::error::ProblemDetails),
    )
)]
async fn invite(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<InviteTenantRequest>,
) -> ApiResult<(StatusCode, Json<InvitationResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let email = Email::parse(&request.email).map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;

    let issued = state
        .invite_tenant
        .execute(
            &context,
            InviteTenantCommand {
                lease_id: LeaseId::from_uuid(id),
                email,
                full_name: request.full_name,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok((
        StatusCode::CREATED,
        Json(InvitationResponse::from_issued(&issued)),
    ))
}

#[utoipa::path(
    delete, path = "/api/v1/invitations/{id}", tag = "inviti",
    params(("id" = Uuid, Path, description = "Identificatore dell'invito")),
    security(("bearer" = [])),
    responses(
        (status = 204, description = "Invito revocato"),
        (status = 404, description = "Non trovato o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Invito già accettato", body = crate::error::ProblemDetails),
    )
)]
async fn revoke(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let request_id = context.request_id().map(str::to_owned);
    state
        .revoke_invitation
        .execute(
            &context,
            RevokeInvitationCommand {
                id: InvitationId::from_uuid(id),
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post, path = "/api/v1/auth/accept-invitation", tag = "inviti",
    request_body = AcceptInvitationRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Invito accettato: si è membri dell'organizzazione", body = AcceptInvitationResponse),
        (status = 401, description = "Token sconosciuto, scaduto o revocato", body = crate::error::ProblemDetails),
        (status = 409, description = "Invito già accettato, o già membri", body = crate::error::ProblemDetails),
    )
)]
async fn accept(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Json(request): Json<AcceptInvitationRequest>,
) -> ApiResult<Json<AcceptInvitationResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let accepted = state
        .accept_invitation
        .execute(
            &context,
            AcceptInvitationCommand {
                token: request.token,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(AcceptInvitationResponse {
        organization_id: accepted.org_id.as_uuid(),
        role: accepted.role.as_str(),
        lease_id: accepted.lease_id.map(|id| id.as_uuid()),
    }))
}
