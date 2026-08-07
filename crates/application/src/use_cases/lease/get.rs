use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::lease::{Lease, LeaseParty, LeaseStatus};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::LeaseId;

use crate::authorization;
use crate::error::{AppError, AppResult};

/// Contratto con le sue parti e lo stato effettivo.
#[derive(Debug, Clone)]
pub struct LeaseDetail {
    pub lease: Lease,
    pub effective_status: LeaseStatus,
    pub parties: Vec<LeaseParty>,
}

/// Legge un contratto.
///
/// `ensure_member`: è qui che l'inquilino legge il proprio contratto. Quale sia il
/// proprio lo decide `app.can_access_lease`, che lo verifica attraverso
/// `lease_parties` — non serve un ramo applicativo per il ruolo, e non si
/// distingue "non esiste" da "non è tuo".
pub struct GetLease {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl GetLease {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>, clock: Arc<dyn Clock>) -> Self {
        Self {
            unit_of_work,
            clock,
        }
    }

    pub async fn execute(&self, context: &RequestContext, id: LeaseId) -> AppResult<LeaseDetail> {
        authorization::ensure_member(context)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        let lease = uow
            .leases()
            .find_by_id(id)
            .await?
            .ok_or(AppError::not_found("contratto"))?;
        let parties = uow.lease_parties().list_for_lease(lease.id()).await?;

        let today = self.clock.now().date_naive();
        Ok(LeaseDetail {
            effective_status: lease.effective_status(today),
            lease,
            parties,
        })
    }
}
