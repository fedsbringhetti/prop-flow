use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use propflow_domain::identity::OrganizationMembership;
use propflow_domain::shared::OrgId;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::Authenticated;
use crate::state::AppState;

/// Le organizzazioni di chi chiama.
///
/// **Senza prefisso `me`**: la Row Level Security restringe già alle proprie, e il
/// percorso dice il vero senza aggiungere una parola. Non prende parametri, e in
/// particolare **non prende `X-Org-Id`**: chiedere quale organizzazione sia attiva
/// per sapere quali si hanno sarebbe circolare.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/organizations", get(list))
        // L'uscita prende l'identificatore nel percorso, al contrario di
        // `/api/v1/me`: qui non si agisce su di sé, si agisce su un **rapporto**, e
        // quale sia va detto. Non è quella attiva per definizione — chi ne ha due
        // può uscire da quella che non sta guardando.
        .route("/api/v1/organizations/{id}/leave", post(leave))
}

/// Un'organizzazione dell'elenco.
///
/// Quattro campi e non l'aggregato intero: fuso orario, valuta predefinita e
/// natura dell'organizzazione non servono a un selettore, e una risposta più
/// larga è più facile da allargare che da stringere.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct OrganizationResponse {
    pub id: Uuid,
    /// Il nome leggibile. **È la ragione per cui questa rotta esiste**: senza,
    /// un client può mostrare soltanto l'identificatore.
    #[schema(example = "Immobili di Mario Rossi")]
    pub name: String,
    #[schema(example = "immobili-di-mario-rossi-4f21ab")]
    pub slug: String,
    /// Il ruolo che il chiamante ha **in questa** organizzazione: la stessa
    /// persona è proprietario della propria e inquilino in quella di chi le
    /// affitta casa.
    #[schema(example = "tenant")]
    pub role: String,
}

impl OrganizationResponse {
    fn from_domain(membership: &OrganizationMembership) -> Self {
        Self {
            id: membership.org_id().as_uuid(),
            name: membership.name().to_owned(),
            slug: membership.slug().as_str().to_owned(),
            role: membership.role().as_str().to_owned(),
        }
    }
}

/// L'elenco, **senza cursore**.
///
/// Le organizzazioni di una persona sono poche per costruzione, e una
/// paginazione su un elenco che sta in una riga è cerimonia. È la stessa forma di
/// `PropertyResidentListResponse`, per la stessa ragione.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct OrganizationListResponse {
    pub items: Vec<OrganizationResponse>,
}

#[utoipa::path(
    get, path = "/api/v1/organizations", tag = "organizzazioni",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Le organizzazioni di cui il chiamante è membro attivo. Risponde anche senza `X-Org-Id`", body = OrganizationListResponse),
        (status = 401, description = "Autenticazione richiesta", body = crate::error::ProblemDetails),
    )
)]
async fn list(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
) -> ApiResult<Json<OrganizationListResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let memberships = state
        .list_organizations
        .execute(&context)
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(OrganizationListResponse {
        items: memberships
            .iter()
            .map(OrganizationResponse::from_domain)
            .collect(),
    }))
}

/// Che cosa resta dopo essere usciti.
///
/// Due campi e non un corpo vuoto: chi esce dall'unica organizzazione in cui
/// abitava resta con la **propria**, personale e vuota — è il caso di
/// [ADR-045](../../../../../docs/08-decisioni.md) al contrario — e il client deve
/// poter dire dove si trova adesso senza rileggere l'elenco per indovinarlo.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct LeftOrganizationResponse {
    pub organization_id: Uuid,
    /// Il ruolo che si aveva. Serve al messaggio che il client mostra dopo: uscire
    /// da inquilino e uscire da gestore lasciano due situazioni diverse.
    #[schema(example = "tenant")]
    pub role: String,
}

#[utoipa::path(
    post, path = "/api/v1/organizations/{id}/leave", tag = "organizzazioni",
    params(("id" = Uuid, Path, description = "L'organizzazione da cui uscire")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Usciti. L'appartenenza passa a `left` e **la riga resta**: è ciò che permette di rientrare accettando un invito nuovo", body = LeftOrganizationResponse),
        (status = 401, description = "Autenticazione richiesta", body = crate::error::ProblemDetails),
        (status = 404, description = "`membership.not_found`: non si è membri di quella organizzazione, oppure non è visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "`membership.lease_in_force` con un contratto in essere — il rifiuto arriva dalla policy, non da un `if`; `membership.last_owner` se si è l'ultimo proprietario; `membership.already_left` se si era già usciti", body = crate::error::ProblemDetails),
    )
)]
async fn leave(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<LeftOrganizationResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let member = state
        .leave_organization
        .execute(&context, OrgId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(LeftOrganizationResponse {
        organization_id: member.org_id().as_uuid(),
        role: member.role().as_str().to_owned(),
    }))
}
