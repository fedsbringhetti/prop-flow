use std::sync::Arc;

use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, LeaseCancelled};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::LeaseId;
use propflow_domain::{Entity, RequestContext};

use super::LeaseView;
use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct CancelLeaseCommand {
    pub id: LeaseId,
}

/// Annulla un contratto mai entrato in vigore.
///
/// # Perché annullare e non cancellare
///
/// Un contratto redatto per sbaglio non aveva via d'uscita: le rotte erano cinque
/// e nessuna lo toglieva di mezzo, quindi l'unico modo sarebbe stato attivarlo e
/// disdirlo — che è precisamente ciò che non riesce quando le quote sono
/// sbagliate. La transizione giusta però **esisteva già** nel dominio,
/// `Lease::cancel`, con il suo invariante a guardia: mancava solo di essere
/// esposta.
///
/// Non è una cancellazione soft, ed è una scelta. `deleted_at` toglierebbe la riga
/// da ogni elenco — `leases_select` filtra i cancellati — e il filtro «Annullato»
/// continuerebbe a non poter corrispondere a niente. Che un contratto sia stato
/// annullato è un fatto da leggere, non da nascondere.
///
/// # Il periodo torna libero
///
/// I due vincoli di esclusione della `0005` coprono `pending_signature`, `active`
/// ed `expiring`: un contratto annullato esce da quell'insieme, quindi il suo
/// periodo torna disponibile per un altro. È l'effetto desiderato — si annulla per
/// rifare — e vale la pena saperlo perché è invisibile leggendo solo questo file.
pub struct CancelLease {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl CancelLease {
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
        command: CancelLeaseCommand,
    ) -> AppResult<LeaseView> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut lease = uow
            .leases()
            .find_by_id(command.id)
            .await?
            .ok_or(AppError::not_found(Entity::Lease))?;

        lease.cancel(now)?;

        uow.leases().update(&lease).await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::LeaseCancelled(LeaseCancelled {
                    lease_id: lease.id(),
                    org_id,
                    property_id: lease.property_id(),
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(LeaseView::new(lease, now.date_naive()))
    }
}
