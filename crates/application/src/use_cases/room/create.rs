use std::sync::Arc;

use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, RoomCreated};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::property::{Room, RoomChanges};
use propflow_domain::shared::{PropertyId, RoomId};
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult, Conflict};

#[derive(Debug, Clone)]
pub struct CreateRoomCommand {
    pub property_id: PropertyId,
    pub name: String,
    /// Gli attributi opzionali arrivano dalla stessa struttura del PATCH: una
    /// stanza appena creata e una modificata condividono le identiche
    /// invarianti, e duplicarle in due tipi le farebbe divergere.
    pub attributes: RoomChanges,
}

/// Crea una stanza dentro un immobile.
pub struct CreateRoom {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl CreateRoom {
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
        command: CreateRoomCommand,
    ) -> AppResult<Room> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;

        // L'immobile si legge prima di scrivere. Senza questo controllo un
        // `property_id` di un'altra organizzazione arriverebbe fino al trigger
        // `assert_org_matches_property` o alla policy di riga, e il chiamante
        // vedrebbe un 409 o un 403 dove la risposta corretta è 404: sotto RLS
        // "non è tuo" non si distingue da "non esiste".
        let property = uow
            .properties()
            .find_by_id(command.property_id)
            .await?
            .ok_or(AppError::not_found(Entity::Property))?;

        let mut room = Room::create(
            RoomId::generate(),
            org_id,
            property.id(),
            &command.name,
            now,
        )?;
        room.apply(command.attributes, now)?;

        uow.rooms()
            .insert(&room)
            .await
            .map_err(|error| map_name_conflict(error.into()))?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::RoomCreated(RoomCreated {
                    room_id: room.id(),
                    property_id: room.property_id(),
                    org_id,
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(room)
    }
}

/// Il nome di una stanza è unico dentro l'immobile: il vincolo parziale
/// `rooms_property_name_uk` lo impone anche fra transazioni concorrenti. Tradurlo
/// evita che un nome ripetuto esca come "vincolo violato" senza dire quale.
pub(super) fn map_name_conflict(error: AppError) -> AppError {
    match &error {
        AppError::Integrity { details } if details.contains("rooms_property_name") => {
            AppError::Conflict(Conflict::RoomNameTaken)
        }
        _ => error,
    }
}
