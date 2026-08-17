use std::sync::Arc;

use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, PropertyArchived};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::PropertyId;
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy)]
pub struct ArchivePropertyCommand {
    pub id: PropertyId,
}

/// Archivia un immobile: soft delete.
///
/// La riga non viene cancellata — il ruolo applicativo non ha nemmeno il
/// privilegio `DELETE` — perché contratti, addebiti e audit log continuano a
/// riferirsi a questo immobile e devono restare leggibili.
pub struct ArchiveProperty {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl ArchiveProperty {
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
        command: ArchivePropertyCommand,
    ) -> AppResult<()> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut property = uow
            .properties()
            .find_by_id(command.id)
            .await?
            .ok_or(AppError::not_found(Entity::Property))?;

        property.archive(now)?;
        uow.properties().soft_delete(&property).await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::PropertyArchived(PropertyArchived::new(
                    property.id(),
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
