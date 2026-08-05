use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, PropertyCreated};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::property::{Property, PropertyType};
use propflow_domain::shared::{Address, PropertyId};

use crate::authorization;
use crate::error::AppResult;

#[derive(Debug, Clone)]
pub struct CreatePropertyCommand {
    pub label: String,
    pub property_type: PropertyType,
    pub address: Address,
}

/// Crea un immobile nell'organizzazione corrente.
pub struct CreateProperty {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl CreateProperty {
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
        command: CreatePropertyCommand,
    ) -> AppResult<Property> {
        let org_id = authorization::ensure_staff(context)?;
        let owner_id = context.require_user()?;
        let now = self.clock.now();

        let property = Property::create(
            PropertyId::generate(),
            org_id,
            owner_id,
            &command.label,
            command.property_type,
            command.address,
            now,
        )?;

        let mut uow = self.unit_of_work.begin(context).await?;
        uow.properties().insert(&property).await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::PropertyCreated(PropertyCreated {
                    property_id: property.id(),
                    org_id,
                    owner_id,
                    city: property.address().city().to_owned(),
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        // L'evento e la riga entrano nella stessa transazione: se il commit
        // fallisce non resta né l'immobile né l'annuncio della sua creazione.
        uow.commit().await?;

        Ok(property)
    }
}
