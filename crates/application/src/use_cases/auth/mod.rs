mod login;
mod logout;
mod password_reset;
mod refresh;
mod register_owner;

pub use login::{Login, LoginCommand};
pub use logout::{Logout, LogoutCommand};
pub use password_reset::{
    ConfirmPasswordReset, ConfirmPasswordResetCommand, RequestPasswordReset,
    RequestPasswordResetCommand,
};
pub use refresh::{RefreshSessionCommand, RefreshSessionUseCase};
pub use register_owner::{RegisterOwner, RegisterOwnerCommand, RegisterOwnerResult};

use chrono::{DateTime, Utc};
use propflow_domain::identity::OrgRole;
use propflow_domain::shared::{OrgId, UserId};

/// Esito di un'autenticazione riuscita.
///
/// Il refresh token in chiaro appare qui e solo qui: in database ne resta
/// l'hash, e questa struct vive il tempo di una risposta HTTP.
#[derive(Debug, Clone)]
pub struct AuthenticatedSession {
    pub user_id: UserId,
    pub access_token: String,
    pub access_token_expires_at: DateTime<Utc>,
    pub refresh_token: String,
    pub refresh_token_expires_at: DateTime<Utc>,
    pub org_id: Option<OrgId>,
    pub org_role: Option<OrgRole>,
}

impl AuthenticatedSession {
    /// Secondi residui dell'access token, mai negativi: il campo `expires_in`
    /// di una risposta OAuth non ammette valori negativi.
    #[must_use]
    pub fn expires_in_seconds(&self, now: DateTime<Utc>) -> i64 {
        seconds_until(self.access_token_expires_at, now)
    }
}

fn seconds_until(target: DateTime<Utc>, now: DateTime<Utc>) -> i64 {
    (target - now).num_seconds().max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiry_never_goes_negative() {
        let now = Utc::now();
        assert_eq!(seconds_until(now - chrono::Duration::hours(1), now), 0);
        assert_eq!(seconds_until(now + chrono::Duration::seconds(90), now), 90);
    }
}
