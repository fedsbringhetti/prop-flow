use std::sync::Arc;

use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, RoomArchived};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::RoomId;
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy)]
pub struct ArchiveRoomCommand {
    pub id: RoomId,
}

/// Archivia una stanza: soft delete.
///
/// La riga resta perché i contratti per stanza continuano a riferirsi a essa e
/// devono restare leggibili; il ruolo applicativo non ha nemmeno il privilegio
/// `DELETE`. Il nome torna disponibile per una stanza nuova, perché il vincolo di
/// unicità è parziale sul vivo.
pub struct ArchiveRoom {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl ArchiveRoom {
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
        command: ArchiveRoomCommand,
    ) -> AppResult<()> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut room = uow
            .rooms()
            .find_by_id(command.id)
            .await?
            .ok_or(AppError::not_found(Entity::Room))?;

        room.archive(now)?;
        uow.rooms().soft_delete(&room).await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::RoomArchived(RoomArchived::new(
                    room.id(),
                    room.property_id(),
                    org_id,
                    authorization::effective_role(context),
                )),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(())
    }
}
