use std::sync::Arc;

use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, TicketOpened};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::{LeaseId, PropertyId, RoomId, TicketId};
use propflow_domain::ticket::{
    NewTicket, Ticket, TicketActor, TicketCategory, TicketDescription, TicketPriority, TicketTitle,
};
use propflow_domain::{DomainError, Entity, RequestContext};

use super::read::{TicketView, resolve_one};
use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct OpenTicketCommand {
    pub property_id: PropertyId,
    /// La stanza, se il guasto è lì.
    pub room_id: Option<RoomId>,
    /// Il contratto a cui la segnalazione si riferisce.
    ///
    /// **Vale la pena valorizzarlo anche quando è lo staff ad aprire per conto di
    /// qualcuno**: `app.can_access_ticket` concede l'accesso a chi è parte del
    /// contratto solo se `lease_id` non è nullo, quindi una segnalazione aperta su
    /// un immobile senza contratto indicato non la vede nessun inquilino — nemmeno
    /// quello che ci abita.
    pub lease_id: Option<LeaseId>,
    pub title: TicketTitle,
    pub description: TicketDescription,
    /// Assente vale `other`: la classificazione è ciò che il triage decide, e
    /// chiederla a chi segnala un guasto produrrebbe classificazioni a caso.
    pub category: Option<TicketCategory>,
    pub priority: Option<TicketPriority>,
}

/// Apre una segnalazione.
///
/// `ensure_member` e non `ensure_staff`: è **il** punto in cui l'inquilino prende
/// la parola. Su quali immobili possa aprirla lo decide
/// `app.can_access_property` attraverso la policy `tickets_insert`, non un ramo
/// applicativo sul ruolo.
pub struct OpenTicket {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl OpenTicket {
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
        command: OpenTicketCommand,
    ) -> AppResult<TicketView> {
        let org_id = authorization::ensure_member(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;

        // Immobile, stanza e contratto si leggono **prima** di scrivere, e non è
        // prudenza: lo schema non ha alcun vincolo che leghi `room_id` e
        // `lease_id` a `property_id`. Il trigger `assert_org_matches_property`
        // verifica soltanto che l'`org_id` coincida con quello dell'immobile,
        // quindi una stanza di un altro immobile o il contratto di un altro
        // immobile passerebbero senza una parola — e il risultato è una
        // segnalazione che nei report appare sotto un immobile e nell'interfaccia
        // sotto un altro.
        //
        // Il controllo sta **qui e solo qui**: dopo l'inserimento quelle tre
        // colonne non sono più scrivibili (migration `0020`), quindi
        // l'incoerenza può nascere unicamente all'apertura.
        let property = uow
            .properties()
            .find_by_id(command.property_id)
            .await?
            .ok_or(AppError::not_found(Entity::Property))?;

        if let Some(room_id) = command.room_id {
            let room = uow
                .rooms()
                .find_by_id(room_id)
                .await?
                .ok_or(AppError::not_found(Entity::Room))?;
            if room.property_id() != property.id() {
                return Err(AppError::Validation(DomainError::validation(
                    "room_id",
                    "the room does not belong to the given property",
                )));
            }
        }

        if let Some(lease_id) = command.lease_id {
            let lease = uow
                .leases()
                .find_by_id(lease_id)
                .await?
                .ok_or(AppError::not_found(Entity::Lease))?;
            if lease.property_id() != property.id() {
                return Err(AppError::Validation(DomainError::validation(
                    "lease_id",
                    "the lease does not belong to the given property",
                )));
            }
        }

        let mut ticket = Ticket::open(NewTicket {
            id: TicketId::generate(),
            org_id,
            property_id: property.id(),
            room_id: command.room_id,
            lease_id: command.lease_id,
            title: command.title,
            description: command.description,
            category: command.category.unwrap_or(TicketCategory::Other),
            priority: command.priority.unwrap_or(TicketPriority::Medium),
            reported_by: context.user_id(),
            reported_by_kind: TicketActor::from_role(authorization::effective_role(context)),
            now,
        });

        // La `INSERT` restituisce il riferimento assegnato dal database: è l'unico
        // identificatore del progetto che non nasce nel dominio.
        let reference = uow.tickets().insert(&ticket).await?;
        ticket.attach_reference(reference);

        // Nessuna riga di storico da scrivere: `tg_ticket_status_history` scatta
        // anche in `INSERT` e ha già inserito `NULL → open`. Aggiungerne una qui
        // la raddoppierebbe — e la tabella non ha comunque una policy di `INSERT`.
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::TicketOpened(TicketOpened::from_ticket(&ticket)),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;

        let people = resolve_one(&mut *uow, &ticket).await?;
        uow.commit().await?;

        Ok(TicketView { ticket, people })
    }
}
