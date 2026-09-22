use std::sync::Arc;

use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, LeaseActivated};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::property::PropertyStatus;
use propflow_domain::shared::LeaseId;
use propflow_domain::{DomainError, Entity, Invariant, RequestContext};

use super::LeaseView;
use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy)]
pub struct ActivateLeaseCommand {
    pub id: LeaseId,
}

/// Attiva un contratto.
///
/// È l'operazione in cui il contratto passa da ipotesi a impegno, e ha effetti
/// propri: firma, stato, e l'occupazione del periodo. Per questo è un verbo e non
/// un `PATCH` sullo stato — la stessa scelta già fatta per l'archiviazione degli
/// immobili.
///
/// La sovrapposizione la rileva il vincolo di esclusione del database, non un
/// controllo qui: due attivazioni concorrenti sullo stesso periodo passerebbero
/// entrambe una verifica applicativa (ADR-015).
pub struct ActivateLease {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl ActivateLease {
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
        command: ActivateLeaseCommand,
    ) -> AppResult<LeaseView> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut lease = uow
            .leases()
            .find_by_id(command.id)
            .await?
            .ok_or(AppError::not_found(Entity::Lease))?;

        // **L'immobile prima delle parti, e l'ordine è il messaggio.** Per il
        // database non cambia niente — è tutto in una transazione — ma dire «le
        // quote non coprono il canone» su un contratto che comunque non si potrebbe
        // attivare manda a sistemare la cosa sbagliata. Un immobile in bozza non si
        // affitta: ADR-015, regola del service layer perché il legame è cross-row.
        let property = uow
            .properties()
            .find_by_id(lease.property_id())
            .await?
            .ok_or(AppError::not_found(Entity::Property))?;
        if matches!(property.status(), PropertyStatus::Draft) {
            return Err(AppError::Validation(DomainError::Invariant(
                Invariant::PropertyNotLettable,
            )));
        }

        // Le parti servono all'invariante: un contratto attivo senza inquilino, o
        // con quote che non coprono il canone, non sarebbe addebitabile.
        let parties = uow.lease_parties().list_for_lease(lease.id()).await?;
        lease.activate(&parties, now)?;

        uow.leases().update(&lease).await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::LeaseActivated(LeaseActivated {
                    lease_id: lease.id(),
                    org_id,
                    property_id: lease.property_id(),
                    activated_at: now,
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(LeaseView::new(lease, now.date_naive()))
    }
}
