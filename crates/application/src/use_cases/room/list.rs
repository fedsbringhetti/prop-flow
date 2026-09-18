use std::sync::Arc;

use propflow_domain::ports::UnitOfWorkFactory;
use propflow_domain::property::Room;
use propflow_domain::shared::{PropertyId, RoomId};
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

/// Una stanza con l'occupazione **derivata** accanto allo stato memorizzato.
///
/// Due campi e non uno, ed è deliberato: `status` è ciò che qualcuno ha deciso —
/// una stanza in ristrutturazione è `unavailable` — e `occupied` è ciò che
/// discende dai contratti. Un campo solo, con il derivato che copre il
/// memorizzato, nasconderebbe proprio il caso che chi amministra deve vedere: una
/// stanza dichiarata non disponibile **mentre** ci abita una persona.
#[derive(Debug, Clone)]
pub struct RoomView {
    pub room: Room,
    pub occupied: bool,
}

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
    ) -> AppResult<Vec<RoomView>> {
        authorization::ensure_member(context)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        // Un immobile non visibile dà 404 invece di un elenco vuoto: le due cose
        // sono diverse per il client, e una lista vuota su un immobile altrui
        // suggerirebbe che l'immobile esiste e non ha stanze.
        uow.properties()
            .find_by_id(property_id)
            .await?
            .ok_or(AppError::not_found(Entity::Property))?;
        let rooms = uow.rooms().list_for_property(property_id).await?;

        // Una query per l'elenco intero, non una per stanza.
        let ids: Vec<RoomId> = rooms.iter().map(Room::id).collect();
        let occupancy = uow.rooms().occupancy(&ids).await?;

        Ok(rooms
            .into_iter()
            .map(|room| {
                let occupied = occupancy
                    .iter()
                    .find(|entry| entry.room_id == room.id())
                    .is_some_and(|entry| entry.occupied);
                RoomView { room, occupied }
            })
            .collect())
    }
}
