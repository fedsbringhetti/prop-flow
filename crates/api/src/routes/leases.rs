use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use propflow_application::use_cases::lease::{
    ActivateLeaseCommand, AddLeasePartyCommand, DraftLeaseCommand, LeaseDetail, LeaseView,
    ListLeasesQuery, TerminateLeaseCommand,
};
use propflow_domain::DomainError;
use propflow_domain::lease::{
    Lease, LeaseParty, LeasePartyRole, LeaseStatus, LeaseType, RentFrequency,
};
use propflow_domain::shared::{Currency, Email, LeaseId, Money, PropertyId, RoomId};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::Authenticated;
use crate::routes::{amount, format_amount};
use crate::state::AppState;

/// Attivazione e disdetta sono sotto-risorse e non un `PATCH` sullo stato: hanno
/// effetti propri — firma, occupazione del periodo, data di fine rapporto — e la
/// stessa scelta è già stata fatta per l'archiviazione degli immobili.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/leases", post(create).get(list))
        .route("/api/v1/leases/{id}", get(detail))
        .route("/api/v1/leases/{id}/parties", post(add_party))
        .route("/api/v1/leases/{id}/activate", post(activate))
        .route("/api/v1/leases/{id}/terminate", post(terminate))
}

// ---------------------------------------------------------------------------
// Rappresentazioni di trasporto
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LeaseTypeDto {
    WholeProperty,
    Room,
}

impl From<LeaseType> for LeaseTypeDto {
    fn from(value: LeaseType) -> Self {
        match value {
            LeaseType::WholeProperty => Self::WholeProperty,
            LeaseType::Room => Self::Room,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LeaseStatusDto {
    Draft,
    PendingSignature,
    Active,
    /// Derivato dalle date, mai memorizzato: compare solo in `effective_status`.
    Expiring,
    /// Derivato dalle date, mai memorizzato.
    Expired,
    Terminated,
    Cancelled,
}

impl From<LeaseStatusDto> for LeaseStatus {
    fn from(value: LeaseStatusDto) -> Self {
        match value {
            LeaseStatusDto::Draft => Self::Draft,
            LeaseStatusDto::PendingSignature => Self::PendingSignature,
            LeaseStatusDto::Active => Self::Active,
            LeaseStatusDto::Expiring => Self::Expiring,
            LeaseStatusDto::Expired => Self::Expired,
            LeaseStatusDto::Terminated => Self::Terminated,
            LeaseStatusDto::Cancelled => Self::Cancelled,
        }
    }
}

impl From<LeaseStatus> for LeaseStatusDto {
    fn from(value: LeaseStatus) -> Self {
        match value {
            LeaseStatus::Draft => Self::Draft,
            LeaseStatus::PendingSignature => Self::PendingSignature,
            LeaseStatus::Active => Self::Active,
            LeaseStatus::Expiring => Self::Expiring,
            LeaseStatus::Expired => Self::Expired,
            LeaseStatus::Terminated => Self::Terminated,
            LeaseStatus::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RentFrequencyDto {
    Monthly,
    Quarterly,
    Semiannual,
    Annual,
}

impl From<RentFrequencyDto> for RentFrequency {
    fn from(value: RentFrequencyDto) -> Self {
        match value {
            RentFrequencyDto::Monthly => Self::Monthly,
            RentFrequencyDto::Quarterly => Self::Quarterly,
            RentFrequencyDto::Semiannual => Self::Semiannual,
            RentFrequencyDto::Annual => Self::Annual,
        }
    }
}

impl From<RentFrequency> for RentFrequencyDto {
    fn from(value: RentFrequency) -> Self {
        match value {
            RentFrequency::Monthly => Self::Monthly,
            RentFrequency::Quarterly => Self::Quarterly,
            RentFrequency::Semiannual => Self::Semiannual,
            RentFrequency::Annual => Self::Annual,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LeasePartyRoleDto {
    Tenant,
    Cotenant,
    Guarantor,
}

impl From<LeasePartyRoleDto> for LeasePartyRole {
    fn from(value: LeasePartyRoleDto) -> Self {
        match value {
            LeasePartyRoleDto::Tenant => Self::Tenant,
            LeasePartyRoleDto::Cotenant => Self::Cotenant,
            LeasePartyRoleDto::Guarantor => Self::Guarantor,
        }
    }
}

impl From<LeasePartyRole> for LeasePartyRoleDto {
    fn from(value: LeasePartyRole) -> Self {
        match value {
            LeasePartyRole::Tenant => Self::Tenant,
            LeasePartyRole::Cotenant => Self::Cotenant,
            LeasePartyRole::Guarantor => Self::Guarantor,
        }
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateLeaseRequest {
    pub property_id: Uuid,
    /// Presente per un contratto per stanza. Il tipo di contratto si deduce da qui:
    /// dichiararlo a parte sarebbe solo un modo di contraddirlo.
    #[serde(default)]
    pub room_id: Option<Uuid>,
    pub start_date: NaiveDate,
    /// Esclusiva: il contratto copre `[start_date, end_date)`. Assente = a tempo
    /// indeterminato.
    #[serde(default)]
    pub end_date: Option<NaiveDate>,
    /// Stringa decimale (`"800.00"`).
    #[schema(example = "800.00")]
    pub rent_amount: String,
    /// ISO 4217. Se omessa, `EUR`.
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub deposit_amount: Option<String>,
    #[serde(default)]
    pub rent_frequency: Option<RentFrequencyDto>,
    /// Giorno del mese in cui scade il canone, 1–28.
    #[serde(default)]
    pub rent_due_day: Option<i16>,
    #[serde(default)]
    pub notice_months: Option<i16>,
    #[serde(default)]
    pub auto_renew: Option<bool>,
    #[serde(default)]
    pub renewal_months: Option<i16>,
    #[serde(default)]
    pub utilities_included: Option<bool>,
    #[serde(default)]
    pub istat_indexation: Option<bool>,
    #[serde(default)]
    pub notes: Option<String>,
}

impl CreateLeaseRequest {
    fn into_command(self) -> Result<DraftLeaseCommand, DomainError> {
        let currency = match self.currency {
            Some(code) => Currency::parse(code)?,
            None => Currency::EUR,
        };
        let rent = Money::non_negative(amount("rent_amount", &self.rent_amount)?, currency)?;
        let deposit = self
            .deposit_amount
            .as_deref()
            .map(|raw| amount("deposit_amount", raw).and_then(|value| Money::new(value, currency)))
            .transpose()?;

        Ok(DraftLeaseCommand {
            property_id: PropertyId::from_uuid(self.property_id),
            room_id: self.room_id.map(RoomId::from_uuid),
            start_date: self.start_date,
            end_date: self.end_date,
            rent,
            deposit,
            rent_frequency: self
                .rent_frequency
                .map_or(RentFrequency::Monthly, Into::into),
            rent_due_day: self.rent_due_day.unwrap_or(1),
            notice_months: self.notice_months,
            auto_renew: self.auto_renew.unwrap_or(false),
            renewal_months: self.renewal_months,
            utilities_included: self.utilities_included.unwrap_or(false),
            istat_indexation: self.istat_indexation.unwrap_or(false),
            notes: self.notes,
        })
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AddLeasePartyRequest {
    #[serde(default = "default_party_role")]
    pub role: LeasePartyRoleDto,
    #[schema(example = "Mario Rossi")]
    pub full_name: String,
    /// Serve a legare l'invito alla parte: l'accettazione confronta l'email
    /// dell'utente registrato con questa.
    #[serde(default)]
    pub email: Option<String>,
    /// Quota di ripartizione del canone, 0–100. Se omessa, 100.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub share_percent: Option<Decimal>,
    #[serde(default)]
    pub is_primary: Option<bool>,
}

const fn default_party_role() -> LeasePartyRoleDto {
    LeasePartyRoleDto::Tenant
}

impl AddLeasePartyRequest {
    fn into_command(self, lease_id: LeaseId) -> Result<AddLeasePartyCommand, DomainError> {
        Ok(AddLeasePartyCommand {
            lease_id,
            role: self.role.into(),
            full_name: self.full_name,
            email: self.email.as_deref().map(Email::parse).transpose()?,
            share_percent: self.share_percent.unwrap_or_else(|| Decimal::from(100)),
            is_primary: self.is_primary.unwrap_or(false),
        })
    }
}

#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
pub struct TerminateLeaseRequest {
    /// Motivo della disdetta. Non finisce negli eventi di dominio: è testo libero.
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct LeaseResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub property_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<Uuid>,
    pub lease_type: LeaseTypeDto,
    /// Stato memorizzato: cambia solo su azioni esplicite.
    pub status: LeaseStatusDto,
    /// Stato alla data odierna. Può valere `expiring` o `expired` mentre `status`
    /// resta `active`: la scadenza è un fatto del calendario e viene calcolata,
    /// così il client non deve ricalcolarla dalle date.
    pub effective_status: LeaseStatusDto,
    pub start_date: NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<NaiveDate>,
    pub notice_months: Option<i16>,
    pub auto_renew: bool,
    pub renewal_months: Option<i16>,
    pub rent_amount: String,
    pub currency: String,
    pub rent_frequency: RentFrequencyDto,
    pub rent_due_day: i16,
    pub deposit_amount: String,
    pub utilities_included: bool,
    pub istat_indexation: bool,
    pub signed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub terminated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub termination_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl LeaseResponse {
    fn from_view(view: &LeaseView) -> Self {
        Self::build(&view.lease, view.effective_status)
    }

    fn build(lease: &Lease, effective_status: LeaseStatus) -> Self {
        Self {
            id: lease.id().as_uuid(),
            organization_id: lease.org_id().as_uuid(),
            property_id: lease.property_id().as_uuid(),
            room_id: lease.room_id().map(|id| id.as_uuid()),
            lease_type: lease.lease_type().into(),
            status: lease.status().into(),
            effective_status: effective_status.into(),
            start_date: lease.start_date(),
            end_date: lease.end_date(),
            notice_months: lease.notice_months(),
            auto_renew: lease.auto_renew(),
            renewal_months: lease.renewal_months(),
            rent_amount: format_amount(lease.rent().amount()),
            currency: lease.currency().as_str().to_owned(),
            rent_frequency: lease.rent_frequency().into(),
            rent_due_day: lease.rent_due_day(),
            deposit_amount: format_amount(lease.deposit().amount()),
            utilities_included: lease.utilities_included(),
            istat_indexation: lease.istat_indexation(),
            signed_at: lease.signed_at(),
            terminated_at: lease.terminated_at(),
            termination_reason: lease.termination_reason().map(str::to_owned),
            notes: lease.notes().map(str::to_owned),
            created_at: lease.created_at(),
            updated_at: lease.updated_at(),
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct LeasePartyResponse {
    pub id: Uuid,
    pub lease_id: Uuid,
    /// Assente finché l'invitato non ha accettato: la parte esiste sul contratto
    /// prima che esista l'utente.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
    pub role: LeasePartyRoleDto,
    pub full_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[schema(value_type = String)]
    pub share_percent: Decimal,
    pub is_primary: bool,
    pub joined_at: Option<NaiveDate>,
    pub left_at: Option<NaiveDate>,
}

impl LeasePartyResponse {
    fn from_domain(party: &LeaseParty) -> Self {
        Self {
            id: party.id().as_uuid(),
            lease_id: party.lease_id().as_uuid(),
            user_id: party.user_id().map(|id| id.as_uuid()),
            role: party.role().into(),
            full_name: party.full_name().to_owned(),
            email: party.email().map(|email| email.as_str().to_owned()),
            share_percent: party.share_percent(),
            is_primary: party.is_primary(),
            joined_at: party.joined_at(),
            left_at: party.left_at(),
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct LeaseDetailResponse {
    #[serde(flatten)]
    pub lease: LeaseResponse,
    pub parties: Vec<LeasePartyResponse>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct LeasePageResponse {
    pub items: Vec<LeaseResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListLeasesParams {
    #[serde(default)]
    pub property_id: Option<Uuid>,
    /// Solo stati memorizzati: `expiring` ed `expired` sono derivati e vanno
    /// chiesti con `in_force`.
    #[serde(default)]
    pub status: Option<LeaseStatusDto>,
    /// `true` = solo i contratti in essere oggi.
    #[serde(default)]
    pub in_force: Option<bool>,
    #[serde(default)]
    pub after: Option<Uuid>,
    #[serde(default)]
    pub limit: Option<u16>,
}

// ---------------------------------------------------------------------------
// Handler: deserializzare, chiamare lo use case, serializzare. Nulla di più.
// ---------------------------------------------------------------------------

#[utoipa::path(
    post, path = "/api/v1/leases", tag = "contratti",
    request_body = CreateLeaseRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Contratto redatto in bozza", body = LeaseResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Immobile o stanza non trovati", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi", body = crate::error::ProblemDetails),
    )
)]
async fn create(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Json(request): Json<CreateLeaseRequest>,
) -> ApiResult<(StatusCode, Json<LeaseResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let command = request.into_command().map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;

    let view = state
        .draft_lease
        .execute(&context, command)
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok((StatusCode::CREATED, Json(LeaseResponse::from_view(&view))))
}

#[utoipa::path(
    get, path = "/api/v1/leases", tag = "contratti",
    params(ListLeasesParams),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Pagina di contratti", body = LeasePageResponse),
        (status = 422, description = "Filtro su uno stato derivato", body = crate::error::ProblemDetails),
    )
)]
async fn list(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Query(params): Query<ListLeasesParams>,
) -> ApiResult<Json<LeasePageResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let page = state
        .list_leases
        .execute(
            &context,
            ListLeasesQuery {
                property_id: params.property_id.map(PropertyId::from_uuid),
                status: params.status.map(Into::into),
                in_force: params.in_force,
                after: params.after.map(LeaseId::from_uuid),
                limit: params.limit,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(LeasePageResponse {
        items: page.items.iter().map(LeaseResponse::from_view).collect(),
        next_cursor: page.next_cursor.map(|id| id.as_uuid()),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/leases/{id}", tag = "contratti",
    params(("id" = Uuid, Path, description = "Identificatore del contratto")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Contratto con le sue parti", body = LeaseDetailResponse),
        (status = 404, description = "Non trovato o non visibile", body = crate::error::ProblemDetails),
    )
)]
async fn detail(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<LeaseDetailResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let LeaseDetail {
        lease,
        effective_status,
        parties,
    } = state
        .get_lease
        .execute(&context, LeaseId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(LeaseDetailResponse {
        lease: LeaseResponse::build(&lease, effective_status),
        parties: parties
            .iter()
            .map(LeasePartyResponse::from_domain)
            .collect(),
    }))
}

#[utoipa::path(
    post, path = "/api/v1/leases/{id}/parties", tag = "contratti",
    params(("id" = Uuid, Path, description = "Identificatore del contratto")),
    request_body = AddLeasePartyRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Parte aggiunta", body = LeasePartyResponse),
        (status = 404, description = "Contratto non trovato", body = crate::error::ProblemDetails),
        (status = 409, description = "Esiste già un intestatario principale", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi", body = crate::error::ProblemDetails),
    )
)]
async fn add_party(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<AddLeasePartyRequest>,
) -> ApiResult<(StatusCode, Json<LeasePartyResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let command = request
        .into_command(LeaseId::from_uuid(id))
        .map_err(|error| {
            ApiError::from(propflow_application::AppError::Validation(error))
                .with_request_id(request_id.clone())
        })?;

    let party = state
        .add_lease_party
        .execute(&context, command)
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok((
        StatusCode::CREATED,
        Json(LeasePartyResponse::from_domain(&party)),
    ))
}

#[utoipa::path(
    post, path = "/api/v1/leases/{id}/activate", tag = "contratti",
    params(("id" = Uuid, Path, description = "Identificatore del contratto")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Contratto attivato", body = LeaseResponse),
        (status = 404, description = "Non trovato o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Stato non ammesso, parti incomplete, o periodo già occupato", body = crate::error::ProblemDetails),
    )
)]
async fn activate(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<LeaseResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let view = state
        .activate_lease
        .execute(
            &context,
            ActivateLeaseCommand {
                id: LeaseId::from_uuid(id),
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(Json(LeaseResponse::from_view(&view)))
}

#[utoipa::path(
    post, path = "/api/v1/leases/{id}/terminate", tag = "contratti",
    params(("id" = Uuid, Path, description = "Identificatore del contratto")),
    request_body = TerminateLeaseRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Contratto disdetto", body = LeaseResponse),
        (status = 404, description = "Non trovato o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Solo un contratto attivo può essere disdetto", body = crate::error::ProblemDetails),
    )
)]
async fn terminate(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<TerminateLeaseRequest>,
) -> ApiResult<Json<LeaseResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let view = state
        .terminate_lease
        .execute(
            &context,
            TerminateLeaseCommand {
                id: LeaseId::from_uuid(id),
                reason: request.reason,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(Json(LeaseResponse::from_view(&view)))
}
