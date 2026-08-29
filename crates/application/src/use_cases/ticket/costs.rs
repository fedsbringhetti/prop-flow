use std::sync::Arc;

use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, TicketCostsUpdated};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::TicketId;
use propflow_domain::ticket::{Ticket, TicketCosts};
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct UpdateTicketCostsCommand {
    pub id: TicketId,
    pub costs: TicketCosts,
}

/// Aggiorna i costi stimati e sostenuti di una segnalazione.
///
/// **È l'unico `PATCH` del modulo, e tocca solo i costi.** Titolo e descrizione
/// sono il testo con cui una persona ha descritto un guasto in un momento preciso
/// e non si modificano — il thread esiste apposta per correggere il tiro;
/// categoria e priorità si impostano con il triage, perché sono ciò che
/// `triaged` significa; ogni transizione ha un verbo proprio, perché ha effetti
/// — `closed_at`, una riga di storico, un evento — che un `PATCH` produrrebbe di
/// soppiatto.
///
/// `ensure_staff`: il costo di una riparazione lo registra chi la paga.
pub struct UpdateTicketCosts {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl UpdateTicketCosts {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>, clock: Arc<dyn Clock>) -> Self {
        Self {
            unit_of_work,
            clock,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        command: UpdateTicketCostsCommand,
    ) -> AppResult<Ticket> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();
        let changed_fields = command.costs.changed_field_names();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut ticket = uow
            .tickets()
            .find_by_id(command.id)
            .await?
            .ok_or(AppError::not_found(Entity::Ticket))?;

        let changed = ticket.apply_costs(&command.costs, now)?;
        if !changed {
            // Nessun campo toccato: niente UPDATE, niente evento, niente riga di
            // audit. Un PATCH vuoto non è un errore, ma non è nemmeno una
            // modifica.
            return Ok(ticket);
        }

        uow.tickets().update(&ticket).await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::TicketCostsUpdated(TicketCostsUpdated {
                    ticket_id: ticket.id(),
                    org_id,
                    changed_fields,
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(ticket)
    }
}
