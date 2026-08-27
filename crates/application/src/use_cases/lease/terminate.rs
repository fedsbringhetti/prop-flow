use std::sync::Arc;

use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, LeaseTerminated};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::LeaseId;
use propflow_domain::{Entity, RequestContext};

use super::LeaseView;
use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct TerminateLeaseCommand {
    pub id: LeaseId,
    pub reason: Option<String>,
}

/// Disdice un contratto attivo.
///
/// Diverso dalla scadenza: la scadenza è un fatto del calendario e si deriva
/// (ADR-019), la disdetta è una decisione e va registrata con la sua data.
pub struct TerminateLease {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl TerminateLease {
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
        command: TerminateLeaseCommand,
    ) -> AppResult<LeaseView> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut lease = uow
            .leases()
            .find_by_id(command.id)
            .await?
            .ok_or(AppError::not_found(Entity::Lease))?;

        lease.terminate(command.reason, now)?;

        uow.leases().update(&lease).await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::LeaseTerminated(LeaseTerminated {
                    lease_id: lease.id(),
                    org_id,
                    property_id: lease.property_id(),
                    terminated_at: now,
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(LeaseView::new(lease, now.date_naive()))
    }
}
