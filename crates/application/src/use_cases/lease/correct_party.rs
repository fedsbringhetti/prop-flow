use std::sync::Arc;

use propflow_domain::events::{
    DomainEvent, DomainEventPayload, EventMetadata, LeasePartyCorrected,
};
use propflow_domain::lease::{LeaseParty, LeasePartyChanges};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::LeasePartyId;
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult, Conflict};

#[derive(Debug, Clone)]
pub struct CorrectLeasePartyCommand {
    pub id: LeasePartyId,
    pub changes: LeasePartyChanges,
}

/// Corregge una parte contrattuale inserita male.
///
/// # L'operazione che mancava
///
/// Una quota sbagliata era definitiva: il contratto non si attivava, e non si
/// poteva rifare perché annullare non era esposto. In database esistono contratti
/// **attivi** con quote incoerenti, e questa è la sola via per sistemarli — per
/// questo non c'è un controllo sullo stato del contratto: rifiutare sui contratti
/// attivi lascerebbe irreparabili proprio quelli rotti.
///
/// # Chi può
///
/// Solo lo staff, come l'aggiunta. La policy `lease_parties_update` della `0014`
/// dice la stessa cosa un livello sotto, e le due si sovrappongono apposta:
/// `ensure_staff` sceglie *che cosa offrire*, la policy è il confine.
pub struct CorrectLeaseParty {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl CorrectLeaseParty {
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
        command: CorrectLeasePartyCommand,
    ) -> AppResult<LeaseParty> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut party = uow
            .lease_parties()
            .find_by_id(command.id)
            .await?
            .ok_or(AppError::not_found(Entity::LeaseParty))?;

        let changed = party.correct(command.changes, now)?;
        if !changed {
            return Ok(party);
        }

        uow.lease_parties()
            .update(&party)
            .await
            .map_err(|error| map_primary_conflict(error.into()))?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::LeasePartyCorrected(LeasePartyCorrected {
                    lease_id: party.lease_id(),
                    org_id,
                    party_id: party.id(),
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(party)
    }
}

/// Un contratto ha un solo intestatario principale, e promuoverne un secondo
/// viola `lease_parties_primary_uk`. Senza questa traduzione uscirebbe come
/// "vincolo violato" senza dire quale, che su una correzione è il momento peggiore
/// per essere vaghi.
fn map_primary_conflict(error: AppError) -> AppError {
    match &error {
        AppError::Integrity { details } if details.contains("lease_parties_primary") => {
            AppError::Conflict(Conflict::LeasePrimaryPartyExists)
        }
        _ => error,
    }
}
