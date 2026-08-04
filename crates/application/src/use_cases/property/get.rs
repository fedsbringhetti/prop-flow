use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::ports::UnitOfWorkFactory;
use propflow_domain::property::Property;
use propflow_domain::shared::PropertyId;

use crate::authorization;
use crate::error::{AppError, AppResult};

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

    pub async fn execute(&self, context: &RequestContext, id: PropertyId) -> AppResult<Property> {
        authorization::ensure_member(context)?;
        let mut uow = self.unit_of_work.begin(context).await?;
        let property = uow.properties().find_by_id(id).await?;
        property.ok_or(AppError::not_found("immobile"))
    }
}
