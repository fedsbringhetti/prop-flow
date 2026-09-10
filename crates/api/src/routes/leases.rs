use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use propflow_application::error::AppError;
use propflow_application::use_cases::lease::{
    ActivateLeaseCommand, AddLeasePartyCommand, CancelLeaseCommand, CorrectLeasePartyCommand,
    DraftLeaseCommand, LeaseDetail, LeaseView, ListLeasesQuery, RemoveLeasePartyCommand,
    TerminateLeaseCommand,
};
use propflow_domain::DomainError;
use propflow_domain::lease::{
    Lease, LeaseParty, LeasePartyChanges, LeasePartyRole, LeaseStatus, LeaseType, RentFrequency,
};
use propflow_domain::ports::LeaseLabels;
use propflow_domain::property::Patch;
use propflow_domain::shared::{Currency, Email, LeaseId, LeasePartyId, Money, PropertyId, RoomId};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::Authenticated;
use crate::routes::{amount, explicit_null, format_amount};
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
        .route("/api/v1/leases/{id}/cancel", post(cancel))
        // Le parti si creano annidate e si correggono da una rotta propria: è la
        // forma delle stanze, che nascono sotto l'immobile e vivono sotto
        // `/rooms/{id}`. Una `PATCH` annidata chiederebbe due identificatori per
        // raggiungere una riga che ne ha già uno.
        .route(
            "/api/v1/lease-parties/{id}",
            patch(correct_party).delete(remove_party),
        )
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

/// La correzione di una parte, con la semantica `PATCH` del progetto: campo
/// assente = invariato, `null` = azzerato, valore = impostato.
///
/// **`user_id` non c'è**: collegare un utente è l'accettazione di un invito, non
/// una modifica amministrativa. **`left_at` nemmeno**: far uscire un inquilino è
/// un fatto con conseguenze proprie, e non è questa operazione.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CorrectLeasePartyRequest {
    #[serde(default)]
    pub role: Option<LeasePartyRoleDto>,
    #[serde(default)]
    pub full_name: Option<String>,
    /// Correggibile **solo finché la parte non è collegata a un utente**: dopo,
    /// cambiarla sposterebbe sotto i piedi il rapporto di chi ha accettato.
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub email: Patch<String>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub phone: Patch<String>,
    /// Quota di ripartizione del canone, 0–100.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub share_percent: Option<Decimal>,
    #[serde(default)]
    pub is_primary: Option<bool>,
}

impl CorrectLeasePartyRequest {
    fn into_changes(self) -> Result<LeasePartyChanges, DomainError> {
        let email = match self.email {
            None => None,
            Some(None) => Some(None),
            Some(Some(raw)) => Some(Some(Email::parse(&raw)?)),
        };
        Ok(LeasePartyChanges {
            full_name: self.full_name,
            email,
            phone: self.phone,
            role: self.role.map(Into::into),
            share_percent: self.share_percent,
            is_primary: self.is_primary,
        })
    }
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
    /// Come si chiama l'immobile, per distinguere una riga dall'altra: senza,
    /// l'elenco mostra tipo e date e i contratti si somigliano tutti.
    ///
    /// **Assente sulle risposte a una singola operazione** — attivazione,
    /// disdetta, annullamento — dove chi chiama ha già il contratto sotto gli
    /// occhi, e **quando l'immobile è archiviato**, che è il caso in cui la sua
    /// riga non si legge più.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_label: Option<String>,
    /// Assente sui contratti sull'immobile intero, che non hanno una stanza.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_name: Option<String>,
    /// Il nome dell'**intestatario**, cioè della parte marcata `is_primary`.
    ///
    /// Viaggia con gli altri due e per la stessa ragione, che *dove* non completa:
    /// due contratti sulla stessa stanza in due anni diversi hanno lo stesso
    /// immobile e la stessa stanza, e a distinguerli è chi ci abita. Serve a
    /// scegliere un contratto in una tendina senza doverne aprire uno per volta.
    ///
    /// **Assente** su un contratto senza intestatario — una bozza a cui non è
    /// ancora stato aggiunto nessuno — e sulle risposte a una singola operazione,
    /// come gli altri due.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_tenant: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl LeaseResponse {
    fn from_view(view: &LeaseView) -> Self {
        Self::build(&view.lease, view.effective_status)
    }

    /// Con i nomi, per le due letture che li caricano.
    fn from_view_with_labels(view: &LeaseView, labels: Option<&LeaseLabels>) -> Self {
        let mut response = Self::build(&view.lease, view.effective_status);
        if let Some(labels) = labels {
            response.property_label.clone_from(&labels.property_label);
            response.room_name.clone_from(&labels.room_name);
            response.primary_tenant.clone_from(&labels.primary_tenant);
        }
        response
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
            // I nomi non stanno nell'aggregato: li mette `from_view_with_labels`,
            // che è l'unica via che li ha caricati.
            property_label: None,
            room_name: None,
            primary_tenant: None,
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
    /// Serve alla correzione: un campo che si può cambiare e non si può leggere
    /// si corregge alla cieca.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    /// Sempre due decimali, come ogni altro numero sul filo. La scala di un
    /// `Decimal` dipende da come è stato costruito: senza normalizzare, la stessa
    /// quota usciva `"100"` nella risposta alla correzione e `"100.00"` alla
    /// lettura successiva, che è una differenza che un client può notare.
    pub share_percent: String,
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
            phone: party.phone().map(str::to_owned),
            share_percent: format_amount(party.share_percent()),
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
        items: page
            .items
            .iter()
            .map(|item| LeaseResponse::from_view_with_labels(&item.view, item.labels.as_ref()))
            .collect(),
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
        labels,
    } = state
        .get_lease
        .execute(&context, LeaseId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(LeaseDetailResponse {
        lease: LeaseResponse::from_view_with_labels(
            &LeaseView {
                lease,
                effective_status,
            },
            labels.as_ref(),
        ),
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

/// Nessun corpo, a differenza della disdetta: annullare non registra un fatto
/// datato — non c'è stato alcun rapporto — quindi non c'è una ragione da
/// conservare accanto a una data che non esiste.
#[utoipa::path(
    post, path = "/api/v1/leases/{id}/cancel", tag = "contratti",
    params(("id" = Uuid, Path, description = "Identificatore del contratto")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Contratto annullato", body = LeaseResponse),
        (status = 404, description = "Non trovato o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Solo una bozza o un contratto in attesa di firma si annulla", body = crate::error::ProblemDetails),
    )
)]
async fn cancel(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<LeaseResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let view = state
        .cancel_lease
        .execute(
            &context,
            CancelLeaseCommand {
                id: LeaseId::from_uuid(id),
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(Json(LeaseResponse::from_view(&view)))
}

#[utoipa::path(
    patch, path = "/api/v1/lease-parties/{id}", tag = "contratti",
    params(("id" = Uuid, Path, description = "Identificatore della parte")),
    request_body = CorrectLeasePartyRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Parte corretta", body = LeasePartyResponse),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Email bloccata da un utente collegato, o intestatario già presente", body = crate::error::ProblemDetails),
        (status = 422, description = "Quota fuori da 0–100, o email non valida", body = crate::error::ProblemDetails),
    )
)]
async fn correct_party(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<CorrectLeasePartyRequest>,
) -> ApiResult<Json<LeasePartyResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let changes = request.into_changes().map_err(|error| {
        ApiError::from(AppError::from(error)).with_request_id(request_id.clone())
    })?;
    let party = state
        .correct_lease_party
        .execute(
            &context,
            CorrectLeasePartyCommand {
                id: LeasePartyId::from_uuid(id),
                changes,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(Json(LeasePartyResponse::from_domain(&party)))
}

/// `204`, e non è una cancellazione vera: la riga resta con `deleted_at`, come
/// ovunque nel progetto.
#[utoipa::path(
    delete, path = "/api/v1/lease-parties/{id}", tag = "contratti",
    params(("id" = Uuid, Path, description = "Identificatore della parte")),
    security(("bearer" = [])),
    responses(
        (status = 204, description = "Parte tolta"),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "La parte è collegata a un utente: non si toglie", body = crate::error::ProblemDetails),
    )
)]
async fn remove_party(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let request_id = context.request_id().map(str::to_owned);
    state
        .remove_lease_party
        .execute(
            &context,
            RemoveLeasePartyCommand {
                id: LeasePartyId::from_uuid(id),
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(StatusCode::NO_CONTENT)
}
