use std::sync::Arc;

use chrono::NaiveDate;
use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, LeaseDrafted};
use propflow_domain::lease::{Lease, LeaseDraft as LeaseDraftInput, RentFrequency};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::{LeaseId, Money, PropertyId, RoomId};
use propflow_domain::{Entity, RequestContext};

use super::LeaseView;
use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct DraftLeaseCommand {
    pub property_id: PropertyId,
    /// `Some` per un contratto per stanza. Il tipo di contratto si deduce da qui.
    pub room_id: Option<RoomId>,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub rent: Money,
    pub deposit: Option<Money>,
    pub rent_frequency: RentFrequency,
    pub rent_due_day: i16,
    pub notice_months: Option<i16>,
    pub auto_renew: bool,
    pub renewal_months: Option<i16>,
    pub utilities_included: bool,
    pub istat_indexation: bool,
    pub notes: Option<String>,
}

/// Redige un contratto in bozza.
///
/// Una bozza non impegna l'immobile: il vincolo di non sovrapposizione si applica
/// solo agli stati `pending_signature`, `active` ed `expiring`, quindi si possono
/// preparare più ipotesi sullo stesso periodo e attivarne una.
pub struct DraftLease {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl DraftLease {
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
        command: DraftLeaseCommand,
    ) -> AppResult<LeaseView> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;

        // Immobile e stanza si leggono prima di scrivere, per la stessa ragione
        // delle stanze: un riferimento non visibile deve dare 404, non il 409 del
        // trigger di coerenza o il 403 delle policy di riga.
        let property = uow
            .properties()
            .find_by_id(command.property_id)
            .await?
            .ok_or(AppError::not_found(Entity::Property))?;

        if let Some(room_id) = command.room_id {
            let room = uow
                .rooms()
                .find_by_id(room_id)
                .await?
                .ok_or(AppError::not_found(Entity::Room))?;
            // Una stanza di un altro immobile della stessa organizzazione è
            // visibile, quindi la RLS non la fermerebbe: il legame va verificato
            // qui, altrimenti si potrebbe affittare la stanza di casa altrui.
            if room.property_id() != property.id() {
                return Err(AppError::Validation(
                    propflow_domain::DomainError::validation(
                        "room_id",
                        "the room does not belong to the given property",
                    ),
                ));
            }
        }

        let lease = Lease::draft(
            LeaseDraftInput {
                id: LeaseId::generate(),
                org_id,
                property_id: property.id(),
                room_id: command.room_id,
                start_date: command.start_date,
                end_date: command.end_date,
                rent: command.rent,
                deposit: command.deposit,
                rent_frequency: command.rent_frequency,
                rent_due_day: command.rent_due_day,
                notice_months: command.notice_months,
                auto_renew: command.auto_renew,
                renewal_months: command.renewal_months,
                utilities_included: command.utilities_included,
                istat_indexation: command.istat_indexation,
                notes: command.notes,
            },
            now,
        )?;

        uow.leases().insert(&lease).await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::LeaseDrafted(LeaseDrafted {
                    lease_id: lease.id(),
                    org_id,
                    property_id: lease.property_id(),
                    room_id: lease.room_id(),
                    lease_type: lease.lease_type().as_str(),
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(LeaseView::new(lease, now.date_naive()))
    }
}
