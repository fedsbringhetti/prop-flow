use std::sync::Arc;

use propflow_domain::ports::{TicketPage, TicketQuery, UnitOfWorkFactory};
use propflow_domain::shared::{PropertyId, TicketId};
use propflow_domain::ticket::{Ticket, TicketStatus, TicketStatusChange};
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

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
    ) -> AppResult<TicketPage> {
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
        Ok(page)
    }
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

        Ok(TicketDetail { ticket, history })
    }
}
