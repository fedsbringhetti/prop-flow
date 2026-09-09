use std::sync::Arc;

use chrono::{DateTime, Utc};
use propflow_domain::events::{
    DomainEvent, DomainEventPayload, EventMetadata, TicketAssigned, TicketMessagePosted,
    TicketStatusChanged,
};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::{OrgId, TicketId, TicketMessageId, UserId};
use propflow_domain::ticket::{
    MessageBody, NewTicketMessage, Ticket, TicketActor, TicketCategory, TicketMessage,
    TicketPriority, TicketStatus, WaitingOn,
};
use propflow_domain::{Denial, DomainResult, Entity, RequestContext};

use super::read::{TicketView, resolve_one};
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
/// dimenticarlo, che è l'altra metà della ragione per cui esiste. Vale identico per
/// `TicketMessagePosted` quando la transizione lascia un messaggio nel thread:
/// scriverlo senza il suo evento è il modo in cui una copia di questa sequenza si
/// romperebbe in silenzio.
async fn run<A, T, E>(
    unit_of_work: &Arc<dyn UnitOfWorkFactory>,
    clock: &Arc<dyn Clock>,
    context: &RequestContext,
    id: TicketId,
    authorize: A,
    apply: T,
    aftermath: E,
) -> AppResult<TicketView>
where
    A: FnOnce(&RequestContext, &Ticket) -> AppResult<()>,
    T: FnOnce(&mut Ticket, DateTime<Utc>) -> DomainResult<()>,
    E: FnOnce(&Ticket, OrgId, DateTime<Utc>) -> Aftermath,
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

    let Aftermath {
        events: extra,
        message,
    } = aftermath(&ticket, org_id, now);
    events.extend(extra);

    // Il messaggio si scrive **dopo** l'aggiornamento dello stato, e non è
    // indifferente: la riapertura è l'unica transizione che ne lascia uno, e lo
    // lascia su una segnalazione che fino alla riga sopra era `closed`. Con
    // l'ordine invertito la regola «un thread terminato non riceve messaggi»
    // colpirebbe proprio il messaggio che esiste per superarla.
    if let Some(message) = message {
        uow.ticket_messages().insert(&message).await?;
        events.push(DomainEventPayload::TicketMessagePosted(
            TicketMessagePosted::from_message(&message),
        ));
    }

    for payload in events {
        uow.events()
            .append(&DomainEvent::new(
                payload,
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
    }

    // I nomi si risolvono **prima** del commit, perché dopo l'unità di lavoro non
    // c'è più. Costa una query su al più due identificatori, ed è ciò che tiene
    // uniforme la risposta: vedi `TicketView`.
    let people = resolve_one(&mut *uow, &ticket).await?;
    uow.commit().await?;

    Ok(TicketView { ticket, people })
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

/// Lo staff sempre, chi ha segnalato **solo finché nessuno ci ha messo mano**.
///
/// **Serve a due operazioni e vive in una funzione sola.** L'annullamento la usa
/// da sempre — un inquilino che annullasse una segnalazione già in lavorazione
/// butterebbe via il lavoro di qualcun altro — e la correzione del testo la usa da
/// [ADR-048](../../../../../docs/08-decisioni.md), che sceglie deliberatamente la
/// stessa forma. Copiarla darebbe due regole che divergono alla prima modifica di
/// una delle due, ed è precisamente la cosa che nessuno si accorgerebbe di aver
/// rotto.
///
/// I due codici di rifiuto **non sono intercambiabili**, e la distinzione è tutto
/// il valore di questa funzione: vedi i commenti dentro.
pub(super) fn reporter_while_open(context: &RequestContext, ticket: &Ticket) -> AppResult<()> {
    if context.is_staff() {
        return Ok(());
    }
    if !is_reporter(context, ticket) {
        // Non gli manca un ruolo — nessun ruolo gli darebbe questa operazione —
        // gli manca l'essere stato lui a segnalare.
        return Err(AppError::Forbidden(Denial::TicketNotYours));
    }
    if !matches!(ticket.status(), TicketStatus::Open) {
        // Qui il primo codice è invece **vero**: la segnalazione è sua, ma da qui
        // in poi l'operazione richiede davvero lo staff.
        return Err(AppError::Forbidden(Denial::StaffRoleRequired));
    }
    Ok(())
}

fn is_reporter(context: &RequestContext, ticket: &Ticket) -> bool {
    ticket.reported_by().is_some() && ticket.reported_by() == context.user_id()
}

/// Ciò che una transizione lascia dietro di sé oltre al cambio di stato.
///
/// Una struct e non due argomenti perché sono la stessa cosa vista da due lati —
/// che cosa è successo, e che cosa ne resta scritto — e perché `run` ha già sette
/// parametri: l'ottavo lo renderebbe una funzione che si chiama leggendo la firma.
#[derive(Default)]
struct Aftermath {
    events: Vec<DomainEventPayload>,
    /// Un messaggio da scrivere nel thread **nella stessa transazione**.
    ///
    /// Lo usa la sola riapertura, che porta il motivo. Il suo evento lo emette
    /// `run`, non chi costruisce il messaggio.
    message: Option<TicketMessage>,
}

/// Niente oltre a `TicketStatusChanged`.
fn nothing_else(_ticket: &Ticket, _org_id: OrgId, _now: DateTime<Utc>) -> Aftermath {
    Aftermath::default()
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
    ) -> AppResult<TicketView> {
        run(
            &self.unit_of_work,
            &self.clock,
            context,
            command.id,
            staff_only,
            |ticket, now| ticket.triage(command.category, command.priority, now),
            nothing_else,
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
    ) -> AppResult<TicketView> {
        run(
            &self.unit_of_work,
            &self.clock,
            context,
            command.id,
            staff_only,
            |ticket, now| ticket.assign(command.assignee, now),
            |ticket, org_id, _now| Aftermath {
                events: vec![DomainEventPayload::TicketAssigned(TicketAssigned {
                    ticket_id: ticket.id(),
                    org_id,
                    assignee_id: command.assignee,
                })],
                message: None,
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
    ) -> AppResult<TicketView> {
        run(
            &self.unit_of_work,
            &self.clock,
            context,
            command.id,
            staff_only,
            |ticket, now| ticket.start(now),
            nothing_else,
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
    ) -> AppResult<TicketView> {
        run(
            &self.unit_of_work,
            &self.clock,
            context,
            command.id,
            staff_only,
            |ticket, now| ticket.wait_for(command.waiting_on, now),
            nothing_else,
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
    ) -> AppResult<TicketView> {
        run(
            &self.unit_of_work,
            &self.clock,
            context,
            command.id,
            staff_only,
            |ticket, now| ticket.close(command.resolution_notes, now),
            nothing_else,
        )
        .await
    }
}

transition_use_case!(
    /// Riapre una segnalazione chiusa, **dicendo perché**.
    ///
    /// È **l'unica transizione che un inquilino può eseguire**, e solo se è chi ha
    /// segnalato. È ciò che rende la segnalazione un rapporto invece di una
    /// casella postale: senza, un guasto ricorrente diventa una segnalazione nuova
    /// ogni volta e lo storico si frantuma proprio dove servirebbe intero.
    ///
    /// # Il motivo è obbligatorio, e finisce nel thread
    ///
    /// Da quando un thread terminato non riceve più messaggi
    /// ([`Ticket::ensure_open_to_messages`]), riaprire è **l'unico modo** di dire
    /// «non è risolto»: il motivo smette di essere un di più ed è l'unica cosa che
    /// spiega perché la segnalazione è tornata viva. È l'asimmetria con
    /// `resolution_notes`, che resta facoltativo — la chiusura lascia comunque
    /// `closed_at` e una riga di storico.
    ///
    /// Va nel **thread** e non su una colonna: il thread non si riscrive mai,
    /// quindi il motivo sopravvive alla chiusura successiva, mentre
    /// `resolution_notes` viene cancellato dalla riapertura stessa e una colonna
    /// simmetrica lo sarebbe a sua volta. E una segnalazione si riapre più volte:
    /// tre riaperture hanno tre motivi, e una colonna ne tiene uno.
    ///
    /// Non va in `ticket_status_history.reason`, che è la colonna fatta apposta e
    /// resta non scritta: la riga la inserisce un trigger `SECURITY DEFINER`, e
    /// farle arrivare il motivo richiederebbe una `set_config` transazionale che
    /// una seconda transizione nella stessa richiesta erediterebbe. Vedi ADR-050.
    ReopenTicket
);

#[derive(Debug, Clone)]
pub struct ReopenTicketCommand {
    pub id: TicketId,
    /// Perché la segnalazione torna viva. Diventa il primo messaggio del giro
    /// nuovo, pubblico: chi ha chiuso deve poterlo leggere.
    pub reason: MessageBody,
}

impl ReopenTicket {
    pub async fn execute(
        &self,
        context: &RequestContext,
        command: ReopenTicketCommand,
    ) -> AppResult<TicketView> {
        let author_kind = TicketActor::from_role(authorization::effective_role(context));
        let author_id = context.user_id();
        let reason = command.reason;

        run(
            &self.unit_of_work,
            &self.clock,
            context,
            command.id,
            reporter_or_staff,
            |ticket, now| ticket.reopen(now),
            move |ticket, org_id, now| Aftermath {
                events: Vec::new(),
                // Mai `is_internal`: il punto del motivo è che lo legga l'altra
                // parte. Una nota interna direbbe alla segnalazione perché è
                // tornata viva senza dirlo a chi l'aveva chiusa — o a chi
                // l'aveva aperta.
                message: Some(TicketMessage::post(NewTicketMessage {
                    id: TicketMessageId::generate(),
                    org_id,
                    ticket_id: ticket.id(),
                    author_id,
                    author_kind,
                    body: reason,
                    is_internal: false,
                    now,
                })),
            },
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
    ) -> AppResult<TicketView> {
        run(
            &self.unit_of_work,
            &self.clock,
            context,
            command.id,
            reporter_while_open,
            |ticket, now| ticket.cancel(now),
            nothing_else,
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
            visible_to_residents: false,
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
        assert!(reporter_while_open(&tenant(UserId::generate()), &ticket).is_err());
    }

    #[test]
    fn the_reporter_can_only_cancel_before_anyone_touched_it() {
        let reporter = UserId::generate();
        let mut ticket = ticket(Some(reporter));
        assert!(reporter_while_open(&tenant(reporter), &ticket).is_ok());

        ticket
            .assign(UserId::generate(), Utc::now())
            .expect("assegnato");
        let refused = reporter_while_open(&tenant(reporter), &ticket).expect_err(
            "annullare una segnalazione già in lavorazione butta via il lavoro di un altro",
        );
        assert_eq!(
            refused.code(),
            "org.staff_role_required",
            "qui il ruolo serve davvero: la segnalazione è sua, ma è in lavorazione"
        );
        assert!(
            reporter_while_open(&owner(), &ticket).is_ok(),
            "lo staff annulla da qualunque stato di lavorazione"
        );
    }
}
