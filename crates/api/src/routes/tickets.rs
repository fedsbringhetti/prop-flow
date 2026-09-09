use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use propflow_application::authorization::discloses_estimated_cost;
use propflow_application::use_cases::ticket::{
    AssignTicketCommand, CancelTicketCommand, CloseTicketCommand, CorrectTicketReportCommand,
    ListTicketMessagesCommand, ListTicketsQuery, OpenTicketCommand, People,
    PostTicketMessageCommand, ReopenTicketCommand, StartTicketWorkCommand, SuspendTicketCommand,
    TicketDetail, TriageTicketCommand, UpdateTicketCostsCommand,
};
use propflow_domain::DomainError;
use propflow_domain::property::Patch;
use propflow_domain::shared::{LeaseId, PropertyId, RoomId, TicketId, TicketMessageId, UserId};
use propflow_domain::ticket::{
    MessageBody, Ticket, TicketActor, TicketCategory, TicketCorrection, TicketCosts,
    TicketDescription, TicketMessage, TicketPriority, TicketStatus, TicketStatusChange,
    TicketTitle, WaitingOn,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::Authenticated;
use crate::routes::{explicit_null, format_amount};
use crate::state::AppState;

/// Le transizioni sono **sotto-risorse e non un `PATCH` sullo stato**.
///
/// Stessa scelta di attivazione e disdetta dei contratti e dell'archiviazione
/// degli immobili, e per la stessa ragione: ognuna ha effetti propri —
/// `closed_at`, una riga di storico, un evento — e un `PATCH` che li producesse di
/// soppiatto renderebbe invisibile che siano avvenuti. Il `PATCH` resta per ciò
/// che è davvero un attributo: i costi.
///
/// L'apertura è annidata sotto l'immobile perché una segnalazione nasce **sempre**
/// su un immobile; tutto il resto va per id, perché il percorso dell'immobile non
/// aggiungerebbe nulla e obbligherebbe il client a portarselo dietro.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/properties/{property_id}/tickets", post(open))
        .route("/api/v1/tickets", get(list))
        .route("/api/v1/tickets/{id}", get(detail).patch(update_costs))
        // La correzione del testo è una rotta **a sé** e non un allargamento del
        // `PATCH` dei costi: quello avrebbe due autorizzazioni diverse secondo i
        // campi presenti nel JSON — i costi allo staff, il testo a chi ha
        // segnalato — che è il genere di cosa che si sbaglia in silenzio, perché
        // dipende da quali chiavi il corpo porta. Qui l'autorizzazione è una sola
        // per l'intera rotta. Vedi ADR-048.
        .route("/api/v1/tickets/{id}/report", patch(correct_report))
        .route(
            "/api/v1/tickets/{id}/messages",
            post(post_message).get(list_messages),
        )
        .route("/api/v1/tickets/{id}/triage", post(triage))
        .route("/api/v1/tickets/{id}/assign", post(assign))
        .route("/api/v1/tickets/{id}/start", post(start))
        .route("/api/v1/tickets/{id}/wait", post(suspend))
        .route("/api/v1/tickets/{id}/close", post(close))
        .route("/api/v1/tickets/{id}/reopen", post(reopen))
        .route("/api/v1/tickets/{id}/cancel", post(cancel))
}

// ---------------------------------------------------------------------------
// Rappresentazioni di trasporto
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TicketCategoryDto {
    Plumbing,
    Electrical,
    HeatingCooling,
    Appliance,
    Structural,
    Pest,
    Cleaning,
    Connectivity,
    Security,
    Other,
}

impl From<TicketCategoryDto> for TicketCategory {
    fn from(value: TicketCategoryDto) -> Self {
        match value {
            TicketCategoryDto::Plumbing => Self::Plumbing,
            TicketCategoryDto::Electrical => Self::Electrical,
            TicketCategoryDto::HeatingCooling => Self::HeatingCooling,
            TicketCategoryDto::Appliance => Self::Appliance,
            TicketCategoryDto::Structural => Self::Structural,
            TicketCategoryDto::Pest => Self::Pest,
            TicketCategoryDto::Cleaning => Self::Cleaning,
            TicketCategoryDto::Connectivity => Self::Connectivity,
            TicketCategoryDto::Security => Self::Security,
            TicketCategoryDto::Other => Self::Other,
        }
    }
}

impl From<TicketCategory> for TicketCategoryDto {
    fn from(value: TicketCategory) -> Self {
        match value {
            TicketCategory::Plumbing => Self::Plumbing,
            TicketCategory::Electrical => Self::Electrical,
            TicketCategory::HeatingCooling => Self::HeatingCooling,
            TicketCategory::Appliance => Self::Appliance,
            TicketCategory::Structural => Self::Structural,
            TicketCategory::Pest => Self::Pest,
            TicketCategory::Cleaning => Self::Cleaning,
            TicketCategory::Connectivity => Self::Connectivity,
            TicketCategory::Security => Self::Security,
            TicketCategory::Other => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TicketPriorityDto {
    Low,
    Medium,
    High,
    Urgent,
}

impl From<TicketPriorityDto> for TicketPriority {
    fn from(value: TicketPriorityDto) -> Self {
        match value {
            TicketPriorityDto::Low => Self::Low,
            TicketPriorityDto::Medium => Self::Medium,
            TicketPriorityDto::High => Self::High,
            TicketPriorityDto::Urgent => Self::Urgent,
        }
    }
}

impl From<TicketPriority> for TicketPriorityDto {
    fn from(value: TicketPriority) -> Self {
        match value {
            TicketPriority::Low => Self::Low,
            TicketPriority::Medium => Self::Medium,
            TicketPriority::High => Self::High,
            TicketPriority::Urgent => Self::Urgent,
        }
    }
}

/// Stato di una segnalazione sul filo.
///
/// `resolved` compare perché una riga letta può contenerlo, ma **l'API non lo
/// produce mai**: il flusso con conferma dell'inquilino è stato scartato, e la
/// riapertura ottiene lo stesso risultato senza richiedere uno scheduler che
/// chiuda le attese.
#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TicketStatusDto {
    Open,
    Triaged,
    Assigned,
    InProgress,
    WaitingTenant,
    WaitingOwner,
    /// Mai scritto dall'API.
    Resolved,
    Closed,
    Cancelled,
}

impl From<TicketStatusDto> for TicketStatus {
    fn from(value: TicketStatusDto) -> Self {
        match value {
            TicketStatusDto::Open => Self::Open,
            TicketStatusDto::Triaged => Self::Triaged,
            TicketStatusDto::Assigned => Self::Assigned,
            TicketStatusDto::InProgress => Self::InProgress,
            TicketStatusDto::WaitingTenant => Self::WaitingTenant,
            TicketStatusDto::WaitingOwner => Self::WaitingOwner,
            TicketStatusDto::Resolved => Self::Resolved,
            TicketStatusDto::Closed => Self::Closed,
            TicketStatusDto::Cancelled => Self::Cancelled,
        }
    }
}

impl From<TicketStatus> for TicketStatusDto {
    fn from(value: TicketStatus) -> Self {
        match value {
            TicketStatus::Open => Self::Open,
            TicketStatus::Triaged => Self::Triaged,
            TicketStatus::Assigned => Self::Assigned,
            TicketStatus::InProgress => Self::InProgress,
            TicketStatus::WaitingTenant => Self::WaitingTenant,
            TicketStatus::WaitingOwner => Self::WaitingOwner,
            TicketStatus::Resolved => Self::Resolved,
            TicketStatus::Closed => Self::Closed,
            TicketStatus::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TicketActorDto {
    Tenant,
    Owner,
    Manager,
    /// Riservati al triage AI, che è il modulo successivo: l'API non li produce.
    System,
    Ai,
}

impl From<TicketActor> for TicketActorDto {
    fn from(value: TicketActor) -> Self {
        match value {
            TicketActor::Tenant => Self::Tenant,
            TicketActor::Owner => Self::Owner,
            TicketActor::Manager => Self::Manager,
            TicketActor::System => Self::System,
            TicketActor::Ai => Self::Ai,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WaitingOnDto {
    Tenant,
    Owner,
}

impl From<WaitingOnDto> for WaitingOn {
    fn from(value: WaitingOnDto) -> Self {
        match value {
            WaitingOnDto::Tenant => Self::Tenant,
            WaitingOnDto::Owner => Self::Owner,
        }
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct OpenTicketRequest {
    /// La stanza, se il guasto è lì. Deve appartenere all'immobile del percorso:
    /// lo schema non lo verifica, quindi lo verifica il service layer.
    #[serde(default)]
    pub room_id: Option<Uuid>,
    /// Il contratto a cui la segnalazione si riferisce.
    ///
    /// **Da valorizzare anche quando è lo staff ad aprire per conto di qualcuno:**
    /// senza, la segnalazione non la vede nessun inquilino.
    #[serde(default)]
    pub lease_id: Option<Uuid>,
    #[schema(example = "Il boiler perde")]
    pub title: String,
    pub description: String,
    /// Assente vale `other`: classificare è il lavoro del triage.
    #[serde(default)]
    pub category: Option<TicketCategoryDto>,
    /// Assente vale `medium`.
    #[serde(default)]
    pub priority: Option<TicketPriorityDto>,
    /// Dichiara la segnalazione di interesse comune per la casa.
    ///
    /// Assente vale `false`, che è il comportamento di sempre: la vedono lo staff,
    /// chi l'ha aperta, chi ce l'ha in carico e chi è parte del contratto
    /// indicato. Con `true` la leggono anche i **residenti dell'immobile**, e con
    /// essa il thread e le fotografie — perché il confine è
    /// `app.can_access_ticket`, che serve tutte e quattro le policy. Non concede
    /// nessuna scrittura: un residente non riapre, non annulla e non corregge.
    #[serde(default)]
    pub visible_to_residents: Option<bool>,
}

impl OpenTicketRequest {
    fn into_command(self, property_id: PropertyId) -> Result<OpenTicketCommand, DomainError> {
        Ok(OpenTicketCommand {
            property_id,
            room_id: self.room_id.map(RoomId::from_uuid),
            lease_id: self.lease_id.map(LeaseId::from_uuid),
            title: TicketTitle::parse(&self.title)?,
            description: TicketDescription::parse(&self.description)?,
            category: self.category.map(Into::into),
            priority: self.priority.map(Into::into),
            visible_to_residents: self.visible_to_residents,
        })
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct TriageTicketRequest {
    pub category: TicketCategoryDto,
    pub priority: TicketPriorityDto,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AssignTicketRequest {
    pub assignee_id: Uuid,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SuspendTicketRequest {
    /// Su chi si sta aspettando. È ciò che distingue una segnalazione ferma per un
    /// motivo da una che sembra abbandonata.
    pub waiting_on: WaitingOnDto,
}

/// Modifica parziale dei costi.
///
/// **È l'unico `PATCH` del modulo.** Titolo e descrizione non si modificano — sono
/// il testo con cui una persona ha descritto un guasto, e il thread serve a
/// correggere il tiro; categoria e priorità si impostano con il triage; ogni
/// transizione ha un verbo proprio.
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
pub struct UpdateTicketCostsRequest {
    /// Stringa decimale (`"120.00"`). `null` azzera il valore.
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub estimated_cost: Patch<Decimal>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub actual_cost: Patch<Decimal>,
}

/// Correzione di ciò che chi ha segnalato aveva dichiarato.
///
/// **Non è il `PATCH` dei costi allargato**, ed è una rotta a sé apposta: qui
/// l'autorizzazione è una sola per l'intera richiesta — lo staff sempre, chi ha
/// segnalato finché la segnalazione è `open` — mentre un `PATCH` unico dovrebbe
/// decidere in base a quali chiavi il corpo porta. Vedi ADR-048.
///
/// Semantica del `PATCH`, metà: **assente = invariato**. Nessuno dei tre campi è
/// azzerabile — un titolo vuoto non è un titolo — quindi `null` non ha qui il
/// significato che ha altrove nel progetto, e mandarlo è un `422`.
///
/// **Categoria e priorità non ci sono**: sono il lavoro del triage, e lasciarle
/// modificabili qui renderebbe `triaged` una dichiarazione non verificabile.
/// **Immobile, stanza e contratto nemmeno**: non sono scrivibili dal ruolo
/// applicativo, e una segnalazione aperta sull'immobile sbagliato si annulla e si
/// riapre ([ADR-026](../../../../../docs/08-decisioni.md)).
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
pub struct CorrectTicketReportRequest {
    #[serde(default)]
    #[schema(example = "Il boiler perde")]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// La casella «visibile a chi abita qui», che si può cambiare anche dopo
    /// l'apertura.
    #[serde(default)]
    pub visible_to_residents: Option<bool>,
}

impl CorrectTicketReportRequest {
    fn into_correction(self) -> Result<TicketCorrection, DomainError> {
        Ok(TicketCorrection {
            title: self.title.as_deref().map(TicketTitle::parse).transpose()?,
            description: self
                .description
                .as_deref()
                .map(TicketDescription::parse)
                .transpose()?,
            visible_to_residents: self.visible_to_residents,
        })
    }
}

/// Corpo della chiusura.
///
/// Tutti i campi sono facoltativi, ma il corpo JSON va inviato — anche solo
/// `{}` — come per la disdetta di un contratto.
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
pub struct CloseTicketRequest {
    /// Come è stata risolta. Visibile a chiunque veda la segnalazione,
    /// l'inquilino compreso: è la risposta a «com'è andata a finire», che
    /// altrimenti andrebbe dedotta rileggendo il thread.
    ///
    /// Una riapertura la **cancella**: riaprire disfa la chiusura, e la nota fa
    /// parte della chiusura.
    #[serde(default)]
    pub resolution_notes: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PostTicketMessageRequest {
    pub body: String,
    /// Nota interna: visibile al solo staff. A un inquilino che la chiede
    /// l'API risponde `403`, non la scrive in chiaro nel thread.
    #[serde(default)]
    pub is_internal: Option<bool>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TicketResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    /// Riferimento leggibile assegnato dal database (`TK-000042`).
    pub reference: Option<String>,
    pub property_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub category: TicketCategoryDto,
    pub priority: TicketPriorityDto,
    pub status: TicketStatusDto,
    /// `true` se la leggono anche i residenti dell'immobile, oltre a chi la vede
    /// per contratto. Il confine è `app.can_access_ticket` e non questo campo:
    /// qui viaggia perché la casella si mostra e si corregge.
    pub visible_to_residents: bool,
    pub reported_by: Option<Uuid>,
    pub reported_by_kind: TicketActorDto,
    /// Come si chiama chi ha segnalato.
    ///
    /// **Assente non è un errore**: la policy `users_select` non concede a
    /// chiunque la riga di chiunque, e chi legge mette un'etichetta neutra — mai
    /// l'identificatore ([ADR-043](../../../../../docs/08-decisioni.md)). Va
    /// mostrato **accanto** a `reported_by_kind` e non al suo posto: il ruolo dice
    /// da che parte del rapporto sta la persona, il nome dice chi è.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reported_by_name: Option<String>,
    pub assigned_to: Option<Uuid>,
    /// Come si chiama chi ha in carico la lavorazione. Stessa regola del
    /// precedente sull'assenza.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_to_name: Option<String>,
    pub assigned_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Stringhe decimali, come tutti gli importi dell'API.
    ///
    /// **`estimated_cost` è `null` per chi non è staff**, anche quando un valore
    /// c'è: è la cifra di una trattativa con un fornitore e non riguarda chi
    /// abita la casa. `actual_cost` invece lo vede chiunque veda la segnalazione.
    pub estimated_cost: Option<String>,
    pub actual_cost: Option<String>,
    pub currency: String,
    pub closed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Come è stata risolta, se chi ha chiuso lo ha scritto.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl TicketResponse {
    /// `discloses_estimated_cost` arriva dal service layer, che è dove la regola
    /// è scritta: qui la si applica alla proiezione, non la si decide.
    ///
    /// `people` è il modello di lettura dei nomi, risolto dallo stesso caso d'uso
    /// che ha letto la segnalazione: **ogni** risposta ne porta uno, transizioni
    /// comprese, così i due campi del nome non ci sono a volte sì e a volte no.
    fn from_domain(ticket: &Ticket, discloses_estimated_cost: bool, people: &People) -> Self {
        Self {
            id: ticket.id().as_uuid(),
            organization_id: ticket.org_id().as_uuid(),
            reference: ticket
                .reference()
                .map(|reference| reference.as_str().to_owned()),
            property_id: ticket.property_id().as_uuid(),
            room_id: ticket.room_id().map(|id| id.as_uuid()),
            lease_id: ticket.lease_id().map(|id| id.as_uuid()),
            title: ticket.title().as_str().to_owned(),
            description: ticket.description().as_str().to_owned(),
            category: ticket.category().into(),
            priority: ticket.priority().into(),
            status: ticket.status().into(),
            visible_to_residents: ticket.visible_to_residents(),
            reported_by: ticket.reported_by().map(|id| id.as_uuid()),
            reported_by_kind: ticket.reported_by_kind().into(),
            reported_by_name: people.name_of(ticket.reported_by()).map(str::to_owned),
            assigned_to: ticket.assigned_to().map(|id| id.as_uuid()),
            assigned_to_name: people.name_of(ticket.assigned_to()).map(str::to_owned),
            assigned_at: ticket.assigned_at(),
            estimated_cost: discloses_estimated_cost
                .then(|| {
                    ticket
                        .estimated_cost()
                        .map(|money| format_amount(money.amount()))
                })
                .flatten(),
            actual_cost: ticket
                .actual_cost()
                .map(|money| format_amount(money.amount())),
            currency: ticket.currency().as_str().to_owned(),
            closed_at: ticket.closed_at(),
            resolution_notes: ticket.resolution_notes().map(str::to_owned),
            created_at: ticket.created_at(),
            updated_at: ticket.updated_at(),
        }
    }
}

/// Una transizione dello storico.
///
/// Lo storico lo scrive il trigger `tg_ticket_status_history`, non l'API: la prima
/// riga è sempre `null → open` e la produce l'`INSERT`.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TicketStatusChangeResponse {
    pub from_status: Option<TicketStatusDto>,
    pub to_status: TicketStatusDto,
    /// Chi ha compiuto la transizione. Assente se la riga è stata prodotta fuori
    /// da un contesto di richiesta.
    pub changed_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub changed_at: chrono::DateTime<chrono::Utc>,
}

impl TicketStatusChangeResponse {
    fn from_domain(change: &TicketStatusChange) -> Self {
        Self {
            from_status: change.from_status().map(Into::into),
            to_status: change.to_status().into(),
            changed_by: change.changed_by().map(|id| id.as_uuid()),
            reason: change.reason().map(str::to_owned),
            changed_at: change.changed_at(),
        }
    }
}

/// Dettaglio della segnalazione: l'aggregato più lo storico degli stati.
///
/// **Senza i messaggi.** Lo storico è corto e limitato per costruzione; una
/// conversazione non ha un tetto, e incorporarla qui significherebbe che il giorno
/// in cui qualcuno scrive duecento messaggi il dettaglio smette di rispondere. Il
/// thread sta su `GET /api/v1/tickets/{id}/messages`, con il suo cursore.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TicketDetailResponse {
    #[serde(flatten)]
    pub ticket: TicketResponse,
    pub history: Vec<TicketStatusChangeResponse>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TicketPageResponse {
    pub items: Vec<TicketResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TicketMessageResponse {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub author_id: Option<Uuid>,
    pub author_kind: TicketActorDto,
    /// Come si chiama chi ha scritto.
    ///
    /// **Il ruolo da solo non basta in un co-living**: due coinquilini sono tutti
    /// e due `tenant`, e «me lo dice il proprietario o un coinquilino?» ha già
    /// risposta mentre «quale coinquilino?» non ne ha. I due si mostrano insieme.
    /// Assente quando la policy `users_select` non concede quella riga a chi
    /// legge: allora un'etichetta neutra, mai l'identificatore.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    pub body: String,
    /// Le note interne non compaiono affatto a un inquilino: le filtra la policy
    /// `ticket_messages_select`, non questo campo.
    pub is_internal: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl TicketMessageResponse {
    fn from_domain(message: &TicketMessage, people: &People) -> Self {
        Self {
            id: message.id().as_uuid(),
            ticket_id: message.ticket_id().as_uuid(),
            author_id: message.author_id().map(|id| id.as_uuid()),
            author_kind: message.author_kind().into(),
            author_name: people.name_of(message.author_id()).map(str::to_owned),
            body: message.body().as_str().to_owned(),
            is_internal: message.is_internal(),
            created_at: message.created_at(),
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct TicketMessagePageResponse {
    pub items: Vec<TicketMessageResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListTicketsParams {
    #[serde(default)]
    pub property_id: Option<Uuid>,
    #[serde(default)]
    pub status: Option<TicketStatusDto>,
    /// `true` = solo la coda di lavorazione, cioè i sei stati non terminali.
    /// È il filtro che usa l'indice parziale `tickets_open_ix`.
    #[serde(default)]
    pub open: Option<bool>,
    #[serde(default)]
    pub after: Option<Uuid>,
    #[serde(default)]
    pub limit: Option<u16>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListMessagesParams {
    #[serde(default)]
    pub after: Option<Uuid>,
    #[serde(default)]
    pub limit: Option<u16>,
}

// ---------------------------------------------------------------------------
// Handler: deserializzare, chiamare lo use case, serializzare. Nulla di più.
// ---------------------------------------------------------------------------

#[utoipa::path(
    post, path = "/api/v1/properties/{property_id}/tickets", tag = "segnalazioni",
    params(("property_id" = Uuid, Path, description = "Identificatore dell'immobile")),
    request_body = OpenTicketRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Segnalazione aperta", body = TicketResponse),
        (status = 404, description = "Immobile, stanza o contratto non trovati", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi, o riferimenti di un altro immobile", body = crate::error::ProblemDetails),
    )
)]
async fn open(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(property_id): Path<Uuid>,
    Json(request): Json<OpenTicketRequest>,
) -> ApiResult<(StatusCode, Json<TicketResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let command = request
        .into_command(PropertyId::from_uuid(property_id))
        .map_err(|error| {
            ApiError::from(propflow_application::AppError::Validation(error))
                .with_request_id(request_id.clone())
        })?;

    let view = state
        .open_ticket
        .execute(&context, command)
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok((
        StatusCode::CREATED,
        Json(TicketResponse::from_domain(
            &view.ticket,
            discloses_estimated_cost(&context),
            &view.people,
        )),
    ))
}

#[utoipa::path(
    get, path = "/api/v1/tickets", tag = "segnalazioni",
    params(ListTicketsParams),
    security(("bearer" = [])),
    responses((status = 200, description = "Pagina di segnalazioni", body = TicketPageResponse))
)]
async fn list(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Query(params): Query<ListTicketsParams>,
) -> ApiResult<Json<TicketPageResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let discloses = discloses_estimated_cost(&context);
    let page = state
        .list_tickets
        .execute(
            &context,
            ListTicketsQuery {
                property_id: params.property_id.map(PropertyId::from_uuid),
                status: params.status.map(Into::into),
                open: params.open,
                after: params.after.map(TicketId::from_uuid),
                limit: params.limit,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(TicketPageResponse {
        items: page
            .items
            .iter()
            .map(|ticket| TicketResponse::from_domain(ticket, discloses, &page.people))
            .collect(),
        next_cursor: page.next_cursor.map(|id| id.as_uuid()),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/tickets/{id}", tag = "segnalazioni",
    params(("id" = Uuid, Path, description = "Identificatore della segnalazione")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Segnalazione con lo storico degli stati", body = TicketDetailResponse),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
    )
)]
async fn detail(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<TicketDetailResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let TicketDetail {
        ticket,
        history,
        people,
    } = state
        .get_ticket
        .execute(&context, TicketId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(TicketDetailResponse {
        ticket: TicketResponse::from_domain(&ticket, discloses_estimated_cost(&context), &people),
        history: history
            .iter()
            .map(TicketStatusChangeResponse::from_domain)
            .collect(),
    }))
}

#[utoipa::path(
    patch, path = "/api/v1/tickets/{id}", tag = "segnalazioni",
    params(("id" = Uuid, Path, description = "Identificatore della segnalazione")),
    request_body = UpdateTicketCostsRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Costi aggiornati", body = TicketResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Segnalazione annullata", body = crate::error::ProblemDetails),
        (status = 422, description = "Importo non valido", body = crate::error::ProblemDetails),
    )
)]
async fn update_costs(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateTicketCostsRequest>,
) -> ApiResult<Json<TicketResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let view = state
        .update_ticket_costs
        .execute(
            &context,
            UpdateTicketCostsCommand {
                id: TicketId::from_uuid(id),
                costs: TicketCosts {
                    estimated: request.estimated_cost,
                    actual: request.actual_cost,
                },
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(TicketResponse::from_domain(
        &view.ticket,
        discloses_estimated_cost(&context),
        &view.people,
    )))
}

#[utoipa::path(
    patch, path = "/api/v1/tickets/{id}/report", tag = "segnalazioni",
    params(("id" = Uuid, Path, description = "Identificatore della segnalazione")),
    request_body = CorrectTicketReportRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Segnalazione corretta", body = TicketResponse),
        (status = 403, description = "Corregge lo staff, o chi ha segnalato finché è aperta", body = crate::error::ProblemDetails),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Segnalazione annullata", body = crate::error::ProblemDetails),
        (status = 422, description = "Titolo o descrizione non validi", body = crate::error::ProblemDetails),
    )
)]
async fn correct_report(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<CorrectTicketReportRequest>,
) -> ApiResult<Json<TicketResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let correction = request.into_correction().map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;

    let view = state
        .correct_ticket_report
        .execute(
            &context,
            CorrectTicketReportCommand {
                id: TicketId::from_uuid(id),
                correction,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(TicketResponse::from_domain(
        &view.ticket,
        discloses_estimated_cost(&context),
        &view.people,
    )))
}

#[utoipa::path(
    post, path = "/api/v1/tickets/{id}/messages", tag = "segnalazioni",
    params(("id" = Uuid, Path, description = "Identificatore della segnalazione")),
    request_body = PostTicketMessageRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Messaggio scritto", body = TicketMessageResponse),
        (status = 403, description = "Una nota interna richiede il ruolo di staff", body = crate::error::ProblemDetails),
        (status = 404, description = "Segnalazione non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 422, description = "Corpo non valido", body = crate::error::ProblemDetails),
    )
)]
async fn post_message(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<PostTicketMessageRequest>,
) -> ApiResult<(StatusCode, Json<TicketMessageResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let body = MessageBody::parse(&request.body).map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;

    let message = state
        .post_ticket_message
        .execute(
            &context,
            PostTicketMessageCommand {
                ticket_id: TicketId::from_uuid(id),
                body,
                is_internal: request.is_internal.unwrap_or(false),
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    // Il nome di chi scrive è **il proprio**, e non serve una query per saperlo:
    // la risposta torna a chi ha appena scritto. Il campo resta comunque assente
    // quando non c'è un utente in contesto, che è il caso che il tipo ammette.
    Ok((
        StatusCode::CREATED,
        Json(TicketMessageResponse::from_domain(
            &message,
            &People::default(),
        )),
    ))
}

#[utoipa::path(
    get, path = "/api/v1/tickets/{id}/messages", tag = "segnalazioni",
    params(("id" = Uuid, Path, description = "Identificatore della segnalazione"), ListMessagesParams),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Pagina di messaggi", body = TicketMessagePageResponse),
        (status = 404, description = "Segnalazione non trovata o non visibile", body = crate::error::ProblemDetails),
    )
)]
async fn list_messages(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Query(params): Query<ListMessagesParams>,
) -> ApiResult<Json<TicketMessagePageResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let page = state
        .list_ticket_messages
        .execute(
            &context,
            ListTicketMessagesCommand {
                ticket_id: TicketId::from_uuid(id),
                after: params.after.map(TicketMessageId::from_uuid),
                limit: params.limit,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(TicketMessagePageResponse {
        items: page
            .items
            .iter()
            .map(|message| TicketMessageResponse::from_domain(message, &page.people))
            .collect(),
        next_cursor: page.next_cursor.map(|id| id.as_uuid()),
    }))
}

#[utoipa::path(
    post, path = "/api/v1/tickets/{id}/triage", tag = "segnalazioni",
    params(("id" = Uuid, Path, description = "Identificatore della segnalazione")),
    request_body = TriageTicketRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Segnalazione classificata", body = TicketResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Solo una segnalazione aperta si può classificare", body = crate::error::ProblemDetails),
    )
)]
async fn triage(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<TriageTicketRequest>,
) -> ApiResult<Json<TicketResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let view = state
        .triage_ticket
        .execute(
            &context,
            TriageTicketCommand {
                id: TicketId::from_uuid(id),
                category: request.category.into(),
                priority: request.priority.into(),
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(Json(TicketResponse::from_domain(
        &view.ticket,
        discloses_estimated_cost(&context),
        &view.people,
    )))
}

#[utoipa::path(
    post, path = "/api/v1/tickets/{id}/assign", tag = "segnalazioni",
    params(("id" = Uuid, Path, description = "Identificatore della segnalazione")),
    request_body = AssignTicketRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Segnalazione assegnata", body = TicketResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Stato non ammesso, o incaricato inesistente", body = crate::error::ProblemDetails),
    )
)]
async fn assign(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<AssignTicketRequest>,
) -> ApiResult<Json<TicketResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let view = state
        .assign_ticket
        .execute(
            &context,
            AssignTicketCommand {
                id: TicketId::from_uuid(id),
                assignee: UserId::from_uuid(request.assignee_id),
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(Json(TicketResponse::from_domain(
        &view.ticket,
        discloses_estimated_cost(&context),
        &view.people,
    )))
}

#[utoipa::path(
    post, path = "/api/v1/tickets/{id}/start", tag = "segnalazioni",
    params(("id" = Uuid, Path, description = "Identificatore della segnalazione")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Lavorazione avviata o ripresa", body = TicketResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Stato non ammesso", body = crate::error::ProblemDetails),
    )
)]
async fn start(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<TicketResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let view = state
        .start_ticket_work
        .execute(
            &context,
            StartTicketWorkCommand {
                id: TicketId::from_uuid(id),
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(Json(TicketResponse::from_domain(
        &view.ticket,
        discloses_estimated_cost(&context),
        &view.people,
    )))
}

#[utoipa::path(
    post, path = "/api/v1/tickets/{id}/wait", tag = "segnalazioni",
    params(("id" = Uuid, Path, description = "Identificatore della segnalazione")),
    request_body = SuspendTicketRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Lavorazione sospesa", body = TicketResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Si sospende solo ciò che è in lavorazione", body = crate::error::ProblemDetails),
    )
)]
async fn suspend(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<SuspendTicketRequest>,
) -> ApiResult<Json<TicketResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let view = state
        .suspend_ticket
        .execute(
            &context,
            SuspendTicketCommand {
                id: TicketId::from_uuid(id),
                waiting_on: request.waiting_on.into(),
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(Json(TicketResponse::from_domain(
        &view.ticket,
        discloses_estimated_cost(&context),
        &view.people,
    )))
}

#[utoipa::path(
    post, path = "/api/v1/tickets/{id}/close", tag = "segnalazioni",
    params(("id" = Uuid, Path, description = "Identificatore della segnalazione")),
    request_body = CloseTicketRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Segnalazione chiusa", body = TicketResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Già chiusa o annullata", body = crate::error::ProblemDetails),
    )
)]
async fn close(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<CloseTicketRequest>,
) -> ApiResult<Json<TicketResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let view = state
        .close_ticket
        .execute(
            &context,
            CloseTicketCommand {
                id: TicketId::from_uuid(id),
                resolution_notes: request.resolution_notes,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(Json(TicketResponse::from_domain(
        &view.ticket,
        discloses_estimated_cost(&context),
        &view.people,
    )))
}

#[utoipa::path(
    post, path = "/api/v1/tickets/{id}/reopen", tag = "segnalazioni",
    params(("id" = Uuid, Path, description = "Identificatore della segnalazione")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Segnalazione riaperta", body = TicketResponse),
        (status = 403, description = "Riapre chi ha segnalato, o lo staff", body = crate::error::ProblemDetails),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Solo una segnalazione chiusa si riapre", body = crate::error::ProblemDetails),
    )
)]
async fn reopen(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<TicketResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let view = state
        .reopen_ticket
        .execute(
            &context,
            ReopenTicketCommand {
                id: TicketId::from_uuid(id),
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(Json(TicketResponse::from_domain(
        &view.ticket,
        discloses_estimated_cost(&context),
        &view.people,
    )))
}

#[utoipa::path(
    post, path = "/api/v1/tickets/{id}/cancel", tag = "segnalazioni",
    params(("id" = Uuid, Path, description = "Identificatore della segnalazione")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Segnalazione annullata", body = TicketResponse),
        (status = 403, description = "Chi ha segnalato annulla solo finché nessuno ci ha messo mano", body = crate::error::ProblemDetails),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Già chiusa o annullata", body = crate::error::ProblemDetails),
    )
)]
async fn cancel(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<TicketResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let view = state
        .cancel_ticket
        .execute(
            &context,
            CancelTicketCommand {
                id: TicketId::from_uuid(id),
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(Json(TicketResponse::from_domain(
        &view.ticket,
        discloses_estimated_cost(&context),
        &view.people,
    )))
}
