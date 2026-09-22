use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, PropertyCreated};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::property::{Property, PropertyChanges, PropertyStatus, PropertyType};
use propflow_domain::shared::{Address, PropertyId};

use crate::authorization;
use crate::error::AppResult;

#[derive(Debug, Clone)]
pub struct CreatePropertyCommand {
    pub label: String,
    pub property_type: PropertyType,
    pub address: Address,
    /// Lo stato iniziale, quando chi crea lo sceglie.
    ///
    /// `None` lascia il default del dominio, che resta `draft`: il form lo offre
    /// (sezione 12), ma il default non si sposta nel dominio — una creazione fatta
    /// senza dirlo non decide al posto di nessuno.
    pub status: Option<PropertyStatus>,
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

        let mut property = Property::create(
            PropertyId::generate(),
            org_id,
            owner_id,
            &command.label,
            command.property_type,
            command.address,
            now,
        )?;

        // **Da `apply` e non da un parametro di `create`.** Passa da `set_status`,
        // che rifiuta `archived` con `property.archive_requires_dedicated_operation`:
        // un secondo controllo scritto qui direbbe la stessa cosa in un posto in
        // più, e il giorno in cui gli stati cambiassero ne resterebbe indietro uno.
        if let Some(status) = command.status {
            property.apply(
                PropertyChanges {
                    status: Some(status),
                    ..PropertyChanges::default()
                },
                now,
            )?;
        }

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
