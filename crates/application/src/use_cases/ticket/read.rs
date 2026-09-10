use std::sync::Arc;

use propflow_domain::ports::{PersonName, TicketLabels, TicketQuery, UnitOfWorkFactory};
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

/// I nomi di **dove** stanno le segnalazioni di una pagina.
///
/// Terza occorrenza dello stesso modello di lettura — `LeaseLabels` sui contratti
/// (sezione 3), `People` sulle persone ([ADR-049](../../../../../docs/08-decisioni.md)),
/// e adesso i luoghi — e la regolarità dice qualcosa: **ogni risposta che porta un
/// identificatore verso una schermata è un difetto che aspetta di essere trovato**.
///
/// Si costruisce una volta per pagina, con un `LEFT JOIN`: un immobile archiviato
/// lascia il campo assente invece di far sparire la riga.
#[derive(Debug, Clone, Default)]
pub struct Places(Vec<TicketLabels>);

impl Places {
    #[must_use]
    pub fn of(&self, ticket_id: TicketId) -> Option<&TicketLabels> {
        self.0.iter().find(|labels| labels.ticket_id == ticket_id)
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
    /// Dove sta la segnalazione, per nome.
    ///
    /// Viaggia insieme ai nomi delle persone e per la stessa ragione: se le
    /// transizioni non li risolvessero, la stessa risposta porterebbe i campi a
    /// volte sì e a volte no, e un client mostrerebbe «Immobile archiviato» subito
    /// dopo un'azione riuscita.
    pub places: Places,
}

/// Pagina di segnalazioni, con i nomi delle persone che vi compaiono.
#[derive(Debug)]
pub struct TicketListPage {
    pub items: Vec<Ticket>,
    pub next_cursor: Option<TicketId>,
    pub people: People,
    pub places: Places,
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

        let places = resolve_places(&mut *uow, &page.items).await?;

        Ok(TicketListPage {
            items: page.items,
            next_cursor: page.next_cursor,
            people,
            places,
        })
    }
}

/// La risposta di **ogni** caso d'uso che restituisce una segnalazione.
///
/// Esiste per essere l'unico punto in cui `TicketView` si costruisce: i due modelli
/// di lettura — le persone e i luoghi — vanno risolti *tutti e due* e **prima del
/// commit**, perché dopo l'unità di lavoro non c'è più. Con sette transizioni, un
/// `open`, due `patch` e due letture, la forma «risolvi e poi costruisci» ripetuta
/// dodici volte è la forma in cui la tredicesima ne dimentica uno — e il sintomo
/// sarebbe un'etichetta neutra subito dopo un'azione riuscita, cioè indistinguibile
/// da «non leggibile».
pub(super) async fn view_of(
    uow: &mut dyn propflow_domain::ports::UnitOfWork,
    ticket: Ticket,
) -> AppResult<TicketView> {
    let people = resolve_one(uow, &ticket).await?;
    let places = resolve_places(uow, std::slice::from_ref(&ticket)).await?;
    Ok(TicketView {
        ticket,
        people,
        places,
    })
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

/// I luoghi di una pagina di segnalazioni, in una query sola.
pub(super) async fn resolve_places(
    uow: &mut dyn propflow_domain::ports::UnitOfWork,
    tickets: &[Ticket],
) -> AppResult<Places> {
    let ids: Vec<TicketId> = tickets.iter().map(Ticket::id).collect();
    Ok(Places(uow.tickets().labels(&ids).await?))
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
    /// Dove sta la segnalazione, per nome. Stessa regola dell'assenza.
    pub places: Places,
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
        let places = resolve_places(&mut *uow, std::slice::from_ref(&ticket)).await?;

        Ok(TicketDetail {
            ticket,
            history,
            people,
            places,
        })
    }
}
