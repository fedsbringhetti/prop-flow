use std::sync::Arc;

use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, LeasePartyRemoved};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::LeasePartyId;
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct RemoveLeasePartyCommand {
    pub id: LeasePartyId,
}

/// Toglie una parte inserita per sbaglio.
///
/// **Non è «l'inquilino se ne va».** Il dominio rifiuta su una parte collegata a un
/// utente, e il codice del rifiuto lo dice: quella è una persona che esce di casa,
/// con una data, conseguenze sui suoi addebiti e una domanda aperta su che cosa
/// veda dopo — cose che `left_at` rappresenta e che nessuno ha ancora progettato.
///
/// La cancellazione è **soft**, come ovunque nel progetto, e ha un effetto che
/// vale la pena sapere: gli indici unici del posto di intestatario principale e
/// del legame con l'utente sono parziali su `deleted_at IS NULL`, quindi togliere
/// una parte libera entrambi.
pub struct RemoveLeaseParty {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl RemoveLeaseParty {
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
        command: RemoveLeasePartyCommand,
    ) -> AppResult<()> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut party = uow
            .lease_parties()
            .find_by_id(command.id)
            .await?
            .ok_or(AppError::not_found(Entity::LeaseParty))?;

        party.remove(now)?;

        uow.lease_parties().soft_delete(&party).await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::LeasePartyRemoved(LeasePartyRemoved {
                    lease_id: party.lease_id(),
                    org_id,
                    party_id: party.id(),
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(())
    }
}
