use std::sync::Arc;

use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, LeasePartyAdded};
use propflow_domain::lease::{LeaseParty, LeasePartyRole, NewLeaseParty};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::{Email, LeaseId, LeasePartyId};
use propflow_domain::{Entity, RequestContext};
use rust_decimal::Decimal;

use crate::authorization;
use crate::error::{AppError, AppResult, Conflict};

#[derive(Debug, Clone)]
pub struct AddLeasePartyCommand {
    pub lease_id: LeaseId,
    pub role: LeasePartyRole,
    pub full_name: String,
    /// Serve a legare l'invito alla parte: l'accettazione confronta l'email
    /// dell'utente registrato con questa.
    pub email: Option<Email>,
    pub share_percent: Decimal,
    pub is_primary: bool,
}

/// Aggiunge una parte a un contratto.
///
/// La parte nasce senza utente collegato: il proprietario redige il contratto
/// prima che l'inquilino abbia un account, e il collegamento avviene
/// all'accettazione dell'invito.
pub struct AddLeaseParty {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl AddLeaseParty {
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
        command: AddLeasePartyCommand,
    ) -> AppResult<LeaseParty> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let lease = uow
            .leases()
            .find_by_id(command.lease_id)
            .await?
            .ok_or(AppError::not_found(Entity::Lease))?;

        // Il client offriva già il bottone solo in bozza, ma quella è una scelta
        // di **che cosa offrire** e non un confine: la regola vive qui.
        lease.ensure_accepts_parties()?;

        let party = LeaseParty::new(
            NewLeaseParty {
                id: LeasePartyId::generate(),
                org_id,
                lease_id: lease.id(),
                role: command.role,
                full_name: command.full_name,
                email: command.email,
                share_percent: command.share_percent,
                is_primary: command.is_primary,
            },
            now,
        )?;

        uow.lease_parties()
            .insert(&party)
            .await
            .map_err(|error| map_primary_conflict(error.into()))?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::LeasePartyAdded(LeasePartyAdded {
                    lease_id: lease.id(),
                    org_id,
                    party_id: party.id(),
                    role: party.role().as_str(),
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(party)
    }
}

/// Un contratto ha un solo intestatario principale: il vincolo parziale
/// `lease_parties_primary_uk` lo impone. Tradurlo evita che un secondo
/// `is_primary` esca come "vincolo violato" senza dire quale.
fn map_primary_conflict(error: AppError) -> AppError {
    match &error {
        AppError::Integrity { details } if details.contains("lease_parties_primary") => {
            AppError::Conflict(Conflict::LeasePrimaryPartyExists)
        }
        _ => error,
    }
}
