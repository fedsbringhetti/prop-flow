use std::sync::Arc;

use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, PropertyUpdated};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::property::{Property, PropertyChanges, PropertyStatus};
use propflow_domain::shared::PropertyId;
use propflow_domain::{DomainError, Entity, Invariant, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct UpdatePropertyCommand {
    pub id: PropertyId,
    pub changes: PropertyChanges,
}

/// Applica una modifica parziale a un immobile.
pub struct UpdateProperty {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl UpdateProperty {
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
        command: UpdatePropertyCommand,
    ) -> AppResult<Property> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();
        let changed_fields = command.changes.changed_field_names();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut property = uow
            .properties()
            .find_by_id(command.id)
            .await?
            .ok_or(AppError::not_found(Entity::Property))?;

        // **Solo quando si chiede la bozza, e non a ogni `PATCH`.** Un `PATCH` che
        // non tocca lo stato, o che mette `maintenance`, non fa la domanda: la
        // 5-ter ha tenuto apposta «In manutenzione (Affittato)», che è un'anomalia
        // vera da vedere. Senza questo ramo la regola rientrerebbe dalla porta di
        // servizio — si archivia no, ma si torna in bozza sì — e «Affittato
        // (Bozza)» tornerebbe raggiungibile (ADR-015).
        if matches!(command.changes.status, Some(PropertyStatus::Draft))
            && uow
                .leases()
                .has_in_force_for_property(property.id())
                .await?
        {
            return Err(AppError::Validation(DomainError::Invariant(
                Invariant::PropertyHasLeaseInForce,
            )));
        }

        let changed = property.apply(command.changes, now)?;
        if !changed {
            // Nessun campo toccato: niente UPDATE, niente evento, niente riga di
            // audit. Un PATCH vuoto non è un errore, ma non è nemmeno una modifica.
            return Ok(property);
        }

        uow.properties().update(&property).await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::PropertyUpdated(PropertyUpdated {
                    property_id: property.id(),
                    org_id,
                    changed_fields,
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(property)
    }
}
