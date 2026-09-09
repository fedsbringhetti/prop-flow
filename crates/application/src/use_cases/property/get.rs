use std::sync::Arc;

use propflow_domain::ports::UnitOfWorkFactory;
use propflow_domain::property::{Property, PropertyStatus};
use propflow_domain::shared::PropertyId;
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

/// Un immobile con lo stato che **discende dai contratti**, quando ce n'è uno.
///
/// `occupancy_status` è `None` quando non c'è niente da derivare, e quel `None`
/// non significa «disponibile»: significa che l'ultima parola resta allo stato
/// memorizzato, che può essere una bozza o un immobile in manutenzione — giudizi
/// di chi amministra, non conseguenze (ADR-019).
#[derive(Debug, Clone)]
pub struct PropertyView {
    pub property: Property,
    pub occupancy_status: Option<PropertyStatus>,
}

/// Legge un immobile.
///
/// Non serve distinguere "non esiste" da "non visibile": la Row Level Security
/// non restituisce la riga in nessuno dei due casi, e rispondere `404` a entrambi
/// è la scelta corretta — un `403` confermerebbe che l'immobile esiste.
pub struct GetProperty {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl GetProperty {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        id: PropertyId,
    ) -> AppResult<PropertyView> {
        authorization::ensure_member(context)?;
        let mut uow = self.unit_of_work.begin(context).await?;
        let property = uow
            .properties()
            .find_by_id(id)
            .await?
            .ok_or(AppError::not_found(Entity::Property))?;
        let occupancy = uow.properties().occupancy(&[property.id()]).await?;
        let derived = occupancy.first().and_then(|entry| entry.status);
        Ok(PropertyView {
            property,
            occupancy_status: derived,
        })
    }
}
