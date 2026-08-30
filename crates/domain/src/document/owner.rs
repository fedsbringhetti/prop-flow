use uuid::Uuid;

use crate::shared::{LeaseId, PropertyId, RoomId, TicketId};

/// Tipo dell'oggetto a cui un documento è allegato.
///
/// L'enum del database (`app.attachment_owner_type`) ha undici valori; qui ce ne
/// sono quattro. Non è una svista: sono gli unici aggregati che hanno già dominio,
/// caso d'uso ed endpoint, e un allegato a una bolletta inesistente nell'API non è
/// modellabile. La conseguenza voluta è che un documento scritto a mano su un
/// altro tipo di owner dà **errore** in lettura invece di essere ignorato — la
/// stessa regola degli altri enum (`docs/06-sviluppo.md` §Aggiungere un enum).
///
/// `Ticket` è arrivato con le segnalazioni, ed è costato **una variante e due
/// rotte**: `can_access_attachment` risolveva già `ticket` su `can_access_ticket`
/// dal primo giorno. È il dividendo di aver fatto i documenti per primi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentOwnerType {
    Property,
    Room,
    Lease,
    Ticket,
}

/// Oggetto a cui un documento è allegato, con il proprio identificatore.
///
/// Tipo e identificatore viaggiano insieme e non come due campi accostati: la
/// colonna `owner_id` non ha una foreign key — l'integrità la garantisce il
/// trigger `app.tg_check_polymorphic_ref` — quindi una coppia incoerente non
/// verrebbe fermata dallo schema come lo sarebbe una FK sbagliata. Qui non è
/// nemmeno esprimibile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentOwner {
    Property(PropertyId),
    Room(RoomId),
    Lease(LeaseId),
    Ticket(TicketId),
}

impl AttachmentOwner {
    #[must_use]
    pub const fn owner_type(self) -> AttachmentOwnerType {
        match self {
            Self::Property(_) => AttachmentOwnerType::Property,
            Self::Room(_) => AttachmentOwnerType::Room,
            Self::Lease(_) => AttachmentOwnerType::Lease,
            Self::Ticket(_) => AttachmentOwnerType::Ticket,
        }
    }

    /// Identificatore nudo: è ciò che finisce nella colonna polimorfica.
    #[must_use]
    pub const fn owner_id(self) -> Uuid {
        match self {
            Self::Property(id) => id.as_uuid(),
            Self::Room(id) => id.as_uuid(),
            Self::Lease(id) => id.as_uuid(),
            Self::Ticket(id) => id.as_uuid(),
        }
    }

    /// Ricomposizione dalla coppia letta in database.
    #[must_use]
    pub const fn from_persistence(owner_type: AttachmentOwnerType, owner_id: Uuid) -> Self {
        match owner_type {
            AttachmentOwnerType::Property => Self::Property(PropertyId::from_uuid(owner_id)),
            AttachmentOwnerType::Room => Self::Room(RoomId::from_uuid(owner_id)),
            AttachmentOwnerType::Lease => Self::Lease(LeaseId::from_uuid(owner_id)),
            AttachmentOwnerType::Ticket => Self::Ticket(TicketId::from_uuid(owner_id)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_survives_a_round_trip_through_persistence() {
        let owners = [
            AttachmentOwner::Property(PropertyId::generate()),
            AttachmentOwner::Room(RoomId::generate()),
            AttachmentOwner::Lease(LeaseId::generate()),
            AttachmentOwner::Ticket(TicketId::generate()),
        ];
        for owner in owners {
            let rebuilt = AttachmentOwner::from_persistence(owner.owner_type(), owner.owner_id());
            assert_eq!(rebuilt, owner);
        }
    }
}
