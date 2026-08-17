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
            Entity::Endpoint.code(),
            Denial::AuthenticationRequired.code(),
            Denial::OrgNotSelected.code(),
            Denial::NotAMember.code(),
            Denial::StaffRoleRequired.code(),
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
}
