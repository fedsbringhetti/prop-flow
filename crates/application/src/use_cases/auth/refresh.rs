use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::identity::RefreshSession;
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::SessionId;

use super::{AuthenticatedSession, login::pick_default_membership};
use crate::error::{AppError, AppResult};
use crate::ports::{AccessTokenClaims, RefreshTokenFactory, TokenIssuer};

#[derive(Debug, Clone)]
pub struct RefreshSessionCommand {
    pub refresh_token: String,
}

/// Rinnova la sessione ruotando il refresh token.
///
/// Rotazione a uso singolo: ogni refresh consuma il token presentato ed emette
/// uno nuovo. Se un token già ruotato viene presentato di nuovo, qualcuno ne ha
/// una copia — non si sa chi sia legittimo, quindi cadono tutte le sessioni
/// dell'utente.
pub struct RefreshSessionUseCase {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    tokens: Arc<dyn TokenIssuer>,
    refresh_tokens: Arc<dyn RefreshTokenFactory>,
    clock: Arc<dyn Clock>,
}

impl RefreshSessionUseCase {
    #[must_use]
    pub const fn new(
        unit_of_work: Arc<dyn UnitOfWorkFactory>,
        tokens: Arc<dyn TokenIssuer>,
        refresh_tokens: Arc<dyn RefreshTokenFactory>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            unit_of_work,
            tokens,
            refresh_tokens,
            clock,
        }
    }

    pub async fn execute(
        &self,
        _context: &RequestContext,
        command: RefreshSessionCommand,
    ) -> AppResult<AuthenticatedSession> {
        let presented_hash = self.refresh_tokens.hash(&command.refresh_token);
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin_privileged().await?;
        let Some(mut current) = uow.sessions().find_by_token_hash(&presented_hash).await? else {
            return Err(AppError::Unauthenticated);
        };

        if current.was_rotated() {
            let revoked = uow
                .sessions()
                .revoke_all_for_user(current.user_id())
                .await?;
            uow.commit().await?;
            tracing::warn!(
                user_id = %current.user_id(),
                revoked_sessions = revoked,
                "riuso di un refresh token già ruotato: revocate tutte le sessioni"
            );
            return Err(AppError::Unauthenticated);
        }
        if current.ensure_usable(now).is_err() {
            return Err(AppError::Unauthenticated);
        }

        let Some(user) = uow.users().find_by_id(current.user_id()).await? else {
            return Err(AppError::Unauthenticated);
        };
        if user.ensure_can_authenticate(now).is_err() {
            return Err(AppError::Unauthenticated);
        }

        let successor_id = SessionId::generate();
        let refresh = self.refresh_tokens.generate().map_err(AppError::internal)?;
        let successor = RefreshSession::issue(
            successor_id,
            user.id(),
            refresh.hash,
            now,
            self.refresh_tokens.lifetime(),
        )?;

        // Prima il successore, poi il puntatore: `user_sessions.rotated_to` è una
        // foreign key sulla tabella stessa, quindi aggiornare `current` prima che
        // il successore esista viola il vincolo.
        uow.sessions().insert(&successor).await?;
        current.mark_rotated(successor_id, now);
        uow.sessions().update(&current).await?;

        let membership = pick_default_membership(&mut uow, &user).await?;
        let claims = AccessTokenClaims {
            user_id: user.id(),
            org_id: membership.as_ref().map(|member| member.org_id()),
            org_role: membership.as_ref().map(|member| member.role()),
            platform_role: user.platform_role(),
            expires_at: now + self.tokens.access_token_lifetime(),
        };
        let access_token = self.tokens.issue(&claims).map_err(AppError::internal)?;

        uow.commit().await?;

        Ok(AuthenticatedSession {
            user_id: user.id(),
            access_token,
            access_token_expires_at: claims.expires_at,
            refresh_token: refresh.plaintext,
            refresh_token_expires_at: successor.expires_at(),
            org_id: claims.org_id,
            org_role: claims.org_role,
        })
    }
}
