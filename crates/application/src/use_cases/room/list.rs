use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::ports::UnitOfWorkFactory;
use propflow_domain::property::Room;
use propflow_domain::shared::PropertyId;

use crate::authorization;
use crate::error::{AppError, AppResult};

/// Elenca le stanze di un immobile.
///
/// `ensure_member` e non `ensure_staff`: l'inquilino di un immobile in co-living
/// deve poter vedere le stanze di casa propria. Quali immobili siano "casa
/// propria" lo decide la Row Level Security attraverso i contratti.
pub struct ListRooms {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl ListRooms {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        property_id: PropertyId,
    ) -> AppResult<Vec<Room>> {
        authorization::ensure_member(context)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        // Un immobile non visibile dà 404 invece di un elenco vuoto: le due cose
        // sono diverse per il client, e una lista vuota su un immobile altrui
        // suggerirebbe che l'immobile esiste e non ha stanze.
        uow.properties()
            .find_by_id(property_id)
            .await?
            .ok_or(AppError::not_found("immobile"))?;
        let rooms = uow.rooms().list_for_property(property_id).await?;
        Ok(rooms)
    }
}
