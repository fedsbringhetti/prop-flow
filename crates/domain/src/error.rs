use std::fmt;

use rust_decimal::Decimal;

/// Entità che una risposta `404` sa nominare.
///
/// Esiste come tipo e non come stringa perché da qui nasce il `code` della
/// risposta: con un `&'static str` il codice si otterrebbe confrontando testo, e
/// un refuso in un letterale cambierebbe il contratto pubblico senza che nulla se
/// ne accorga. Vedi ADR-022.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Entity {
    #[error("Property not found")]
    Property,
    #[error("Room not found")]
    Room,
    #[error("Lease not found")]
    Lease,
    #[error("Invitation not found")]
    Invitation,
    #[error("Document not found")]
    Document,
    #[error("Ticket not found")]
    Ticket,
    #[error("Utility bill not found")]
    UtilityBill,
    #[error("Charge not found")]
    Charge,
    #[error("Payment not found")]
    Payment,
    /// Rotta sconosciuta. Sta qui e non nel bordo HTTP perché un `404` di rotta e
    /// un `404` di risorsa devono avere la stessa forma per il client.
    #[error("Endpoint not found")]
    Endpoint,
}

impl Entity {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Property => "property.not_found",
            Self::Room => "room.not_found",
            Self::Lease => "lease.not_found",
            Self::Invitation => "invitation.not_found",
            Self::Document => "document.not_found",
            Self::Ticket => "ticket.not_found",
            Self::UtilityBill => "utility_bill.not_found",
            Self::Charge => "charge.not_found",
            Self::Payment => "payment.not_found",
            Self::Endpoint => "endpoint.not_found",
        }
    }
}

/// Motivo per cui un'operazione è negata a chi la chiede.
///
/// **Non tutte queste varianti raggiungono il bordo HTTP.** Quelle su account,
/// sessione e invito vengono ripiegate su `auth.invalid_credentials` dai casi
/// d'uso di login, refresh e accettazione: distinguerle in risposta regalerebbe a
/// un attaccante un oracolo per sapere se un account esiste, se è bloccato o se un
/// token d'invito è solo scaduto. Il codice esiste comunque perché il tipo lo
/// richiede e perché nei log la distinzione serve; non compare in
/// `docs/05-api.md`, che documenta ciò che l'API può produrre davvero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Denial {
    #[error("Authentication is required")]
    AuthenticationRequired,
    #[error("No organization is selected")]
    OrgNotSelected,
    #[error("You do not belong to this organization")]
    NotAMember,
    #[error("The owner or manager role is required")]
    StaffRoleRequired,
    /// Chi ha segnalato può riaprire e annullare; chi condivide il contratto no.
    ///
    /// Non è una scelta di prodotto rivedibile nel codice: la `USING` di
    /// `tickets_update` ammette staff, `reported_by` e `assigned_to`, e non passa
    /// dal contratto. Esiste come variante propria perché
    /// `org.staff_role_required` direbbe al coinquilino che gli manca un ruolo,
    /// mentre gli manca l'essere stato lui a segnalare — due rimedi diversi.
    #[error("This ticket was reported by someone else")]
    TicketNotYours,

    // Da qui in giù: mai in risposta, solo nei log.
    #[error("The account is not active")]
    AccountInactive,
    #[error("The account is temporarily locked")]
    AccountLocked,
    #[error("The account has no local credentials")]
    NoLocalCredentials,
    #[error("The session was revoked")]
    SessionRevoked,
    #[error("The session has expired")]
    SessionExpired,
    #[error("The invitation was revoked")]
    InvitationRevoked,
    #[error("The invitation has expired")]
    InvitationExpired,
}

impl Denial {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::AuthenticationRequired => "auth.required",
            Self::OrgNotSelected => "org.not_selected",
            Self::NotAMember => "org.not_a_member",
            Self::StaffRoleRequired => "org.staff_role_required",
            Self::TicketNotYours => "ticket.not_yours",
            Self::AccountInactive => "account.inactive",
            Self::AccountLocked => "account.locked",
            Self::NoLocalCredentials => "account.no_local_credentials",
            Self::SessionRevoked => "session.revoked",
            Self::SessionExpired => "session.expired",
            Self::InvitationRevoked => "invitation.revoked",
            Self::InvitationExpired => "invitation.expired",
        }
    }
}

/// Invariante di aggregato che l'operazione richiesta violerebbe.
///
/// Più varianti possono condividere un `code`: le tre transizioni di contratto
/// danno tutte `lease.invalid_transition`, e le due operazioni su un invito già
/// accettato danno `invitation.already_accepted`. È deliberato — il codice dice al
/// client **cosa fare**, e in quei casi il rimedio è lo stesso. Le varianti
/// restano distinte perché il messaggio che accompagna il codice non lo è.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Invariant {
    #[error("An archived property cannot be modified")]
    PropertyArchived,
    #[error("Archiving requires the dedicated operation, not a status update")]
    PropertyArchiveRequiresDedicatedOperation,
    #[error("An archived room cannot be modified")]
    RoomArchived,

    #[error("Only a draft or a lease awaiting signature can be activated")]
    LeaseNotActivatable,
    #[error("Only an active lease can be terminated")]
    LeaseNotTerminable,
    #[error("Only a draft or a lease awaiting signature can be cancelled")]
    LeaseNotCancellable,
    #[error("This lease party is already linked to another user")]
    LeasePartyAlreadyLinked,
    #[error("The lease has no tenant among its parties")]
    LeaseWithoutTenantParty,
    #[error("Tenant shares add up to {total} instead of 100")]
    LeaseSharesNotHundred { total: Decimal },

    #[error("The invitation has already been accepted")]
    InvitationAlreadyAccepted,
    #[error("An invitation that was already accepted cannot be revoked")]
    AcceptedInvitationCannotBeRevoked,

    #[error("The document is already archived")]
    DocumentArchived,

    #[error("Only an open ticket can be triaged")]
    TicketNotTriageable,
    #[error("A ticket can only be assigned while it is open, triaged or already assigned")]
    TicketNotAssignable,
    #[error("Work can only start on an assigned or suspended ticket")]
    TicketNotStartable,
    #[error("Only a ticket being worked on can be suspended")]
    TicketNotSuspendable,
    #[error("Only a ticket still being handled can be closed")]
    TicketNotClosable,
    #[error("Only a closed ticket can be reopened")]
    TicketNotReopenable,
    #[error("Only a ticket still being handled can be cancelled")]
    TicketNotCancellable,
    #[error("A cancelled ticket cannot be modified")]
    TicketCancelled,

    #[error("Only a draft or a validated bill can be corrected")]
    UtilityBillNotEditable,
    #[error("Only a draft bill can be validated")]
    UtilityBillNotValidatable,
    #[error("Only a validated or already allocated bill can be recharged")]
    UtilityBillNotAllocatable,
    /// Da `allocated` in poi la bolletta è la base di un addebito già emesso:
    /// farla sparire lascerebbe quell'addebito a citare una riga invisibile.
    #[error("A bill that has already been recharged cannot be voided")]
    UtilityBillNotVoidable,

    #[error("Only an issued charge can be voided")]
    ChargeNotVoidable,
    #[error("Only an issued charge can be written off")]
    ChargeNotWriteOffable,
    /// La pretesa è caduta: incassare su un addebito annullato o stornato è
    /// un'incoerenza, non un caso d'uso.
    #[error("A voided or written off charge cannot take a payment")]
    ChargeNotPayable,

    #[error("Only a pending payment can be confirmed")]
    PaymentNotConfirmable,
    #[error("Only a pending payment can be rejected")]
    PaymentNotRejectable,

    #[error("Refresh token already rotated: possible reuse, revoke the chain")]
    RefreshTokenReused,
}

impl Invariant {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::PropertyArchived => "property.archived",
            Self::PropertyArchiveRequiresDedicatedOperation => {
                "property.archive_requires_dedicated_operation"
            }
            Self::RoomArchived => "room.archived",
            Self::LeaseNotActivatable | Self::LeaseNotTerminable | Self::LeaseNotCancellable => {
                "lease.invalid_transition"
            }
            Self::LeasePartyAlreadyLinked => "lease.party_already_linked",
            Self::LeaseWithoutTenantParty => "lease.no_tenant_party",
            Self::LeaseSharesNotHundred { .. } => "lease.shares_not_hundred",
            Self::InvitationAlreadyAccepted | Self::AcceptedInvitationCannotBeRevoked => {
                "invitation.already_accepted"
            }
            Self::DocumentArchived => "document.archived",
            // Sette varianti, un codice: il rimedio è lo stesso — rileggere lo
            // stato attuale e chiedere una transizione ammessa — e le tre
            // transizioni di contratto hanno già lo stesso trattamento. Restano
            // distinte perché il messaggio che accompagna il codice non lo è.
            Self::TicketNotTriageable
            | Self::TicketNotAssignable
            | Self::TicketNotStartable
            | Self::TicketNotSuspendable
            | Self::TicketNotClosable
            | Self::TicketNotReopenable
            | Self::TicketNotCancellable => "ticket.invalid_transition",
            Self::TicketCancelled => "ticket.cancelled",
            // Stesso trattamento delle transizioni di contratto e di
            // segnalazione: il rimedio è lo stesso — rileggere lo stato attuale e
            // chiedere una transizione ammessa — e i due aggregati restano
            // distinti perché il rimedio *non* è lo stesso fra loro.
            Self::UtilityBillNotEditable
            | Self::UtilityBillNotValidatable
            | Self::UtilityBillNotAllocatable
            | Self::UtilityBillNotVoidable => "utility_bill.invalid_transition",
            Self::ChargeNotVoidable | Self::ChargeNotWriteOffable | Self::ChargeNotPayable => {
                "charge.invalid_transition"
            }
            // Stesso trattamento degli altri aggregati: il rimedio è rileggere lo
            // stato attuale e chiedere una transizione ammessa. Il codice è
            // **distinto** da `charge.invalid_transition` perché un pagamento che
            // non si può confermare e un addebito che non accetta incassi si
            // rimediano in due posti diversi.
            Self::PaymentNotConfirmable | Self::PaymentNotRejectable => {
                "payment.invalid_transition"
            }
            Self::RefreshTokenReused => "session.refresh_token_reused",
        }
    }
}

/// Errori che il dominio sa produrre. Non contengono mai dettagli di
/// infrastruttura: la traduzione verso HTTP avviene ai bordi.
///
/// I messaggi sono in inglese perché finiscono nel campo `detail` della risposta,
/// che è superficie pubblica. I commenti di questo crate restano in italiano.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// Un valore in ingresso non rispetta le regole del value object.
    ///
    /// Il `code` è sempre `validation.failed`: a discriminare è `field`, che il
    /// bordo HTTP riporta in `errors[]`. Un codice per campo moltiplicherebbe il
    /// contratto senza dire al client nulla che `field` non dica già.
    #[error("invalid field `{field}`: {message}")]
    Validation {
        field: &'static str,
        message: String,
    },

    /// Un'invariante dell'aggregato sarebbe violata dall'operazione richiesta.
    #[error("{0}")]
    Invariant(Invariant),

    /// L'entità richiesta non esiste (o non è visibile a chi la chiede: la RLS
    /// rende i due casi indistinguibili, ed è voluto — non si conferma
    /// l'esistenza di dati che non si possono vedere).
    #[error("{0}")]
    NotFound(Entity),

    /// L'attore non ha i permessi per l'operazione.
    #[error("{0}")]
    Forbidden(Denial),
}

impl DomainError {
    pub fn validation(field: &'static str, message: impl fmt::Display) -> Self {
        Self::Validation {
            field,
            message: message.to_string(),
        }
    }

    /// Codice stabile della condizione, parte del contratto pubblico (ADR-022).
    ///
    /// Nasce qui e non al bordo HTTP: il bordo lo trascrive, non lo inventa.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Validation { .. } => "validation.failed",
            Self::Invariant(invariant) => invariant.code(),
            Self::NotFound(entity) => entity.code(),
            Self::Forbidden(denial) => denial.code(),
        }
    }
}

pub type DomainResult<T> = Result<T, DomainError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Il formato `<dominio>.<condizione>` è il contratto: un codice che non lo
    /// rispetta è un codice che un client non può instradare.
    #[test]
    fn every_code_is_a_dotted_pair_of_snake_case_segments() {
        let codes: Vec<&'static str> = [
            Entity::Property.code(),
            Entity::Room.code(),
            Entity::Lease.code(),
            Entity::Invitation.code(),
            Entity::Document.code(),
            Entity::Ticket.code(),
            Entity::UtilityBill.code(),
            Entity::Charge.code(),
            Entity::Payment.code(),
            Entity::Endpoint.code(),
            Denial::AuthenticationRequired.code(),
            Denial::OrgNotSelected.code(),
            Denial::NotAMember.code(),
            Denial::StaffRoleRequired.code(),
            Denial::TicketNotYours.code(),
            Denial::AccountInactive.code(),
            Denial::AccountLocked.code(),
            Denial::NoLocalCredentials.code(),
            Denial::SessionRevoked.code(),
            Denial::SessionExpired.code(),
            Denial::InvitationRevoked.code(),
            Denial::InvitationExpired.code(),
            Invariant::PropertyArchived.code(),
            Invariant::PropertyArchiveRequiresDedicatedOperation.code(),
            Invariant::RoomArchived.code(),
            Invariant::LeaseNotActivatable.code(),
            Invariant::LeasePartyAlreadyLinked.code(),
            Invariant::LeaseWithoutTenantParty.code(),
            Invariant::LeaseSharesNotHundred {
                total: Decimal::from(90),
            }
            .code(),
            Invariant::InvitationAlreadyAccepted.code(),
            Invariant::DocumentArchived.code(),
            Invariant::TicketNotTriageable.code(),
            Invariant::TicketNotAssignable.code(),
            Invariant::TicketNotStartable.code(),
            Invariant::TicketNotSuspendable.code(),
            Invariant::TicketNotClosable.code(),
            Invariant::TicketNotReopenable.code(),
            Invariant::TicketNotCancellable.code(),
            Invariant::TicketCancelled.code(),
            Invariant::UtilityBillNotEditable.code(),
            Invariant::UtilityBillNotValidatable.code(),
            Invariant::UtilityBillNotAllocatable.code(),
            Invariant::UtilityBillNotVoidable.code(),
            Invariant::ChargeNotVoidable.code(),
            Invariant::ChargeNotWriteOffable.code(),
            Invariant::ChargeNotPayable.code(),
            Invariant::PaymentNotConfirmable.code(),
            Invariant::PaymentNotRejectable.code(),
            Invariant::RefreshTokenReused.code(),
            DomainError::validation("field", "message").code(),
        ]
        .to_vec();

        for code in codes {
            let (domain, condition) = code
                .split_once('.')
                .unwrap_or_else(|| panic!("`{code}` non è nella forma <dominio>.<condizione>"));
            assert!(!domain.is_empty() && !condition.is_empty(), "`{code}`");
            assert!(
                code.chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c == '.'),
                "`{code}` deve essere snake_case minuscolo"
            );
            assert_eq!(condition.find('.'), None, "`{code}` ha più di due segmenti");
        }
    }

    /// Le tre transizioni di contratto condividono un codice perché il rimedio è
    /// lo stesso; se qualcuno le separasse, il test dice che è una scelta e non
    /// una svista.
    #[test]
    fn lease_transitions_share_one_code() {
        assert_eq!(
            Invariant::LeaseNotActivatable.code(),
            Invariant::LeaseNotTerminable.code()
        );
        assert_eq!(
            Invariant::LeaseNotActivatable.code(),
            Invariant::LeaseNotCancellable.code()
        );
        assert_eq!(
            Invariant::InvitationAlreadyAccepted.code(),
            Invariant::AcceptedInvitationCannotBeRevoked.code()
        );
    }

    /// Le sette transizioni di segnalazione condividono un codice per la stessa
    /// ragione delle tre di contratto: il rimedio non cambia. `TicketCancelled`
    /// invece **non** lo condivide, perché non è una transizione rifiutata ma una
    /// modifica su un aggregato che non ne accetta più.
    #[test]
    fn ticket_transitions_share_one_code_but_cancellation_does_not() {
        let transitions = [
            Invariant::TicketNotTriageable,
            Invariant::TicketNotAssignable,
            Invariant::TicketNotStartable,
            Invariant::TicketNotSuspendable,
            Invariant::TicketNotClosable,
            Invariant::TicketNotReopenable,
            Invariant::TicketNotCancellable,
        ];
        for invariant in &transitions {
            assert_eq!(invariant.code(), "ticket.invalid_transition");
        }
        assert_ne!(
            Invariant::TicketCancelled.code(),
            Invariant::TicketNotClosable.code()
        );
    }

    /// I due aggregati contabili condividono un codice **ciascuno**, e non fra
    /// loro: da una bolletta che non si può correggere e da un addebito che non
    /// si può annullare non si esce nello stesso modo, e un client che instrada
    /// su questi codici deve poterli distinguere.
    #[test]
    fn billing_transitions_share_one_code_per_aggregate() {
        for invariant in [
            Invariant::UtilityBillNotEditable,
            Invariant::UtilityBillNotValidatable,
            Invariant::UtilityBillNotAllocatable,
            Invariant::UtilityBillNotVoidable,
        ] {
            assert_eq!(invariant.code(), "utility_bill.invalid_transition");
        }
        for invariant in [
            Invariant::ChargeNotVoidable,
            Invariant::ChargeNotWriteOffable,
            Invariant::ChargeNotPayable,
        ] {
            assert_eq!(invariant.code(), "charge.invalid_transition");
        }
        assert_ne!(
            Invariant::UtilityBillNotVoidable.code(),
            Invariant::ChargeNotVoidable.code()
        );
    }

    /// Le due transizioni del pagamento condividono un codice fra loro e **non**
    /// con l'addebito: un pagamento che non si può confermare e un addebito che
    /// non accetta incassi si rimediano in due posti diversi, e un client che
    /// instrada su questi codici deve poterli distinguere.
    #[test]
    fn payment_transitions_have_their_own_code() {
        for invariant in [
            Invariant::PaymentNotConfirmable,
            Invariant::PaymentNotRejectable,
        ] {
            assert_eq!(invariant.code(), "payment.invalid_transition");
        }
        assert_ne!(
            Invariant::PaymentNotConfirmable.code(),
            Invariant::ChargeNotPayable.code()
        );
    }
}
