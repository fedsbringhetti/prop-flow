use chrono::{DateTime, Utc};

use crate::error::{DomainError, DomainResult};
use crate::shared::{SessionId, UserId};

/// Hash SHA-256 di un refresh token.
///
/// Il token in chiaro esiste solo nella risposta HTTP che lo consegna: in
/// database va l'hash, così una copia del dump non permette di impersonare
/// nessuno. Stesso motivo per cui non si salvano le password in chiaro.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenHash([u8; 32]);

impl TokenHash {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn from_slice(bytes: &[u8]) -> DomainResult<Self> {
        let sized: [u8; 32] = bytes
            .try_into()
            .map_err(|_| DomainError::validation("token_hash", "attesi 32 byte"))?;
        Ok(Self(sized))
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for TokenHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TokenHash(<redatto>)")
    }
}

/// Sessione di refresh: un token opaco, a rotazione, con rilevamento del riuso.
#[derive(Debug, Clone)]
pub struct RefreshSession {
    id: SessionId,
    user_id: UserId,
    token_hash: TokenHash,
    issued_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
    rotated_to: Option<SessionId>,
}

impl RefreshSession {
    pub fn issue(
        id: SessionId,
        user_id: UserId,
        token_hash: TokenHash,
        now: DateTime<Utc>,
        lifetime: chrono::Duration,
    ) -> DomainResult<Self> {
        if lifetime <= chrono::Duration::zero() {
            return Err(DomainError::validation("lifetime", "deve essere positiva"));
        }
        Ok(Self {
            id,
            user_id,
            token_hash,
            issued_at: now,
            expires_at: now + lifetime,
            revoked_at: None,
            rotated_to: None,
        })
    }

    #[must_use]
    pub const fn from_persistence(
        id: SessionId,
        user_id: UserId,
        token_hash: TokenHash,
        issued_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        revoked_at: Option<DateTime<Utc>>,
        rotated_to: Option<SessionId>,
    ) -> Self {
        Self {
            id,
            user_id,
            token_hash,
            issued_at,
            expires_at,
            revoked_at,
            rotated_to,
        }
    }

    /// Una sessione è utilizzabile se non è scaduta e non è stata revocata.
    ///
    /// Se è già stata ruotata, presentarla di nuovo significa che il token è
    /// stato replicato: è il segnale di furto e chi chiama deve revocare l'intera
    /// catena, non solo rifiutare la richiesta.
    pub fn ensure_usable(&self, now: DateTime<Utc>) -> DomainResult<()> {
        if self.rotated_to.is_some() {
            return Err(DomainError::invariant(
                "refresh token già ruotato: possibile riuso, revocare la catena",
            ));
        }
        if self.revoked_at.is_some() {
            return Err(DomainError::Forbidden("sessione revocata"));
        }
        if self.expires_at <= now {
            return Err(DomainError::Forbidden("sessione scaduta"));
        }
        Ok(())
    }

    #[must_use]
    pub const fn was_rotated(&self) -> bool {
        self.rotated_to.is_some()
    }

    pub const fn mark_rotated(&mut self, successor: SessionId, now: DateTime<Utc>) {
        self.rotated_to = Some(successor);
        self.revoked_at = Some(now);
    }

    pub const fn revoke(&mut self, now: DateTime<Utc>) {
        if self.revoked_at.is_none() {
            self.revoked_at = Some(now);
        }
    }

    #[must_use]
    pub const fn id(&self) -> SessionId {
        self.id
    }
    #[must_use]
    pub const fn user_id(&self) -> UserId {
        self.user_id
    }
    #[must_use]
    pub const fn token_hash(&self) -> TokenHash {
        self.token_hash
    }
    #[must_use]
    pub const fn issued_at(&self) -> DateTime<Utc> {
        self.issued_at
    }
    #[must_use]
    pub const fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
    #[must_use]
    pub const fn revoked_at(&self) -> Option<DateTime<Utc>> {
        self.revoked_at
    }
    #[must_use]
    pub const fn rotated_to(&self) -> Option<SessionId> {
        self.rotated_to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(now: DateTime<Utc>) -> RefreshSession {
        RefreshSession::issue(
            SessionId::generate(),
            UserId::generate(),
            TokenHash::from_bytes([7; 32]),
            now,
            chrono::Duration::days(30),
        )
        .expect("sessione valida")
    }

    #[test]
    fn fresh_session_is_usable() {
        let now = Utc::now();
        assert!(session(now).ensure_usable(now).is_ok());
    }

    #[test]
    fn expired_and_revoked_sessions_are_refused() {
        let now = Utc::now();
        let expired = session(now);
        assert!(
            expired
                .ensure_usable(now + chrono::Duration::days(31))
                .is_err()
        );

        let mut revoked = session(now);
        revoked.revoke(now);
        assert!(revoked.ensure_usable(now).is_err());
    }

    #[test]
    fn reuse_of_a_rotated_token_is_reported_as_invariant_violation() {
        let now = Utc::now();
        let mut rotated = session(now);
        rotated.mark_rotated(SessionId::generate(), now);
        let error = rotated.ensure_usable(now).expect_err("deve fallire");
        assert!(
            matches!(error, DomainError::Invariant(_)),
            "atteso Invariant, ottenuto {error:?}"
        );
    }

    #[test]
    fn token_hash_is_not_leaked_by_debug() {
        assert_eq!(
            format!("{:?}", TokenHash::from_bytes([1; 32])),
            "TokenHash(<redatto>)"
        );
    }
}
