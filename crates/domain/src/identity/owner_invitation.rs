use chrono::{DateTime, Utc};

use super::session::TokenHash;
use crate::error::{Denial, DomainError, DomainResult, Invariant};
use crate::shared::{Email, OwnerInvitationId, UserId};

/// Invito a diventare proprietario: l'accettazione crea insieme utente e
/// organizzazione, dietro un token di piattaforma (ADR-053).
///
/// A differenza di [`Invitation`](super::Invitation) non porta `org_id` — non
/// esiste ancora, la crea l'accettazione — né `role`: chi lo accetta diventa
/// sempre `owner` della propria organizzazione. **Nessun costruttore
/// `issue`**: la riga la scrive uno script in `ops/`, con il ruolo del
/// migratore, perché la verifica di chi sia un proprietario vero sta fuori dal
/// prodotto. Il dominio sa solo leggerlo e accettarlo.
#[derive(Debug, Clone)]
pub struct OwnerInvitation {
    id: OwnerInvitationId,
    email: Email,
    token_hash: TokenHash,
    expires_at: DateTime<Utc>,
    accepted_at: Option<DateTime<Utc>>,
    accepted_by: Option<UserId>,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl OwnerInvitation {
    /// Verifica che l'invito sia ancora utilizzabile.
    ///
    /// **A differenza di [`Invitation::ensure_acceptable`](super::Invitation::ensure_acceptable),
    /// qui i tre esiti negativi sono trattati allo stesso modo** (ADR-053,
    /// decisione 6): non solo scaduto e revocato, ma anche già accettato. Chi
    /// presenta un token da proprietario non ha ancora una sessione, quindi non
    /// c'è modo di verificare che sia "la persona giusta" come invece si fa per
    /// l'invito da inquilino (dove chi accetta è già autenticato) — e
    /// distinguere gli esiti sarebbe di nuovo l'oracolo che ADR-017 vieta. Il
    /// chiamante collassa ogni errore di questo metodo sullo stesso
    /// `401 auth.invalid_credentials`; i codici restano distinti perché il tipo
    /// li richiede e perché nei log la distinzione serve.
    pub fn ensure_acceptable(&self, now: DateTime<Utc>) -> DomainResult<()> {
        if self.accepted_at.is_some() {
            return Err(DomainError::Invariant(
                Invariant::OwnerInvitationAlreadyAccepted,
            ));
        }
        if self.revoked_at.is_some() {
            return Err(DomainError::Forbidden(Denial::InvitationRevoked));
        }
        if self.expires_at <= now {
            return Err(DomainError::Forbidden(Denial::InvitationExpired));
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

    #[must_use]
    pub const fn id(&self) -> OwnerInvitationId {
        self.id
    }
    #[must_use]
    pub const fn email(&self) -> &Email {
        &self.email
    }
    #[must_use]
    pub const fn token_hash(&self) -> TokenHash {
        self.token_hash
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
    pub fn from_persistence(parts: OwnerInvitationParts) -> Self {
        let OwnerInvitationParts {
            id,
            email,
            token_hash,
            expires_at,
            accepted_at,
            accepted_by,
            revoked_at,
            created_at,
        } = parts;
        Self {
            id,
            email,
            token_hash,
            expires_at,
            accepted_at,
            accepted_by,
            revoked_at,
            created_at,
        }
    }
}

/// Dati grezzi di un invito da proprietario letti dalla persistenza.
#[derive(Debug, Clone)]
pub struct OwnerInvitationParts {
    pub id: OwnerInvitationId,
    pub email: Email,
    pub token_hash: TokenHash,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub accepted_by: Option<UserId>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(email: &str, now: DateTime<Utc>) -> OwnerInvitationParts {
        OwnerInvitationParts {
            id: OwnerInvitationId::generate(),
            email: Email::parse(email).expect("email valida"),
            token_hash: TokenHash::from_bytes([9; 32]),
            expires_at: now + chrono::Duration::days(7),
            accepted_at: None,
            accepted_by: None,
            revoked_at: None,
            created_at: now,
        }
    }

    fn invitation(now: DateTime<Utc>) -> OwnerInvitation {
        OwnerInvitation::from_persistence(parts("proprietario@example.test", now))
    }

    #[test]
    fn a_fresh_invitation_is_acceptable() {
        let now = Utc::now();
        assert!(invitation(now).ensure_acceptable(now).is_ok());
    }

    #[test]
    fn an_expired_invitation_is_refused_as_a_credential() {
        let now = Utc::now();
        let invitation = invitation(now);
        // `parts` fissa la scadenza a sette giorni da `now`: un giorno oltre
        // basta a superarla.
        let error = invitation
            .ensure_acceptable(now + chrono::Duration::days(8))
            .expect_err("scaduto");
        assert!(matches!(error, DomainError::Forbidden(_)));
    }

    #[test]
    fn a_revoked_invitation_is_refused_as_a_credential() {
        let now = Utc::now();
        let mut raw = parts("proprietario@example.test", now);
        raw.revoked_at = Some(now);
        let invitation = OwnerInvitation::from_persistence(raw);
        assert!(matches!(
            invitation.ensure_acceptable(now).expect_err("revocato"),
            DomainError::Forbidden(_)
        ));
    }

    /// Già accettato è un'invariante e non un rifiuto di credenziale — la
    /// distinzione esiste nel dominio anche se il caso d'uso la appiattisce sullo
    /// stesso `401` (ADR-053, decisione 6).
    #[test]
    fn an_already_accepted_invitation_reports_an_invariant() {
        let now = Utc::now();
        let mut raw = parts("proprietario@example.test", now);
        raw.accepted_at = Some(now);
        raw.accepted_by = Some(UserId::generate());
        let invitation = OwnerInvitation::from_persistence(raw);
        assert!(matches!(
            invitation
                .ensure_acceptable(now)
                .expect_err("già accettato"),
            DomainError::Invariant(Invariant::OwnerInvitationAlreadyAccepted)
        ));
    }

    #[test]
    fn accept_sets_the_acceptor_and_the_instant() {
        let now = Utc::now();
        let mut invitation = invitation(now);
        let user = UserId::generate();
        invitation.accept(user, now).expect("accettabile");
        assert_eq!(invitation.accepted_by(), Some(user));
        assert_eq!(invitation.accepted_at(), Some(now));

        // Non si accetta due volte.
        let error = invitation.accept(user, now).expect_err("già accettato");
        assert!(matches!(error, DomainError::Invariant(_)));
    }
}
