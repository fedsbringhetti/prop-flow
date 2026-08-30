use std::sync::Arc;

use chrono::{DateTime, Utc};
use propflow_domain::events::{
    DomainEvent, DomainEventPayload, EventMetadata, TicketAssigned, TicketStatusChanged,
};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::{OrgId, TicketId, UserId};
use propflow_domain::ticket::{Ticket, TicketCategory, TicketPriority, TicketStatus, WaitingOn};
use propflow_domain::{Denial, DomainResult, Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

/// Le sette transizioni sono sette casi d'uso, e questa è la parte che hanno in
/// comune: autorizza → carica l'aggregato → chiama il metodo di dominio →
/// persiste → emette l'evento → commit.
///
/// Vive in una funzione e non copiata sette volte perché ciò che distingue le
/// transizioni è **l'autorizzazione e la chiamata al dominio**, non l'ordine dei
/// passi: replicare la sequenza sette volte darebbe sette occasioni di sbagliarne
/// uno, e la sequenza è precisamente ciò che non deve variare.
///
/// `TicketStatusChanged` lo emette sempre questa funzione: nessuna transizione può
/// dimenticarlo, che è l'altra metà della ragione per cui esiste.
async fn run<A, T, E>(
    unit_of_work: &Arc<dyn UnitOfWorkFactory>,
    clock: &Arc<dyn Clock>,
    context: &RequestContext,
    id: TicketId,
    authorize: A,
    apply: T,
    extra_events: E,
) -> AppResult<Ticket>
where
    A: FnOnce(&RequestContext, &Ticket) -> AppResult<()>,
    T: FnOnce(&mut Ticket, DateTime<Utc>) -> DomainResult<()>,
    E: FnOnce(&Ticket, OrgId) -> Vec<DomainEventPayload>,
{
    let org_id = authorization::ensure_member(context)?;
    let now = clock.now();

    let mut uow = unit_of_work.begin(context).await?;
    let mut ticket = uow
        .tickets()
        .find_by_id(id)
        .await?
        .ok_or(AppError::not_found(Entity::Ticket))?;

    // L'autorizzazione ha bisogno dell'aggregato — chi ha segnalato può riaprire,
    // e per saperlo bisogna aver letto la riga — quindi arriva dopo la lettura e
    // prima di qualunque scrittura.
    authorize(context, &ticket)?;

    let from_status = ticket.status();
    apply(&mut ticket, now)?;

    uow.tickets().update(&ticket).await?;

    let mut events = vec![DomainEventPayload::TicketStatusChanged(
        TicketStatusChanged::new(
            ticket.id(),
            org_id,
            from_status,
            ticket.status(),
            authorization::effective_role(context),
        ),
    )];
    events.extend(extra_events(&ticket, org_id));

    for payload in events {
        uow.events()
            .append(&DomainEvent::new(
                payload,
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
    }
    uow.commit().await?;

    Ok(ticket)
}

/// Lo staff guida la lavorazione. Vale per triage, assegnazione, avvio,
/// sospensione e chiusura.
fn staff_only(context: &RequestContext, _ticket: &Ticket) -> AppResult<()> {
    if context.is_staff() {
        return Ok(());
    }
    Err(AppError::Forbidden(Denial::StaffRoleRequired))
}

/// Riaprire tocca a chi ha segnalato, oltre che allo staff.
///
/// **Non è una scelta di prodotto rivedibile qui.** La `USING` di
/// `tickets_update` è staff **oppure** `reported_by` **oppure** `assigned_to`: non
/// passa dal contratto. Un coinquilino sullo stesso contratto *vede* la
/// segnalazione — glielo concede `can_access_ticket` attraverso `lease_id` — ma
/// non può aggiornarla, e allargare la riapertura a lui richiederebbe una
/// migration sulla policy, non un ramo in più qui.
fn reporter_or_staff(context: &RequestContext, ticket: &Ticket) -> AppResult<()> {
    if context.is_staff() || is_reporter(context, ticket) {
        return Ok(());
    }
    // `ticket.not_yours` e non `org.staff_role_required`: al coinquilino non
    // manca un ruolo — nessun ruolo gli darebbe questa operazione — gli manca
    // l'essere stato lui a segnalare. Due codici perché i due rimedi sono
    // diversi, e un client che instradasse sul primo direbbe all'utente di
    // chiedere permessi che non servono.
    Err(AppError::Forbidden(Denial::TicketNotYours))
}

/// Annullare tocca allo staff sempre, a chi ha segnalato **solo finché nessuno ci
/// ha messo mano**: un inquilino che annullasse una segnalazione già in
/// lavorazione butterebbe via il lavoro di qualcun altro.
fn canceller(context: &RequestContext, ticket: &Ticket) -> AppResult<()> {
    if context.is_staff() {
        return Ok(());
    }
    if !is_reporter(context, ticket) {
        return Err(AppError::Forbidden(Denial::TicketNotYours));
    }
    if !matches!(ticket.status(), TicketStatus::Open) {
        // Qui il primo codice è invece **vero**: la segnalazione è sua, ma
        // annullarne una già in lavorazione richiede davvero lo staff, perché
        // butterebbe via il lavoro di qualcun altro.
        return Err(AppError::Forbidden(Denial::StaffRoleRequired));
    }
    Ok(())
}

fn is_reporter(context: &RequestContext, ticket: &Ticket) -> bool {
    ticket.reported_by().is_some() && ticket.reported_by() == context.user_id()
}

/// Nessun evento oltre a `TicketStatusChanged`.
fn no_extra_events(_ticket: &Ticket, _org_id: OrgId) -> Vec<DomainEventPayload> {
    Vec::new()
}

macro_rules! transition_use_case {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        pub struct $name {
            unit_of_work: Arc<dyn UnitOfWorkFactory>,
            clock: Arc<dyn Clock>,
        }

        impl $name {
            #[must_use]
            pub const fn new(
                unit_of_work: Arc<dyn UnitOfWorkFactory>,
                clock: Arc<dyn Clock>,
            ) -> Self {
                Self { unit_of_work, clock }
            }
        }
    };
}

transition_use_case!(
    /// Classifica una segnalazione: categoria e priorità si decidono **qui** e non
    /// con un `PATCH`, perché sono ciò che `triaged` significa. Lasciarle
    /// modificabili altrove renderebbe lo stato una dichiarazione non
    /// verificabile.
    TriageTicket
);

#[derive(Debug, Clone, Copy)]
pub struct TriageTicketCommand {
    pub id: TicketId,
    pub category: TicketCategory,
    pub priority: TicketPriority,
}

impl TriageTicket {
    pub async fn execute(
        &self,
        context: &RequestContext,
        command: TriageTicketCommand,
    ) -> AppResult<Ticket> {
        run(
            &self.unit_of_work,
            &self.clock,
            context,
            command.id,
            staff_only,
            |ticket, now| ticket.triage(command.category, command.priority, now),
            no_extra_events,
        )
        .await
    }
}

transition_use_case!(
    /// Assegna la lavorazione a una persona.
    ///
    /// Emette **due** eventi: la transizione di stato e `TicketAssigned`. Il primo
    /// da solo non direbbe a chi il lavoro è finito in mano; il secondo da solo
    /// farebbe perdere l'assegnazione a chi consuma le transizioni.
    AssignTicket
);

#[derive(Debug, Clone, Copy)]
pub struct AssignTicketCommand {
    pub id: TicketId,
    pub assignee: UserId,
}

impl AssignTicket {
    pub async fn execute(
        &self,
        context: &RequestContext,
        command: AssignTicketCommand,
    ) -> AppResult<Ticket> {
        run(
            &self.unit_of_work,
            &self.clock,
            context,
            command.id,
            staff_only,
            |ticket, now| ticket.assign(command.assignee, now),
            |ticket, org_id| {
                vec![DomainEventPayload::TicketAssigned(TicketAssigned {
                    ticket_id: ticket.id(),
                    org_id,
                    assignee_id: command.assignee,
                })]
            },
        )
        .await
    }
}

transition_use_case!(
    /// Avvia la lavorazione, o la riprende dopo una sospensione: è la stessa cosa,
    /// perché `waiting_tenant` e `waiting_owner` sono pause dentro `in_progress` e
    /// non fasi a sé.
    StartTicketWork
);

#[derive(Debug, Clone, Copy)]
pub struct StartTicketWorkCommand {
    pub id: TicketId,
}

impl StartTicketWork {
    pub async fn execute(
        &self,
        context: &RequestContext,
        command: StartTicketWorkCommand,
    ) -> AppResult<Ticket> {
        run(
            &self.unit_of_work,
            &self.clock,
            context,
            command.id,
            staff_only,
            |ticket, now| ticket.start(now),
            no_extra_events,
        )
        .await
    }
}

transition_use_case!(
    /// Sospende la lavorazione dichiarando su chi si sta aspettando.
    SuspendTicket
);

#[derive(Debug, Clone, Copy)]
pub struct SuspendTicketCommand {
    pub id: TicketId,
    pub waiting_on: WaitingOn,
}

impl SuspendTicket {
    pub async fn execute(
        &self,
        context: &RequestContext,
        command: SuspendTicketCommand,
    ) -> AppResult<Ticket> {
        run(
            &self.unit_of_work,
            &self.clock,
            context,
            command.id,
            staff_only,
            |ticket, now| ticket.wait_for(command.waiting_on, now),
            no_extra_events,
        )
        .await
    }
}

transition_use_case!(
    /// Chiude la segnalazione.
    CloseTicket
);

#[derive(Debug, Clone)]
pub struct CloseTicketCommand {
    pub id: TicketId,
    /// Come è stata risolta. Facoltativo, e visibile a chiunque veda la
    /// segnalazione: senza, l'unica traccia di cosa sia stato fatto resta il
    /// thread, che è una conversazione e non una conclusione.
    pub resolution_notes: Option<String>,
}

impl CloseTicket {
    pub async fn execute(
        &self,
        context: &RequestContext,
        command: CloseTicketCommand,
    ) -> AppResult<Ticket> {
        run(
            &self.unit_of_work,
            &self.clock,
            context,
            command.id,
            staff_only,
            |ticket, now| ticket.close(command.resolution_notes, now),
            no_extra_events,
        )
        .await
    }
}

transition_use_case!(
    /// Riapre una segnalazione chiusa.
    ///
    /// È **l'unica transizione che un inquilino può eseguire**, e solo se è chi ha
    /// segnalato. È ciò che rende la segnalazione un rapporto invece di una
    /// casella postale: senza, un guasto ricorrente diventa una segnalazione nuova
    /// ogni volta e lo storico si frantuma proprio dove servirebbe intero.
    ReopenTicket
);

#[derive(Debug, Clone, Copy)]
pub struct ReopenTicketCommand {
    pub id: TicketId,
}

impl ReopenTicket {
    pub async fn execute(
        &self,
        context: &RequestContext,
        command: ReopenTicketCommand,
    ) -> AppResult<Ticket> {
        run(
            &self.unit_of_work,
            &self.clock,
            context,
            command.id,
            reporter_or_staff,
            |ticket, now| ticket.reopen(now),
            no_extra_events,
        )
        .await
    }
}

transition_use_case!(
    /// Annulla la segnalazione.
    CancelTicket
);

#[derive(Debug, Clone, Copy)]
pub struct CancelTicketCommand {
    pub id: TicketId,
}

impl CancelTicket {
    pub async fn execute(
        &self,
        context: &RequestContext,
        command: CancelTicketCommand,
    ) -> AppResult<Ticket> {
        run(
            &self.unit_of_work,
            &self.clock,
            context,
            command.id,
            canceller,
            |ticket, now| ticket.cancel(now),
            no_extra_events,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propflow_domain::identity::OrgRole;
    use propflow_domain::shared::{OrgId, PropertyId};
    use propflow_domain::ticket::{
        NewTicket, TicketActor, TicketDescription, TicketPriority, TicketTitle,
    };

    fn ticket(reporter: Option<UserId>) -> Ticket {
        Ticket::open(NewTicket {
            id: TicketId::generate(),
            org_id: OrgId::generate(),
            property_id: PropertyId::generate(),
            room_id: None,
            lease_id: None,
            title: TicketTitle::parse("Il boiler perde").expect("titolo"),
            description: TicketDescription::parse("Perde da ieri").expect("descrizione"),
            category: TicketCategory::Other,
            priority: TicketPriority::Medium,
            reported_by: reporter,
            reported_by_kind: TicketActor::Tenant,
            now: Utc::now(),
        })
    }

    fn tenant(user_id: UserId) -> RequestContext {
        RequestContext::authenticated(user_id).with_org(OrgId::generate(), OrgRole::Tenant)
    }

    fn owner() -> RequestContext {
        RequestContext::authenticated(UserId::generate())
            .with_org(OrgId::generate(), OrgRole::Owner)
    }

    #[test]
    fn only_staff_drives_the_handling() {
        let ticket = ticket(Some(UserId::generate()));
        assert!(staff_only(&owner(), &ticket).is_ok());
        assert!(staff_only(&tenant(UserId::generate()), &ticket).is_err());
    }

    #[test]
    fn the_reporter_can_reopen_and_a_stranger_cannot() {
        let reporter = UserId::generate();
        let ticket = ticket(Some(reporter));

        assert!(reporter_or_staff(&tenant(reporter), &ticket).is_ok());
        assert!(reporter_or_staff(&owner(), &ticket).is_ok());
        // Un coinquilino sullo stesso contratto vede la segnalazione ma non può
        // aggiornarla: è la `USING` di `tickets_update`, non una scelta di qui.
        let refused = reporter_or_staff(&tenant(UserId::generate()), &ticket)
            .expect_err("il coinquilino non riapre");
        assert_eq!(
            refused.code(),
            "ticket.not_yours",
            "al coinquilino non manca un ruolo, gli manca l'aver segnalato"
        );
    }

    /// Una segnalazione senza `reported_by` — l'utente è stato cancellato, e la
    /// FK è `ON DELETE SET NULL` — non deve diventare riapribile da chiunque.
    #[test]
    fn a_ticket_without_a_reporter_is_not_everyones() {
        let ticket = ticket(None);
        assert!(reporter_or_staff(&tenant(UserId::generate()), &ticket).is_err());
        assert!(canceller(&tenant(UserId::generate()), &ticket).is_err());
    }

    #[test]
    fn the_reporter_can_only_cancel_before_anyone_touched_it() {
        let reporter = UserId::generate();
        let mut ticket = ticket(Some(reporter));
        assert!(canceller(&tenant(reporter), &ticket).is_ok());

        ticket
            .assign(UserId::generate(), Utc::now())
            .expect("assegnato");
        let refused = canceller(&tenant(reporter), &ticket).expect_err(
            "annullare una segnalazione già in lavorazione butta via il lavoro di un altro",
        );
        assert_eq!(
            refused.code(),
            "org.staff_role_required",
            "qui il ruolo serve davvero: la segnalazione è sua, ma è in lavorazione"
        );
        assert!(
            canceller(&owner(), &ticket).is_ok(),
            "lo staff annulla da qualunque stato di lavorazione"
        );
    }
}
