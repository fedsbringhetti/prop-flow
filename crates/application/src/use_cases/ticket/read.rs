use std::sync::Arc;

use propflow_domain::ports::{PersonName, TicketQuery, UnitOfWorkFactory};
use propflow_domain::shared::{PropertyId, TicketId, UserId};
use propflow_domain::ticket::{Ticket, TicketStatus, TicketStatusChange};
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

/// I nomi delle persone citate da una pagina.
///
/// **Un modello di lettura, non un pezzo dell'aggregato.** Il nome di una persona
/// appartiene alla persona: una segnalazione che se lo portasse dietro avrebbe due
/// fonti di verità che divergono alla prima correzione. È la stessa forma di
/// `LeaseLabels`, che la sezione 3 ha introdotto per i nomi degli immobili.
///
/// Si costruisce **una volta per pagina** e non una per riga: risolvere venticinque
/// identificatori uno alla volta sarebbe il problema `N+1` scritto a mano.
///
/// Un identificatore che non torna **non è un errore**: la policy `users_select`
/// non concede quella riga a chi guarda, e chi legge mette un'etichetta neutra —
/// mai l'UUID ([ADR-043](../../../../../docs/08-decisioni.md)).
#[derive(Debug, Clone, Default)]
pub struct People(Vec<PersonName>);

impl People {
    #[must_use]
    pub fn name_of(&self, id: Option<UserId>) -> Option<&str> {
        let id = id?;
        self.0
            .iter()
            .find(|person| person.user_id == id)
            .map(|person| person.full_name.as_str())
    }
}

/// Una segnalazione e i nomi delle persone che vi compaiono.
///
/// **Ogni caso d'uso che restituisce una segnalazione restituisce questo**, le
/// transizioni comprese, e costa una query in più su al più due identificatori.
/// L'alternativa — risolvere i nomi solo in lettura — darebbe una risposta in cui
/// i campi del nome a volte ci sono e a volte no, e un client non potrebbe
/// distinguere «non leggibile» da «non risolto»: mostrerebbe l'etichetta neutra
/// subito dopo un'azione riuscita.
#[derive(Debug)]
pub struct TicketView {
    pub ticket: Ticket,
    pub people: People,
}

/// Pagina di segnalazioni, con i nomi delle persone che vi compaiono.
#[derive(Debug)]
pub struct TicketListPage {
    pub items: Vec<Ticket>,
    pub next_cursor: Option<TicketId>,
    pub people: People,
}

#[derive(Debug, Clone, Default)]
pub struct ListTicketsQuery {
    pub property_id: Option<PropertyId>,
    pub status: Option<TicketStatus>,
    /// `Some(true)` = solo la coda di lavorazione.
    pub open: Option<bool>,
    pub after: Option<TicketId>,
    pub limit: Option<u16>,
}

/// Elenca le segnalazioni.
///
/// `ensure_member`: l'inquilino vede le proprie — quelle che ha aperto, quelle a
/// lui assegnate e quelle legate a un contratto di cui è parte. La distinzione la
/// fa `app.can_access_ticket` attraverso le policy, non un ramo applicativo sul
/// ruolo.
pub struct ListTickets {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl ListTickets {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        query: ListTicketsQuery,
    ) -> AppResult<TicketListPage> {
        authorization::ensure_member(context)?;

        // Il limite si tronca invece di essere rifiutato: un client che chiede
        // 10 000 righe ottiene la pagina massima, non un errore da gestire.
        let limit = query
            .limit
            .unwrap_or(TicketQuery::DEFAULT_LIMIT)
            .clamp(1, TicketQuery::MAX_LIMIT);

        let mut uow = self.unit_of_work.begin(context).await?;
        let page = uow
            .tickets()
            .list(&TicketQuery {
                property_id: query.property_id,
                status: query.status,
                open: query.open,
                after: query.after,
                limit,
            })
            .await?;

        let people = resolve_people(
            &mut *uow,
            page.items
                .iter()
                .flat_map(|ticket| [ticket.reported_by(), ticket.assigned_to()]),
        )
        .await?;

        Ok(TicketListPage {
            items: page.items,
            next_cursor: page.next_cursor,
            people,
        })
    }
}

/// Risolve in una query sola i nomi di tutti gli identificatori citati.
///
/// Deduplica prima di chiedere: in un thread la stessa persona scrive spesso più
/// volte, e in un elenco chi ha segnalato è spesso anche chi ha in carico.
pub(super) async fn resolve_one(
    uow: &mut dyn propflow_domain::ports::UnitOfWork,
    ticket: &Ticket,
) -> AppResult<People> {
    resolve_people(
        uow,
        [ticket.reported_by(), ticket.assigned_to()].into_iter(),
    )
    .await
}

pub(super) async fn resolve_people(
    uow: &mut dyn propflow_domain::ports::UnitOfWork,
    ids: impl Iterator<Item = Option<UserId>>,
) -> AppResult<People> {
    let mut wanted: Vec<UserId> = Vec::new();
    for id in ids.flatten() {
        if !wanted.contains(&id) {
            wanted.push(id);
        }
    }
    Ok(People(uow.users().names(&wanted).await?))
}

/// Segnalazione con il suo storico degli stati.
///
/// **Senza i messaggi**: il thread sta su una risorsa a parte, con il proprio
/// cursore. Lo storico invece è corto e limitato per costruzione — una transizione
/// per riga, e le transizioni sono poche — mentre una conversazione non ha un
/// tetto: incorporarla qui significherebbe che il giorno in cui qualcuno scrive
/// duecento messaggi il dettaglio smette di rispondere.
#[derive(Debug)]
pub struct TicketDetail {
    pub ticket: Ticket,
    pub history: Vec<TicketStatusChange>,
    /// Chi ha segnalato e chi ha in carico, per nome.
    ///
    /// **Non lo storico**: `changed_by` di ogni transizione resterebbe un
    /// identificatore in più da risolvere, e la riga dello storico dice già la cosa
    /// che serve — che cosa è cambiato e quando. Aggiungere lì i nomi è un'altra
    /// domanda, e la si fa quando qualcuno la pone.
    pub people: People,
}

/// Legge una segnalazione con lo storico delle sue transizioni.
pub struct GetTicket {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl GetTicket {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(&self, context: &RequestContext, id: TicketId) -> AppResult<TicketDetail> {
        authorization::ensure_member(context)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        let ticket = uow
            .tickets()
            .find_by_id(id)
            .await?
            .ok_or(AppError::not_found(Entity::Ticket))?;
        let history = uow.tickets().list_status_history(ticket.id()).await?;
        let people = resolve_one(&mut *uow, &ticket).await?;

        Ok(TicketDetail {
            ticket,
            history,
            people,
        })
    }
}
