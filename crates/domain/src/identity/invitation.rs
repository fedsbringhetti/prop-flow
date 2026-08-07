use chrono::{DateTime, Duration, Utc};

use super::member::OrgRole;
use super::session::TokenHash;
use crate::error::{DomainError, DomainResult};
use crate::shared::{Email, InvitationId, LeaseId, OrgId, UserId};

/// Invito a entrare in un'organizzazione, tipicamente come inquilino di un
/// contratto.
///
/// Del token resta soltanto lo hash, come per i refresh token: il valore in chiaro
/// esiste una volta sola, nella risposta che lo consegna. Una copia del dump non
/// permette a nessuno di accettare un invito altrui.
#[derive(Debug, Clone)]
pub struct Invitation {
    id: InvitationId,
    org_id: OrgId,
    email: Email,
    role: OrgRole,
    token_hash: TokenHash,
    lease_id: Option<LeaseId>,
    invited_by: Option<UserId>,
    expires_at: DateTime<Utc>,
    accepted_at: Option<DateTime<Utc>>,
    accepted_by: Option<UserId>,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

/// Dati di un invito da emettere.
///
/// Una struct e non sette argomenti posizionali: `OrgId`, `LeaseId` e `UserId` sono
/// tutti identificatori, e in posizione sbagliata il compilatore li distinguerebbe
/// solo per fortuna del tipo.
#[derive(Debug, Clone)]
pub struct NewInvitation {
    pub id: InvitationId,
    pub org_id: OrgId,
    pub email: Email,
    pub role: OrgRole,
    pub token_hash: TokenHash,
    pub lease_id: Option<LeaseId>,
    pub invited_by: Option<UserId>,
}

impl Invitation {
    /// Durata di un invito.
    ///
    /// Sette giorni: abbastanza perché un'email non urgente venga letta, poco
    /// perché un indirizzo compromesso mesi dopo non valga ancora come credenziale.
    pub const TTL_DAYS: i64 = 7;

    pub fn issue(invitation: NewInvitation, now: DateTime<Utc>) -> DomainResult<Self> {
        let NewInvitation {
            id,
            org_id,
            email,
            role,
            token_hash,
            lease_id,
            invited_by,
        } = invitation;

        // Un invito non è un modo di nominare uno staff: quel percorso passa da chi
        // amministra l'organizzazione, non da un token spedito per email. La stessa
        // condizione è ripetuta nella policy di riga, che è l'autorità.
        if !matches!(role, OrgRole::Tenant) {
            return Err(DomainError::validation(
                "role",
                "un invito può conferire soltanto il ruolo di inquilino",
            ));
        }
        Ok(Self {
            id,
            org_id,
            email,
            role,
            token_hash,
            lease_id,
            invited_by,
            expires_at: now + Duration::days(Self::TTL_DAYS),
            accepted_at: None,
            accepted_by: None,
            revoked_at: None,
            created_at: now,
        })
    }

    /// Verifica che l'invito sia ancora utilizzabile.
    ///
    /// I due esiti sono deliberatamente di natura diversa. "Scaduto" e "revocato"
    /// sono `Forbidden`: il token non vale più, ed è un problema di credenziale.
    /// "Già accettato" è un'invariante: chi presenta quel token *è* la persona
    /// giusta, e merita di sapere che l'operazione è già avvenuta invece di ricevere
    /// un rifiuto indistinguibile da un token sbagliato.
    pub fn ensure_acceptable(&self, now: DateTime<Utc>) -> DomainResult<()> {
        if self.accepted_at.is_some() {
            return Err(DomainError::invariant("invito già accettato"));
        }
        if self.revoked_at.is_some() {
            return Err(DomainError::Forbidden("invito revocato"));
        }
        if self.expires_at <= now {
            return Err(DomainError::Forbidden("invito scaduto"));
        }
        Ok(())
    }

    /// Marca l'invito come accettato da `user_id`.
    pub fn accept(&mut self, user_id: UserId, now: DateTime<Utc>) -> DomainResult<()> {
        self.ensure_acceptable(now)?;
        self.accepted_at = Some(now);
        self.accepted_by = Some(user_id);
        Ok(())
    }

    /// Revoca l'invito. Idempotente: revocare due volte non è un errore, perché
    /// l'esito richiesto — quel token non vale più — è già vero.
    pub fn revoke(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if self.accepted_at.is_some() {
            return Err(DomainError::invariant(
                "un invito già accettato non può essere revocato",
            ));
        }
        if self.revoked_at.is_none() {
            self.revoked_at = Some(now);
        }
        Ok(())
    }

    #[must_use]
    pub const fn id(&self) -> InvitationId {
        self.id
    }
    #[must_use]
    pub const fn org_id(&self) -> OrgId {
        self.org_id
    }
    #[must_use]
    pub const fn email(&self) -> &Email {
        &self.email
    }
    #[must_use]
    pub const fn role(&self) -> OrgRole {
        self.role
    }
    #[must_use]
    pub const fn token_hash(&self) -> TokenHash {
        self.token_hash
    }
    #[must_use]
    pub const fn lease_id(&self) -> Option<LeaseId> {
        self.lease_id
    }
    #[must_use]
    pub const fn invited_by(&self) -> Option<UserId> {
        self.invited_by
    }
    #[must_use]
    pub const fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
    #[must_use]
    pub const fn accepted_at(&self) -> Option<DateTime<Utc>> {
        self.accepted_at
    }
    #[must_use]
    pub const fn accepted_by(&self) -> Option<UserId> {
        self.accepted_by
    }
    #[must_use]
    pub const fn revoked_at(&self) -> Option<DateTime<Utc>> {
        self.revoked_at
    }
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Ricostruzione da persistenza: non riesegue le invarianti.
    #[must_use]
    pub fn from_persistence(parts: InvitationParts) -> Self {
        let InvitationParts {
            id,
            org_id,
            email,
            role,
            token_hash,
            lease_id,
            invited_by,
            expires_at,
            accepted_at,
            accepted_by,
            revoked_at,
            created_at,
        } = parts;
        Self {
            id,
            org_id,
            email,
            role,
            token_hash,
            lease_id,
            invited_by,
            expires_at,
            accepted_at,
            accepted_by,
            revoked_at,
            created_at,
        }
    }
}

/// Dati grezzi di un invito letti dalla persistenza.
#[derive(Debug, Clone)]
pub struct InvitationParts {
    pub id: InvitationId,
    pub org_id: OrgId,
    pub email: Email,
    pub role: OrgRole,
    pub token_hash: TokenHash,
    pub lease_id: Option<LeaseId>,
    pub invited_by: Option<UserId>,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub accepted_by: Option<UserId>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(email: &str, role: OrgRole) -> NewInvitation {
        NewInvitation {
            id: InvitationId::generate(),
            org_id: OrgId::generate(),
            email: Email::parse(email).expect("email valida"),
            role,
            token_hash: TokenHash::from_bytes([3; 32]),
            lease_id: Some(LeaseId::generate()),
            invited_by: Some(UserId::generate()),
        }
    }

    fn invitation(now: DateTime<Utc>) -> Invitation {
        Invitation::issue(input("inquilino@example.test", OrgRole::Tenant), now)
            .expect("invito valido")
    }

    #[test]
    fn a_fresh_invitation_is_acceptable() {
        let now = Utc::now();
        assert!(invitation(now).ensure_acceptable(now).is_ok());
    }

    #[test]
    fn an_invitation_cannot_grant_staff_roles() {
        for role in [OrgRole::Owner, OrgRole::Manager] {
            assert!(
                Invitation::issue(input("chiunque@example.test", role), Utc::now()).is_err(),
                "{role:?} non deve essere conferibile per invito"
            );
        }
    }

    #[test]
    fn an_expired_invitation_is_refused_as_a_credential() {
        let now = Utc::now();
        let invitation = invitation(now);
        let error = invitation
            .ensure_acceptable(now + Duration::days(Invitation::TTL_DAYS + 1))
            .expect_err("scaduto");
        assert!(matches!(error, DomainError::Forbidden(_)));
    }

    #[test]
    fn a_revoked_invitation_is_refused_as_a_credential() {
        let now = Utc::now();
        let mut invitation = invitation(now);
        invitation.revoke(now).expect("revocabile");
        assert!(matches!(
            invitation.ensure_acceptable(now).expect_err("revocato"),
            DomainError::Forbidden(_)
        ));
        // Revocare due volte non è un errore: l'esito richiesto è già vero.
        assert!(invitation.revoke(now).is_ok());
    }

    #[test]
    fn an_already_accepted_invitation_reports_an_invariant_not_a_refusal() {
        let now = Utc::now();
        let mut invitation = invitation(now);
        let user = UserId::generate();
        invitation.accept(user, now).expect("accettabile");
        assert_eq!(invitation.accepted_by(), Some(user));

        let error = invitation.accept(user, now).expect_err("già accettato");
        assert!(
            matches!(error, DomainError::Invariant(_)),
            "atteso Invariant, ottenuto {error:?}"
        );
        // E non si revoca a posteriori: la membership esiste già.
        assert!(invitation.revoke(now).is_err());
    }
}
