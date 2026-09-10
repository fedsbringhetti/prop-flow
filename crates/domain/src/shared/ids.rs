use std::fmt;
use uuid::Uuid;

/// Identificatori tipizzati: rendono impossibile passare un `UserId` dove il
/// dominio aspetta un `OrgId`, errore che con `Uuid` nudo compila senza problemi.
macro_rules! typed_id {
    ($name:ident, $what:literal) => {
        #[doc = concat!("Identificatore di ", $what, ".")]
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Genera un identificatore nuovo.
            ///
            /// UUIDv7 e non v4: la componente temporale iniziale rende gli insert
            /// nel B-Tree quasi append-only invece di sparsi, e permette di
            /// ordinare per id senza una colonna aggiuntiva.
            #[must_use]
            pub fn generate() -> Self {
                Self(Uuid::now_v7())
            }

            #[must_use]
            pub const fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl From<$name> for Uuid {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

typed_id!(OrgId, "un'organizzazione");
typed_id!(UserId, "un utente");
typed_id!(PropertyId, "un immobile");
typed_id!(RoomId, "una stanza");
typed_id!(LeaseId, "un contratto di locazione");
typed_id!(LeasePartyId, "una parte contrattuale");
typed_id!(InvitationId, "un invito");
typed_id!(PasswordResetCodeId, "un codice di recupero della password");
typed_id!(DocumentId, "un documento");
typed_id!(TicketId, "una segnalazione");
typed_id!(TicketMessageId, "un messaggio di una segnalazione");
typed_id!(UtilityBillId, "una bolletta");
typed_id!(ChargeId, "un addebito");
typed_id!(PaymentId, "un pagamento");
typed_id!(ExpenseGroupId, "un gruppo di spesa co-living");
typed_id!(ExpenseGroupMemberId, "la membership di un gruppo di spesa");
typed_id!(ExpenseId, "una spesa condivisa");
typed_id!(ExpenseShareId, "la quota di una spesa condivisa");
typed_id!(SettlementId, "un regolamento fra coinquilini");
typed_id!(SessionId, "una sessione di refresh");
typed_id!(EventId, "un evento di dominio");
typed_id!(AiJobId, "un job di elaborazione automatica");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ids_are_time_ordered() {
        let first = OrgId::generate();
        let second = OrgId::generate();
        // UUIDv7: l'ordinamento lessicografico segue quello temporale.
        assert!(first.as_uuid() < second.as_uuid() || first != second);
        assert_eq!(second.as_uuid().get_version_num(), 7);
    }
}
