mod accept_owner_invitation;
mod login;
mod logout;
mod password_reset;
mod refresh;
mod register_owner;
mod register_tenant_from_invitation;

pub use accept_owner_invitation::{AcceptOwnerInvitation, AcceptOwnerInvitationCommand};
pub use login::{Login, LoginCommand};
pub use logout::{Logout, LogoutCommand};
pub use password_reset::{
    ConfirmPasswordReset, ConfirmPasswordResetCommand, RequestPasswordReset,
    RequestPasswordResetCommand,
};
pub use refresh::{RefreshSessionCommand, RefreshSessionUseCase};
pub use register_tenant_from_invitation::{
    RegisterTenantFromInvitation, RegisterTenantFromInvitationCommand,
};

use std::sync::Arc;

use chrono::{DateTime, Utc};
use propflow_domain::identity::{OrgRole, RefreshSession, User};
use propflow_domain::ports::UnitOfWork;
use propflow_domain::shared::{OrgId, SessionId, UserId};

use crate::error::{AppError, AppResult};
use crate::ports::{AccessTokenClaims, RefreshTokenFactory, TokenIssuer};

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

/// Emette refresh token, riga di sessione e access token: **lo stesso esito per
/// login, accettazione di un invito da proprietario e registrazione da invito
/// d'inquilino** (ADR-053). Prima di questa fetta era un metodo privato di
/// [`Login`]; le due rotte nuove rispondono `201` con la stessa sessione e lo
/// stesso cookie del login, quindi la scrittura è condivisa qui invece che
/// duplicata tre volte — è la stessa ragione per cui `session_reply` al bordo
/// HTTP è una funzione sola per login e refresh.
///
/// `membership` porta `(OrgId, OrgRole)` e non un `&OrgMember`: i tre chiamanti
/// non hanno sempre in mano un `OrgMember` (chi accetta un invito da
/// proprietario ne costruisce uno appena inserito, non lo rilegge), e i soli
/// due campi che contano per i claim sono questi.
pub(super) async fn issue_session(
    uow: &mut Box<dyn UnitOfWork>,
    tokens: &Arc<dyn TokenIssuer>,
    refresh_tokens: &Arc<dyn RefreshTokenFactory>,
    user: &User,
    membership: Option<(OrgId, OrgRole)>,
    now: DateTime<Utc>,
) -> AppResult<AuthenticatedSession> {
    let refresh = refresh_tokens.generate().map_err(AppError::internal)?;
    let refresh_session = RefreshSession::issue(
        SessionId::generate(),
        user.id(),
        refresh.hash,
        now,
        refresh_tokens.lifetime(),
    )?;
    uow.sessions().insert(&refresh_session).await?;

    let claims = AccessTokenClaims {
        user_id: user.id(),
        org_id: membership.map(|(org_id, _)| org_id),
        org_role: membership.map(|(_, role)| role),
        platform_role: user.platform_role(),
        expires_at: now + tokens.access_token_lifetime(),
    };
    let access_token = tokens.issue(&claims).map_err(AppError::internal)?;

    Ok(AuthenticatedSession {
        user_id: user.id(),
        access_token,
        access_token_expires_at: claims.expires_at,
        refresh_token: refresh.plaintext,
        refresh_token_expires_at: refresh_session.expires_at(),
        org_id: claims.org_id,
        org_role: claims.org_role,
    })
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
