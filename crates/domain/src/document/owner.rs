use uuid::Uuid;

use crate::shared::{ChargeId, LeaseId, PropertyId, RoomId, TicketId, UtilityBillId};

/// Tipo dell'oggetto a cui un documento è allegato.
///
/// L'enum del database (`app.attachment_owner_type`) ha undici valori; qui ce ne
/// sono sei. Non è una svista: sono gli unici aggregati che hanno già dominio,
/// caso d'uso ed endpoint, e un allegato a un pagamento inesistente nell'API non è
/// modellabile. La conseguenza voluta è che un documento scritto a mano su un
/// altro tipo di owner dà **errore** in lettura invece di essere ignorato — la
/// stessa regola degli altri enum (`docs/06-sviluppo.md` §Aggiungere un enum).
///
/// `Ticket` è arrivato con le segnalazioni, ed è costato **una variante e due
/// rotte**: `can_access_attachment` risolveva già `ticket` su `can_access_ticket`
/// dal primo giorno. È il dividendo di aver fatto i documenti per primi.
///
/// `UtilityBill` e `Charge` sono arrivati con le bollette, e il dividendo si è
/// ripetuto — con **un'eccezione che vale la pena ricordare**. Il ramo `charge`
/// della funzione era già corretto: risolve su `can_access_lease(c.lease_id)`,
/// cioè esattamente la condizione di `charges_select`. Il ramo `utility_bill` no:
/// risolveva su `can_access_property`, che è **più largo** di
/// `utility_bills_select` — in un co-living con contratti per stanza, ogni
/// inquilino dell'immobile avrebbe scaricato la bolletta imputata al contratto di
/// un altro, senza poterne vedere la riga. La migration `0022` lo riallinea, ed è
/// parte di questa fetta perché è questa fetta ad aprire quelle rotte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentOwnerType {
    Property,
    Room,
    Lease,
    Ticket,
    UtilityBill,
    Charge,
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
    UtilityBill(UtilityBillId),
    Charge(ChargeId),
}

impl AttachmentOwner {
    #[must_use]
    pub const fn owner_type(self) -> AttachmentOwnerType {
        match self {
            Self::Property(_) => AttachmentOwnerType::Property,
            Self::Room(_) => AttachmentOwnerType::Room,
            Self::Lease(_) => AttachmentOwnerType::Lease,
            Self::Ticket(_) => AttachmentOwnerType::Ticket,
            Self::UtilityBill(_) => AttachmentOwnerType::UtilityBill,
            Self::Charge(_) => AttachmentOwnerType::Charge,
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
            Self::UtilityBill(id) => id.as_uuid(),
            Self::Charge(id) => id.as_uuid(),
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
            AttachmentOwnerType::UtilityBill => {
                Self::UtilityBill(UtilityBillId::from_uuid(owner_id))
            }
            AttachmentOwnerType::Charge => Self::Charge(ChargeId::from_uuid(owner_id)),
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
            AttachmentOwner::UtilityBill(UtilityBillId::generate()),
            AttachmentOwner::Charge(ChargeId::generate()),
        ];
        for owner in owners {
            let rebuilt = AttachmentOwner::from_persistence(owner.owner_type(), owner.owner_id());
            assert_eq!(rebuilt, owner);
        }
    }
}
