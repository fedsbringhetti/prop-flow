use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::ports::{Clock, UnitOfWorkFactory};

use crate::authorization;
use crate::error::AppResult;
use crate::ports::RefreshTokenFactory;

#[derive(Debug, Clone)]
pub struct LogoutCommand {
    pub refresh_token: String,
}

/// Revoca la sessione di refresh presentata.
///
/// A differenza di login e refresh, qui il chiamante è già autenticato: la
/// transazione passa dal contesto normale, e la policy RLS su `user_sessions`
/// (`user_id = app.current_user_id()`) impedisce di revocare la sessione di
/// qualcun altro anche presentandone l'hash corretto.
pub struct Logout {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    refresh_tokens: Arc<dyn RefreshTokenFactory>,
    clock: Arc<dyn Clock>,
}

impl Logout {
    #[must_use]
    pub const fn new(
        unit_of_work: Arc<dyn UnitOfWorkFactory>,
        refresh_tokens: Arc<dyn RefreshTokenFactory>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            unit_of_work,
            refresh_tokens,
            clock,
        }
    }

    pub async fn execute(&self, context: &RequestContext, command: LogoutCommand) -> AppResult<()> {
        authorization::ensure_authenticated(context)?;
        let presented_hash = self.refresh_tokens.hash(&command.refresh_token);
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        // Un token già inesistente o non proprio non è un errore: il logout è
        // idempotente, e segnalare la differenza rivelerebbe l'esistenza di
        // sessioni altrui.
        if let Some(mut session) = uow.sessions().find_by_token_hash(&presented_hash).await? {
            session.revoke(now);
            uow.sessions().update(&session).await?;
        }
        uow.commit().await?;
        Ok(())
    }
}
