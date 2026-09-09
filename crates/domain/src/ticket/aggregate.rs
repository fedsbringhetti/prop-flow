use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use super::value_objects::{TicketDescription, TicketReference, TicketTitle};
use crate::error::{DomainError, DomainResult, Invariant};
use crate::property::Patch;
use crate::shared::{Currency, LeaseId, Money, OrgId, PropertyId, RoomId, TicketId, UserId};

/// Classificazione del guasto. Rispecchia `app.ticket_category` per intero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketCategory {
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

impl TicketCategory {
    /// Nome stabile sul filo degli eventi. Non è la mappatura verso il database:
    /// quella vive nella tabella di conversione dell'infrastruttura, e tenerle
    /// separate evita che una rinomina dell'una cambi in silenzio l'altra.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Plumbing => "plumbing",
            Self::Electrical => "electrical",
            Self::HeatingCooling => "heating_cooling",
            Self::Appliance => "appliance",
            Self::Structural => "structural",
            Self::Pest => "pest",
            Self::Cleaning => "cleaning",
            Self::Connectivity => "connectivity",
            Self::Security => "security",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl TicketPriority {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }
}

/// Chi ha compiuto l'azione: chi segnala, chi risponde, chi scrive nel thread.
///
/// `System` e `Ai` esistono nell'enum del database e **l'applicazione non li
/// scrive mai** in questa fetta: il triage AI è il modulo successivo. Restano fra
/// le varianti perché una riga scritta a mano potrebbe contenerli, e un valore
/// sconosciuto in lettura deve dare errore invece di ricadere su un default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketActor {
    Tenant,
    Owner,
    Manager,
    System,
    Ai,
}

impl TicketActor {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tenant => "tenant",
            Self::Owner => "owner",
            Self::Manager => "manager",
            Self::System => "system",
            Self::Ai => "ai",
        }
    }

    /// L'attore che corrisponde al ruolo di chi agisce nell'organizzazione.
    ///
    /// La conversione vive qui e non su `OrgRole` perché è il dominio delle
    /// segnalazioni a decidere come si chiama chi scrive: `OrgRole` non deve
    /// sapere che esistono i ticket.
    #[must_use]
    pub const fn from_role(role: crate::identity::OrgRole) -> Self {
        match role {
            crate::identity::OrgRole::Owner => Self::Owner,
            crate::identity::OrgRole::Manager => Self::Manager,
            crate::identity::OrgRole::Tenant => Self::Tenant,
        }
    }
}

/// Stato di una segnalazione.
///
/// Il ciclo di vita è `open → triaged → assigned → in_progress → closed`, con
/// `waiting_tenant` e `waiting_owner` come **sospensioni** della lavorazione — si
/// entra e si esce da `in_progress` — e la riapertura in mano a chi ha segnalato.
///
/// `Resolved` esiste nell'enum del database e **l'applicazione non lo scrive
/// mai**, come `Expiring` ed `Expired` per i contratti ([ADR-019]). Il flusso
/// `resolved` con conferma dell'inquilino è stato scartato: introdurrebbe
/// un'attesa che senza scheduler non si chiude mai da sé, e la riapertura ottiene
/// lo stesso risultato senza richiedere un orologio. Resta fra le varianti perché
/// una riga scritta a mano potrebbe contenerlo.
///
/// [ADR-019]: ../../../../docs/08-decisioni.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketStatus {
    Open,
    Triaged,
    Assigned,
    InProgress,
    WaitingTenant,
    WaitingOwner,
    /// Mai scritto dall'applicazione: vedi la nota sul tipo.
    Resolved,
    Closed,
    Cancelled,
}

impl TicketStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Triaged => "triaged",
            Self::Assigned => "assigned",
            Self::InProgress => "in_progress",
            Self::WaitingTenant => "waiting_tenant",
            Self::WaitingOwner => "waiting_owner",
            Self::Resolved => "resolved",
            Self::Closed => "closed",
            Self::Cancelled => "cancelled",
        }
    }

    /// I sei stati della coda di lavorazione.
    ///
    /// Coincide **esattamente** con il predicato dell'indice parziale
    /// `tickets_open_ix`, ed è deliberato: un elenco «tutti i ticket non chiusi»
    /// scritto come `status <> 'closed'` non userebbe l'indice. Chi filtra la coda
    /// enumera questi sei valori, e questa costante è l'unico posto dove sono
    /// elencati.
    pub const WORKING: [Self; 6] = [
        Self::Open,
        Self::Triaged,
        Self::Assigned,
        Self::InProgress,
        Self::WaitingTenant,
        Self::WaitingOwner,
    ];

    /// `true` se la segnalazione è ancora in lavorazione.
    #[must_use]
    pub const fn is_working(self) -> bool {
        matches!(
            self,
            Self::Open
                | Self::Triaged
                | Self::Assigned
                | Self::InProgress
                | Self::WaitingTenant
                | Self::WaitingOwner
        )
    }

    /// `true` se nessuna transizione parte da qui.
    ///
    /// `Resolved` è terminale perché l'applicazione non lo produce e non sa cosa
    /// farne: una riga che lo contenga si legge e non si muove.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        !self.is_working()
    }
}

/// Su chi si sta aspettando quando la lavorazione è sospesa.
///
/// Due valori e non l'intero [`TicketStatus`]: `wait_for` accetta soltanto le due
/// sospensioni, e passargli `Closed` non deve essere nemmeno esprimibile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitingOn {
    Tenant,
    Owner,
}

impl WaitingOn {
    #[must_use]
    const fn status(self) -> TicketStatus {
        match self {
            Self::Tenant => TicketStatus::WaitingTenant,
            Self::Owner => TicketStatus::WaitingOwner,
        }
    }
}

/// Dati necessari ad aprire una segnalazione.
///
/// Una struct e non nove argomenti: fra tre `Option<…Id>` di fila, scambiarne due
/// compilerebbe senza che il compilatore se ne accorga.
#[derive(Debug, Clone)]
pub struct NewTicket {
    pub id: TicketId,
    pub org_id: OrgId,
    pub property_id: PropertyId,
    /// La stanza, se il guasto è lì. Lo schema **non** verifica che appartenga
    /// all'immobile: lo fa il service layer, vedi [`Ticket::open`].
    pub room_id: Option<RoomId>,
    /// Il contratto a cui la segnalazione si riferisce.
    ///
    /// Senza, la segnalazione è **invisibile a qualunque inquilino**:
    /// `app.can_access_ticket` concede l'accesso a staff, a chi ha segnalato, a
    /// chi è assegnato, e solo poi — se `lease_id` non è nullo — a chi è parte di
    /// quel contratto.
    pub lease_id: Option<LeaseId>,
    pub title: TicketTitle,
    pub description: TicketDescription,
    pub category: TicketCategory,
    pub priority: TicketPriority,
    /// Dichiara la segnalazione di interesse comune: la leggono anche i residenti
    /// dell'immobile, e con essa il thread e le fotografie.
    ///
    /// Il default è `false`, cioè il comportamento che il modulo ha sempre avuto,
    /// e la scelta la fa chi apre. Non è un permesso applicativo: il ramo sta in
    /// `app.can_access_ticket` (migration `0035`), quindi la riga è illeggibile a
    /// un residente finché questo è falso.
    pub visible_to_residents: bool,
    pub reported_by: Option<UserId>,
    pub reported_by_kind: TicketActor,
    pub now: DateTime<Utc>,
}

/// Segnalazione di manutenzione.
///
/// **Fuori perimetro in questa fetta** (`docs/09-stato-e-roadmap.md`): il triage
/// AI (`ai_job_id`, `ai_suggested_*`, `ai_confidence`, `ai_analysis`), la ricerca
/// full-text (`search_vector`, che è `GENERATED ALWAYS` e non va mai nominata in
/// una scrittura), gli SLA (`sla_due_at`), la programmazione (`scheduled_at`) e
/// `resolution_notes`. Le colonne esistono nello schema e restano ai loro default:
/// vederle non è una ragione per riempirle.
#[derive(Debug, Clone)]
pub struct Ticket {
    id: TicketId,
    org_id: OrgId,
    property_id: PropertyId,
    room_id: Option<RoomId>,
    lease_id: Option<LeaseId>,
    /// `None` finché la riga non è stata scritta: la assegna il database.
    reference: Option<TicketReference>,
    title: TicketTitle,
    description: TicketDescription,
    category: TicketCategory,
    priority: TicketPriority,
    status: TicketStatus,
    /// Se vero, la leggono anche i residenti dell'immobile.
    ///
    /// Il confine è nel database — `app.can_access_ticket` — e questo campo è la
    /// sua faccia leggibile: serve a mostrarlo e a lasciarlo correggere, non a
    /// decidere chi vede che cosa.
    visible_to_residents: bool,
    reported_by: Option<UserId>,
    reported_by_kind: TicketActor,
    assigned_to: Option<UserId>,
    assigned_at: Option<DateTime<Utc>>,
    estimated_cost: Option<Money>,
    actual_cost: Option<Money>,
    currency: Currency,
    resolved_at: Option<DateTime<Utc>>,
    closed_at: Option<DateTime<Utc>>,
    /// Come è stata risolta, scritto da chi ha chiuso.
    ///
    /// È l'unico posto che risponde «com'è andata a finire»: il thread è una
    /// conversazione, e a distanza di mesi la conclusione andrebbe dedotta
    /// rileggendola. Viaggia in risposta a chiunque veda la segnalazione,
    /// l'inquilino compreso.
    resolution_notes: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
}

impl Ticket {
    /// Apre una segnalazione. Nasce sempre `open`.
    ///
    /// L'aggregato che esce **non ha ancora un `reference`**: lo assegna il
    /// database con il `DEFAULT` sulla colonna, e il repository lo rilegge subito
    /// dopo la scrittura per poi chiamare [`Ticket::attach_reference`]. È l'unico
    /// identificatore del progetto che non nasce qui.
    ///
    /// La riga `NULL → open` nello storico **non va scritta**: la inserisce il
    /// trigger `tg_ticket_status_history`, che scatta anche in `INSERT`.
    #[must_use]
    pub fn open(parts: NewTicket) -> Self {
        let NewTicket {
            id,
            org_id,
            property_id,
            room_id,
            lease_id,
            title,
            description,
            category,
            priority,
            visible_to_residents,
            reported_by,
            reported_by_kind,
            now,
        } = parts;

        Self {
            id,
            org_id,
            property_id,
            room_id,
            lease_id,
            reference: None,
            title,
            description,
            category,
            priority,
            status: TicketStatus::Open,
            visible_to_residents,
            reported_by,
            reported_by_kind,
            assigned_to: None,
            assigned_at: None,
            estimated_cost: None,
            actual_cost: None,
            currency: Currency::EUR,
            resolved_at: None,
            closed_at: None,
            resolution_notes: None,
            created_at: now,
            updated_at: now,
            archived_at: None,
        }
    }

    /// Registra il riferimento restituito dalla `INSERT`.
    ///
    /// Chiamata una volta sola, dal caso d'uso di apertura subito dopo la
    /// scrittura. Non è un `setter` di comodo: è la seconda metà di un
    /// costruttore che il database completa, e non esiste altro modo di ottenere
    /// un [`TicketReference`].
    pub fn attach_reference(&mut self, reference: TicketReference) {
        self.reference = Some(reference);
    }

    /// Classifica la segnalazione: qualcuno l'ha guardata e ha deciso di cosa si
    /// tratta.
    ///
    /// Categoria e priorità si impostano **qui e non con un `PATCH`**: sono ciò
    /// che `triaged` significa, e lasciarle modificabili altrove renderebbe lo
    /// stato una dichiarazione non verificabile.
    pub fn triage(
        &mut self,
        category: TicketCategory,
        priority: TicketPriority,
        now: DateTime<Utc>,
    ) -> DomainResult<()> {
        if !matches!(self.status, TicketStatus::Open) {
            return Err(DomainError::Invariant(Invariant::TicketNotTriageable));
        }
        self.category = category;
        self.priority = priority;
        self.status = TicketStatus::Triaged;
        self.updated_at = now;
        Ok(())
    }

    /// Assegna la lavorazione a una persona.
    ///
    /// Ammessa anche da `open`: una segnalazione ovvia si assegna senza passare
    /// dal triage, e obbligare a classificarla prima produrrebbe classificazioni
    /// finte. Ammessa anche da `assigned`, che è una **riassegnazione**.
    pub fn assign(&mut self, assignee: UserId, now: DateTime<Utc>) -> DomainResult<()> {
        if !matches!(
            self.status,
            TicketStatus::Open | TicketStatus::Triaged | TicketStatus::Assigned
        ) {
            return Err(DomainError::Invariant(Invariant::TicketNotAssignable));
        }
        self.assigned_to = Some(assignee);
        self.assigned_at = Some(now);
        self.status = TicketStatus::Assigned;
        self.updated_at = now;
        Ok(())
    }

    /// Avvia la lavorazione, o la riprende dopo una sospensione.
    ///
    /// Un solo verbo per le due cose perché sono la stessa: `waiting_tenant` e
    /// `waiting_owner` non sono una fase a sé ma una pausa dentro `in_progress`.
    pub fn start(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if !matches!(
            self.status,
            TicketStatus::Assigned | TicketStatus::WaitingTenant | TicketStatus::WaitingOwner
        ) {
            return Err(DomainError::Invariant(Invariant::TicketNotStartable));
        }
        self.status = TicketStatus::InProgress;
        self.updated_at = now;
        Ok(())
    }

    /// Sospende la lavorazione dichiarando su chi si sta aspettando.
    ///
    /// «Aspetto che tu sia in casa» è lo stato più comune di una manutenzione
    /// reale: senza un nome resta un ticket che sembra abbandonato.
    pub fn wait_for(&mut self, waiting_on: WaitingOn, now: DateTime<Utc>) -> DomainResult<()> {
        if !matches!(self.status, TicketStatus::InProgress) {
            return Err(DomainError::Invariant(Invariant::TicketNotSuspendable));
        }
        self.status = waiting_on.status();
        self.updated_at = now;
        Ok(())
    }

    /// Chiude la segnalazione.
    ///
    /// Ammessa da tutti e sei gli stati di lavorazione, non solo da
    /// `in_progress`: una segnalazione che si rivela un falso allarme si chiude
    /// senza inventare un avanzamento che non c'è stato. `cancelled` resta per il
    /// caso diverso in cui la richiesta viene ritirata.
    ///
    /// `closed_at` è obbligatorio anche a schema (`tickets_closed_ck`): uno stato
    /// «chiuso» senza la data del fatto non direbbe quando è finito.
    ///
    /// `resolution_notes` è facoltativo e sovrascrive quello di una chiusura
    /// precedente: la nota descrive **questa** chiusura, non tutte quelle che ci
    /// sono state.
    pub fn close(
        &mut self,
        resolution_notes: Option<String>,
        now: DateTime<Utc>,
    ) -> DomainResult<()> {
        if !self.status.is_working() {
            return Err(DomainError::Invariant(Invariant::TicketNotClosable));
        }
        self.resolution_notes =
            crate::shared::text::optional("resolution_notes", resolution_notes, 4000)?;
        self.status = TicketStatus::Closed;
        self.closed_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    /// Riapre una segnalazione chiusa: il guasto è tornato.
    ///
    /// È ciò che rende la segnalazione un rapporto invece di una casella postale.
    /// Senza, un guasto ricorrente diventa una segnalazione nuova ogni volta e lo
    /// storico si frantuma proprio dove servirebbe intero.
    ///
    /// **`closed_at` va azzerato qui, e nessuno lo fa al posto nostro.** Il
    /// vincolo `tickets_closed_ck` è `status <> 'closed' OR closed_at IS NOT NULL`:
    /// costringe una sola direzione, quindi il database accetterebbe senza dire
    /// niente una segnalazione `open` con una data di chiusura nel passato.
    pub fn reopen(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if !matches!(self.status, TicketStatus::Closed) {
            return Err(DomainError::Invariant(Invariant::TicketNotReopenable));
        }
        self.status = TicketStatus::Open;
        self.closed_at = None;
        // Lo stesso varrebbe per `resolved_at` il giorno in cui `resolved`
        // entrasse in uso: oggi l'applicazione non lo scrive mai, ma una riga
        // riletta può portarselo dietro e riaprire deve ripulire anche quello.
        self.resolved_at = None;
        // La regola è una sola e si spiega in una riga: **riaprire disfa la
        // chiusura**, e la nota di risoluzione fa parte della chiusura. Tenerla
        // lascerebbe una segnalazione `open` che dichiara come è stata risolta,
        // cioè una riga che dice il falso sul proprio stato — e la risposta la
        // vede anche l'inquilino. Se quel testo deve sopravvivere alla
        // riapertura, il posto è il thread, che non si riscrive mai.
        self.resolution_notes = None;
        self.updated_at = now;
        Ok(())
    }

    /// Annulla la segnalazione: non c'è nulla da fare, o non c'era.
    ///
    /// Il dominio verifica lo **stato**; *chi* può annullare è autorizzazione
    /// applicativa — lo staff da qualunque stato di lavorazione, chi ha segnalato
    /// solo finché nessuno ci ha messo mano.
    pub fn cancel(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if !self.status.is_working() {
            return Err(DomainError::Invariant(Invariant::TicketNotCancellable));
        }
        self.status = TicketStatus::Cancelled;
        self.updated_at = now;
        Ok(())
    }

    /// Applica una modifica parziale ai costi. `true` se qualcosa è cambiato.
    ///
    /// I costi sono l'unico attributo che il `PATCH` tocca: tutto il resto o è
    /// immutabile (titolo, descrizione, immobile) o ha un verbo proprio, perché
    /// ogni transizione ha effetti che un `PATCH` produrrebbe di soppiatto.
    ///
    /// Gli importi restano nella valuta della segnalazione, che nasce `EUR` e non
    /// è modificabile: la colonna `currency` è una sola per entrambi i costi, e
    /// due importi in valute diverse non sarebbero rappresentabili.
    pub fn apply_costs(&mut self, changes: &TicketCosts, now: DateTime<Utc>) -> DomainResult<bool> {
        if matches!(self.status, TicketStatus::Cancelled) {
            return Err(DomainError::Invariant(Invariant::TicketCancelled));
        }

        let estimated =
            self.resolve_cost("estimated_cost", changes.estimated, self.estimated_cost)?;
        let actual = self.resolve_cost("actual_cost", changes.actual, self.actual_cost)?;

        let changed = estimated != self.estimated_cost || actual != self.actual_cost;
        if !changed {
            return Ok(false);
        }

        self.estimated_cost = estimated;
        self.actual_cost = actual;
        self.updated_at = now;
        Ok(true)
    }

    fn resolve_cost(
        &self,
        field: &'static str,
        patch: Patch<Decimal>,
        current: Option<Money>,
    ) -> DomainResult<Option<Money>> {
        match patch {
            None => Ok(current),
            Some(None) => Ok(None),
            Some(Some(amount)) => {
                let money = Money::non_negative(amount, self.currency)
                    .map_err(|_| DomainError::validation(field, "must be a non-negative amount"))?;
                Ok(Some(money))
            }
        }
    }

    #[must_use]
    pub const fn id(&self) -> TicketId {
        self.id
    }
    #[must_use]
    pub const fn org_id(&self) -> OrgId {
        self.org_id
    }
    #[must_use]
    pub const fn property_id(&self) -> PropertyId {
        self.property_id
    }
    #[must_use]
    pub const fn room_id(&self) -> Option<RoomId> {
        self.room_id
    }
    #[must_use]
    pub const fn lease_id(&self) -> Option<LeaseId> {
        self.lease_id
    }
    /// `None` solo fra la costruzione e la `INSERT`: dopo la scrittura c'è sempre.
    #[must_use]
    pub const fn reference(&self) -> Option<&TicketReference> {
        self.reference.as_ref()
    }
    #[must_use]
    pub const fn title(&self) -> &TicketTitle {
        &self.title
    }
    #[must_use]
    pub const fn description(&self) -> &TicketDescription {
        &self.description
    }
    #[must_use]
    pub const fn category(&self) -> TicketCategory {
        self.category
    }
    #[must_use]
    pub const fn priority(&self) -> TicketPriority {
        self.priority
    }
    #[must_use]
    pub const fn status(&self) -> TicketStatus {
        self.status
    }
    #[must_use]
    pub const fn visible_to_residents(&self) -> bool {
        self.visible_to_residents
    }
    #[must_use]
    pub const fn reported_by(&self) -> Option<UserId> {
        self.reported_by
    }
    #[must_use]
    pub const fn reported_by_kind(&self) -> TicketActor {
        self.reported_by_kind
    }
    #[must_use]
    pub const fn assigned_to(&self) -> Option<UserId> {
        self.assigned_to
    }
    #[must_use]
    pub const fn assigned_at(&self) -> Option<DateTime<Utc>> {
        self.assigned_at
    }
    #[must_use]
    pub const fn estimated_cost(&self) -> Option<Money> {
        self.estimated_cost
    }
    #[must_use]
    pub const fn actual_cost(&self) -> Option<Money> {
        self.actual_cost
    }
    #[must_use]
    pub const fn currency(&self) -> Currency {
        self.currency
    }
    #[must_use]
    pub const fn resolved_at(&self) -> Option<DateTime<Utc>> {
        self.resolved_at
    }
    #[must_use]
    pub const fn closed_at(&self) -> Option<DateTime<Utc>> {
        self.closed_at
    }
    #[must_use]
    pub fn resolution_notes(&self) -> Option<&str> {
        self.resolution_notes.as_deref()
    }
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
    #[must_use]
    pub const fn archived_at(&self) -> Option<DateTime<Utc>> {
        self.archived_at
    }

    /// Ricostruzione da persistenza: non riesegue le invarianti, perché una
    /// segnalazione già salvata deve poter essere riletta anche se le regole sono
    /// cambiate da allora.
    #[must_use]
    pub fn from_persistence(parts: TicketParts) -> Self {
        let TicketParts {
            id,
            org_id,
            property_id,
            room_id,
            lease_id,
            reference,
            title,
            description,
            category,
            priority,
            status,
            visible_to_residents,
            reported_by,
            reported_by_kind,
            assigned_to,
            assigned_at,
            estimated_cost,
            actual_cost,
            currency,
            resolved_at,
            closed_at,
            resolution_notes,
            created_at,
            updated_at,
            archived_at,
        } = parts;
        Self {
            id,
            org_id,
            property_id,
            room_id,
            lease_id,
            reference: Some(reference),
            title,
            description,
            category,
            priority,
            status,
            visible_to_residents,
            reported_by,
            reported_by_kind,
            assigned_to,
            assigned_at,
            estimated_cost,
            actual_cost,
            currency,
            resolved_at,
            closed_at,
            resolution_notes,
            created_at,
            updated_at,
            archived_at,
        }
    }
}

/// Modifica parziale dei costi.
#[derive(Debug, Clone, Default)]
pub struct TicketCosts {
    pub estimated: Patch<Decimal>,
    pub actual: Patch<Decimal>,
}

impl TicketCosts {
    /// Nomi dei campi menzionati dal chiamante. Finiscono nell'evento al posto
    /// dei valori: chi consuma sa cosa invalidare senza che l'importo di una
    /// riparazione viaggi su un broker.
    #[must_use]
    pub fn changed_field_names(&self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        if self.estimated.is_some() {
            fields.push("estimated_cost");
        }
        if self.actual.is_some() {
            fields.push("actual_cost");
        }
        fields
    }
}

/// Dati grezzi di una segnalazione letta dalla persistenza.
#[derive(Debug, Clone)]
pub struct TicketParts {
    pub id: TicketId,
    pub org_id: OrgId,
    pub property_id: PropertyId,
    pub room_id: Option<RoomId>,
    pub lease_id: Option<LeaseId>,
    pub reference: TicketReference,
    pub title: TicketTitle,
    pub description: TicketDescription,
    pub category: TicketCategory,
    pub priority: TicketPriority,
    pub status: TicketStatus,
    pub visible_to_residents: bool,
    pub reported_by: Option<UserId>,
    pub reported_by_kind: TicketActor,
    pub assigned_to: Option<UserId>,
    pub assigned_at: Option<DateTime<Utc>>,
    pub estimated_cost: Option<Money>,
    pub actual_cost: Option<Money>,
    pub currency: Currency,
    pub resolved_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub resolution_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn ticket() -> Ticket {
        Ticket::open(NewTicket {
            id: TicketId::generate(),
            org_id: OrgId::generate(),
            property_id: PropertyId::generate(),
            room_id: None,
            lease_id: Some(LeaseId::generate()),
            title: TicketTitle::parse("Il boiler perde").expect("titolo valido"),
            description: TicketDescription::parse("Perde da ieri sera").expect("valida"),
            category: TicketCategory::Other,
            priority: TicketPriority::Medium,
            visible_to_residents: false,
            reported_by: Some(UserId::generate()),
            reported_by_kind: TicketActor::Tenant,
            now: Utc::now(),
        })
    }

    fn dec(raw: &str) -> Decimal {
        Decimal::from_str(raw).expect("decimale di test valido")
    }

    /// Porta la segnalazione fino allo stato richiesto percorrendo il flusso
    /// nominale. Fallisce se una transizione del percorso non è ammessa, il che è
    /// esattamente ciò che si vuole sapere.
    fn at(status: TicketStatus) -> Ticket {
        let now = Utc::now();
        let mut ticket = ticket();
        let steps: &[&dyn Fn(&mut Ticket)] = match status {
            TicketStatus::Open => &[],
            TicketStatus::Triaged => &[&|t: &mut Ticket| {
                t.triage(TicketCategory::Plumbing, TicketPriority::High, Utc::now())
                    .expect("triage");
            }],
            TicketStatus::Assigned => &[&|t: &mut Ticket| {
                t.assign(UserId::generate(), Utc::now()).expect("assegnato");
            }],
            TicketStatus::InProgress => &[
                &|t: &mut Ticket| {
                    t.assign(UserId::generate(), Utc::now()).expect("assegnato");
                },
                &|t: &mut Ticket| t.start(Utc::now()).expect("avviato"),
            ],
            TicketStatus::WaitingTenant | TicketStatus::WaitingOwner => &[
                &|t: &mut Ticket| {
                    t.assign(UserId::generate(), Utc::now()).expect("assegnato");
                },
                &|t: &mut Ticket| t.start(Utc::now()).expect("avviato"),
            ],
            TicketStatus::Closed => &[&|t: &mut Ticket| t.close(None, Utc::now()).expect("chiuso")],
            TicketStatus::Cancelled => {
                &[&|t: &mut Ticket| t.cancel(Utc::now()).expect("annullato")]
            }
            // Mai prodotto dall'applicazione: si costruisce solo da persistenza,
            // e i test che lo riguardano lo fanno esplicitamente.
            TicketStatus::Resolved => &[],
        };
        for step in steps {
            step(&mut ticket);
        }
        if matches!(status, TicketStatus::WaitingTenant) {
            ticket.wait_for(WaitingOn::Tenant, now).expect("sospeso");
        }
        if matches!(status, TicketStatus::WaitingOwner) {
            ticket.wait_for(WaitingOn::Owner, now).expect("sospeso");
        }
        ticket
    }

    #[test]
    fn a_new_ticket_is_open_without_a_reference() {
        let ticket = ticket();
        assert_eq!(ticket.status(), TicketStatus::Open);
        assert_eq!(ticket.reference(), None, "la assegna il database");
        assert_eq!(ticket.assigned_to(), None);
        assert_eq!(ticket.closed_at(), None);
    }

    #[test]
    fn the_reference_arrives_from_persistence_and_only_from_there() {
        let mut ticket = ticket();
        ticket.attach_reference(TicketReference::from_persistence("TK-000007".to_owned()));
        assert_eq!(
            ticket.reference().map(TicketReference::as_str),
            Some("TK-000007")
        );
    }

    /// La tabella delle transizioni, ammesse e rifiutate, verificata per intero.
    ///
    /// È il punto in cui una macchina a stati si verifica senza database, e vale
    /// la sua lunghezza: ogni riga assente qui è una transizione su cui nessuno
    /// si accorgerebbe di una regressione.
    #[test]
    fn the_transition_table_is_complete() {
        use TicketStatus::{
            Assigned, Cancelled, Closed, InProgress, Open, Triaged, WaitingOwner, WaitingTenant,
        };

        // triage: solo da `open`.
        for from in [Open, Triaged, Assigned, InProgress, Closed, Cancelled] {
            let allowed = matches!(from, Open);
            let ok = at(from)
                .triage(TicketCategory::Pest, TicketPriority::Low, Utc::now())
                .is_ok();
            assert_eq!(ok, allowed, "triage da {from:?}");
        }

        // assign: da `open`, `triaged` e `assigned` (riassegnazione).
        for from in [Open, Triaged, Assigned, InProgress, Closed, Cancelled] {
            let allowed = matches!(from, Open | Triaged | Assigned);
            let ok = at(from).assign(UserId::generate(), Utc::now()).is_ok();
            assert_eq!(ok, allowed, "assign da {from:?}");
        }

        // start: da `assigned` e dalle due sospensioni.
        for from in [
            Open,
            Triaged,
            Assigned,
            InProgress,
            WaitingTenant,
            WaitingOwner,
            Closed,
            Cancelled,
        ] {
            let allowed = matches!(from, Assigned | WaitingTenant | WaitingOwner);
            assert_eq!(
                at(from).start(Utc::now()).is_ok(),
                allowed,
                "start da {from:?}"
            );
        }

        // wait_for: solo da `in_progress`.
        for from in [
            Open,
            Triaged,
            Assigned,
            InProgress,
            WaitingTenant,
            Closed,
            Cancelled,
        ] {
            let allowed = matches!(from, InProgress);
            let ok = at(from).wait_for(WaitingOn::Tenant, Utc::now()).is_ok();
            assert_eq!(ok, allowed, "wait_for da {from:?}");
        }

        // close e cancel: da tutti e sei gli stati di lavorazione, e da nessun
        // altro. È la stessa condizione, ed è deliberato.
        for from in [
            Open,
            Triaged,
            Assigned,
            InProgress,
            WaitingTenant,
            WaitingOwner,
            Closed,
            Cancelled,
        ] {
            assert_eq!(
                at(from).close(None, Utc::now()).is_ok(),
                from.is_working(),
                "close da {from:?}"
            );
            assert_eq!(
                at(from).cancel(Utc::now()).is_ok(),
                from.is_working(),
                "cancel da {from:?}"
            );
        }

        // reopen: solo da `closed`.
        for from in [
            Open,
            Triaged,
            Assigned,
            InProgress,
            WaitingTenant,
            WaitingOwner,
            Closed,
            Cancelled,
        ] {
            let allowed = matches!(from, Closed);
            assert_eq!(
                at(from).reopen(Utc::now()).is_ok(),
                allowed,
                "reopen da {from:?}"
            );
        }
    }

    #[test]
    fn triage_is_where_category_and_priority_are_decided() {
        let mut ticket = ticket();
        assert_eq!(ticket.category(), TicketCategory::Other);
        ticket
            .triage(TicketCategory::Plumbing, TicketPriority::Urgent, Utc::now())
            .expect("triage");
        assert_eq!(ticket.category(), TicketCategory::Plumbing);
        assert_eq!(ticket.priority(), TicketPriority::Urgent);
        assert_eq!(ticket.status(), TicketStatus::Triaged);
    }

    #[test]
    fn assigning_records_who_and_when() {
        let assignee = UserId::generate();
        let now = Utc::now();
        let mut ticket = ticket();
        ticket.assign(assignee, now).expect("assegnato");
        assert_eq!(ticket.assigned_to(), Some(assignee));
        assert_eq!(ticket.assigned_at(), Some(now));
    }

    #[test]
    fn a_suspension_is_a_pause_inside_the_work_and_not_a_phase() {
        let mut ticket = at(TicketStatus::InProgress);
        ticket
            .wait_for(WaitingOn::Tenant, Utc::now())
            .expect("sospeso");
        assert_eq!(ticket.status(), TicketStatus::WaitingTenant);
        ticket.start(Utc::now()).expect("ripreso");
        assert_eq!(ticket.status(), TicketStatus::InProgress);
    }

    /// La trappola: `tickets_closed_ck` costringe una sola direzione, quindi il
    /// database accetterebbe senza dire niente una segnalazione `open` con una
    /// data di chiusura nel passato. Deve azzerarla il dominio.
    #[test]
    fn reopening_clears_the_closing_date() {
        let now = Utc::now();
        let mut ticket = ticket();
        ticket.close(None, now).expect("chiuso");
        assert_eq!(ticket.closed_at(), Some(now));

        ticket.reopen(Utc::now()).expect("riaperto");
        assert_eq!(ticket.status(), TicketStatus::Open);
        assert_eq!(
            ticket.closed_at(),
            None,
            "una segnalazione riaperta con una data di chiusura è una riga che mente"
        );
    }

    /// Chiudere registra come è stata risolta; riaprire disfa la chiusura, e la
    /// nota fa parte della chiusura. Una segnalazione `open` che dichiara come è
    /// stata risolta direbbe il falso sul proprio stato, e la risposta la vede
    /// anche chi ha segnalato.
    #[test]
    fn closing_records_the_resolution_and_reopening_takes_it_away() {
        let mut ticket = ticket();
        ticket
            .close(Some("  sostituita la valvola  ".to_owned()), Utc::now())
            .expect("chiusa");
        assert_eq!(ticket.resolution_notes(), Some("sostituita la valvola"));

        ticket.reopen(Utc::now()).expect("riaperta");
        assert_eq!(ticket.resolution_notes(), None);
        assert_eq!(ticket.closed_at(), None);

        // Una chiusura successiva descrive sé stessa, non la somma delle
        // precedenti.
        ticket
            .close(Some("rifatta la guarnizione".to_owned()), Utc::now())
            .expect("richiusa");
        assert_eq!(ticket.resolution_notes(), Some("rifatta la guarnizione"));
    }

    /// Una nota vuota o di soli spazi non è una nota: diventa `None`, così il
    /// database non accumula la distinzione priva di senso fra `''` e `NULL`.
    #[test]
    fn a_blank_resolution_note_is_no_note() {
        let mut ticket = ticket();
        ticket
            .close(Some("   ".to_owned()), Utc::now())
            .expect("chiusa");
        assert_eq!(ticket.resolution_notes(), None);
    }

    #[test]
    fn a_reopened_ticket_can_run_the_whole_cycle_again() {
        let mut ticket = ticket();
        ticket.close(None, Utc::now()).expect("chiuso");
        ticket.reopen(Utc::now()).expect("riaperto");
        ticket
            .triage(
                TicketCategory::HeatingCooling,
                TicketPriority::High,
                Utc::now(),
            )
            .expect("classificabile di nuovo");
    }

    #[test]
    fn costs_are_a_partial_update_that_can_clear_a_value() {
        let mut ticket = ticket();
        let changed = ticket
            .apply_costs(
                &TicketCosts {
                    estimated: Some(Some(dec("120.00"))),
                    actual: None,
                },
                Utc::now(),
            )
            .expect("valido");
        assert!(changed);
        assert_eq!(
            ticket.estimated_cost().map(|money| money.amount()),
            Some(dec("120.00"))
        );

        // Campo non menzionato: resta com'era. Campo a `null`: azzerato.
        let changed = ticket
            .apply_costs(
                &TicketCosts {
                    estimated: Some(None),
                    actual: Some(Some(dec("95.50"))),
                },
                Utc::now(),
            )
            .expect("valido");
        assert!(changed);
        assert_eq!(ticket.estimated_cost(), None);
        assert_eq!(
            ticket.actual_cost().map(|money| money.amount()),
            Some(dec("95.50"))
        );
    }

    /// Un PATCH che non cambia nulla non deve produrre scrittura, evento e riga
    /// di audit: la stessa proprietà di `Property::apply`.
    #[test]
    fn an_empty_cost_patch_changes_nothing() {
        let mut ticket = ticket();
        let changed = ticket
            .apply_costs(&TicketCosts::default(), Utc::now())
            .expect("valido");
        assert!(!changed);
        assert!(TicketCosts::default().changed_field_names().is_empty());
    }

    #[test]
    fn costs_cannot_be_negative_and_cannot_touch_a_cancelled_ticket() {
        let mut ticket = ticket();
        let error = ticket
            .apply_costs(
                &TicketCosts {
                    estimated: Some(Some(dec("-1.00"))),
                    actual: None,
                },
                Utc::now(),
            )
            .expect_err("un costo negativo non ha significato");
        assert!(matches!(error, DomainError::Validation { .. }));

        let mut cancelled = at(TicketStatus::Cancelled);
        let error = cancelled
            .apply_costs(
                &TicketCosts {
                    actual: Some(Some(dec("10.00"))),
                    estimated: None,
                },
                Utc::now(),
            )
            .expect_err("una segnalazione annullata non ha costi");
        assert!(matches!(
            error,
            DomainError::Invariant(Invariant::TicketCancelled)
        ));
    }

    /// La costante deve restare allineata al predicato di `tickets_open_ix`: se
    /// qualcuno aggiungesse uno stato di lavorazione senza toccare l'indice, la
    /// coda smetterebbe di usarlo senza che nulla fallisca.
    #[test]
    fn the_working_set_and_the_predicate_agree() {
        for status in TicketStatus::WORKING {
            assert!(status.is_working(), "{status:?}");
            assert!(!status.is_terminal(), "{status:?}");
        }
        for status in [
            TicketStatus::Resolved,
            TicketStatus::Closed,
            TicketStatus::Cancelled,
        ] {
            assert!(status.is_terminal(), "{status:?}");
        }
    }

    /// `resolved` è un valore che il codice legge e non scrive mai. Il test fissa
    /// che nessuna transizione lo produca: se qualcuno lo aggiungesse, la coppia
    /// `resolved`/`closed` tornerebbe a significare due cose diverse senza che
    /// nulla nel flusso lo dica.
    #[test]
    fn no_transition_ever_produces_resolved() {
        let now = Utc::now();
        for status in TicketStatus::WORKING {
            let mut ticket = at(status);
            let _ = ticket.triage(TicketCategory::Other, TicketPriority::Low, now);
            let _ = ticket.assign(UserId::generate(), now);
            let _ = ticket.start(now);
            let _ = ticket.wait_for(WaitingOn::Owner, now);
            let _ = ticket.close(None, now);
            let _ = ticket.reopen(now);
            let _ = ticket.cancel(now);
            assert_ne!(ticket.status(), TicketStatus::Resolved);
        }
    }

    #[test]
    fn the_actor_follows_the_role_of_who_acts() {
        use crate::identity::OrgRole;
        assert_eq!(TicketActor::from_role(OrgRole::Owner), TicketActor::Owner);
        assert_eq!(
            TicketActor::from_role(OrgRole::Manager),
            TicketActor::Manager
        );
        assert_eq!(TicketActor::from_role(OrgRole::Tenant), TicketActor::Tenant);
    }
}
