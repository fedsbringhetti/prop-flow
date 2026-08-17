use std::sync::Arc;

use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, RoomUpdated};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::property::{Room, RoomChanges};
use propflow_domain::shared::RoomId;
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

use super::create::map_name_conflict;

#[derive(Debug, Clone)]
pub struct UpdateRoomCommand {
    pub id: RoomId,
    pub changes: RoomChanges,
}

/// Applica una modifica parziale a una stanza.
pub struct UpdateRoom {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl UpdateRoom {
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
        command: UpdateRoomCommand,
    ) -> AppResult<Room> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();
        let changed_fields = command.changes.changed_field_names();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut room = uow
            .rooms()
            .find_by_id(command.id)
            .await?
            .ok_or(AppError::not_found(Entity::Room))?;

        let changed = room.apply(command.changes, now)?;
        if !changed {
            // Nessun campo toccato: niente UPDATE, niente evento, niente riga di
            // audit. Un PATCH vuoto non è un errore, ma non è una modifica.
            return Ok(room);
        }

        uow.rooms()
            .update(&room)
            .await
            .map_err(|error| map_name_conflict(error.into()))?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::RoomUpdated(RoomUpdated {
                    room_id: room.id(),
                    property_id: room.property_id(),
                    org_id,
                    changed_fields,
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(room)
    }
}
