use std::sync::Arc;

use chrono::Duration;
use propflow_domain::RequestContext;
use propflow_domain::identity::{OrgMember, OrgRole, RefreshSession, User};
use propflow_domain::ports::{Clock, UnitOfWork, UnitOfWorkFactory};
use propflow_domain::shared::{Email, SessionId};

use super::AuthenticatedSession;
use crate::error::{AppError, AppResult};
use crate::ports::{AccessTokenClaims, PasswordHasher, RefreshTokenFactory, TokenIssuer};

/// Durata del blocco dopo troppi tentativi falliti.
const LOCKOUT_DURATION_MINUTES: i64 = 15;

#[derive(Debug, Clone)]
pub struct LoginCommand {
    pub email: String,
    pub password: String,
}

/// Autenticazione con email e password.
///
/// Gira su transazione privilegiata: al momento del login non esiste ancora un
/// contesto di sicurezza, e la policy RLS su `users` mostra una riga solo a chi è
/// già identificato. Prima di autenticare non si può leggere l'utente da
/// autenticare.
pub struct Login {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    hasher: Arc<dyn PasswordHasher>,
    tokens: Arc<dyn TokenIssuer>,
    refresh_tokens: Arc<dyn RefreshTokenFactory>,
    clock: Arc<dyn Clock>,
}

impl Login {
    #[must_use]
    pub const fn new(
        unit_of_work: Arc<dyn UnitOfWorkFactory>,
        hasher: Arc<dyn PasswordHasher>,
        tokens: Arc<dyn TokenIssuer>,
        refresh_tokens: Arc<dyn RefreshTokenFactory>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            unit_of_work,
            hasher,
            tokens,
            refresh_tokens,
            clock,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        command: LoginCommand,
    ) -> AppResult<AuthenticatedSession> {
        // Un'email malformata è comunque un tentativo fallito, non un 422: dire
        // "email non valida" a chi prova indirizzi a caso è informazione gratuita.
        let Ok(email) = Email::parse(&command.email) else {
            return Err(AppError::Unauthenticated);
        };
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin_privileged().await?;
        let Some(mut user) = uow.users().find_by_email(&email).await? else {
            // Utente inesistente: si spende lo stesso tempo di una verifica reale,
            // altrimenti la latenza distingue gli account esistenti.
            let _ = self
                .hasher
                .verify(&command.password, self.hasher.dummy_hash())
                .await;
            return Err(AppError::Unauthenticated);
        };

        let stored_hash = match user.ensure_can_authenticate(now) {
            Ok(hash) => hash.clone(),
            Err(_) => {
                let _ = self
                    .hasher
                    .verify(&command.password, self.hasher.dummy_hash())
                    .await;
                return Err(AppError::Unauthenticated);
            }
        };

        let matches = self
            .hasher
            .verify(&command.password, &stored_hash)
            .await
            .map_err(AppError::internal)?;

        if !matches {
            user.record_failed_login(now, Duration::minutes(LOCKOUT_DURATION_MINUTES));
            uow.users().update(&user).await?;
            // Il contatore va persistito anche se la richiesta fallisce, altrimenti
            // il lockout non conta nulla.
            uow.commit().await?;
            return Err(AppError::Unauthenticated);
        }

        user.record_successful_login(now);
        uow.users().update(&user).await?;

        let default_membership = pick_default_membership(&mut uow, &user).await?;
        let session = self
            .issue_session(&mut uow, &user, default_membership.as_ref(), now)
            .await?;

        uow.commit().await?;
        let _ = context;
        Ok(session)
    }

    async fn issue_session(
        &self,
        uow: &mut Box<dyn UnitOfWork>,
        user: &User,
        membership: Option<&OrgMember>,
        now: chrono::DateTime<chrono::Utc>,
    ) -> AppResult<AuthenticatedSession> {
        let refresh = self.refresh_tokens.generate().map_err(AppError::internal)?;
        let refresh_session = RefreshSession::issue(
            SessionId::generate(),
            user.id(),
            refresh.hash,
            now,
            self.refresh_tokens.lifetime(),
        )?;
        uow.sessions().insert(&refresh_session).await?;

        let claims = AccessTokenClaims {
            user_id: user.id(),
            org_id: membership.map(OrgMember::org_id),
            org_role: membership.map(OrgMember::role),
            platform_role: user.platform_role(),
            expires_at: now + self.tokens.access_token_lifetime(),
        };
        let access_token = self.tokens.issue(&claims).map_err(AppError::internal)?;

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
}

/// Organizzazione predefinita all'accesso: si preferisce quella di cui l'utente è
/// owner, poi quella dove gestisce, infine la prima disponibile. Un inquilino con
/// una sola organizzazione non deve scegliere nulla.
pub(super) async fn pick_default_membership(
    uow: &mut Box<dyn UnitOfWork>,
    user: &User,
) -> AppResult<Option<OrgMember>> {
    let mut memberships: Vec<OrgMember> = uow
        .members()
        .list_for_user(user.id())
        .await?
        .into_iter()
        .filter(OrgMember::is_active)
        .collect();

    memberships.sort_by_key(|member| match member.role() {
        OrgRole::Owner => 0,
        OrgRole::Manager => 1,
        OrgRole::Tenant => 2,
    });
    Ok(memberships.into_iter().next())
}
