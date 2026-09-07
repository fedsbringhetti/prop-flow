use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use propflow_domain::identity::OrganizationMembership;
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
    Router::new().route("/api/v1/organizations", get(list))
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
