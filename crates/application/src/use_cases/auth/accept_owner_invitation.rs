use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::identity::OrgRole;
use propflow_domain::ports::{Clock, UnitOfWorkFactory};

use super::register_owner::create_owner_account;
use super::{AuthenticatedSession, issue_session};
use crate::error::{AppError, AppResult};
use crate::ports::{OpaqueTokenFactory, PasswordHasher, RefreshTokenFactory, TokenIssuer};

#[derive(Debug, Clone)]
pub struct AcceptOwnerInvitationCommand {
    /// Token in chiaro emesso da `ops/dev/issue-owner-invitation.sh`. Non
    /// finisce nei log.
    pub token: String,
    pub full_name: String,
    pub password: String,
    /// Nome dell'organizzazione personale. Se assente si usa il nome
    /// dell'utente, come in [`RegisterOwner`](super::RegisterOwner).
    pub organization_name: Option<String>,
}

/// Accetta un invito da proprietario: crea utente e organizzazione personale
/// come [`RegisterOwner`](super::RegisterOwner) fa oggi per chiunque, ma dietro
/// un token invece che su una rotta aperta (ADR-053).
///
/// Gira sul pool privilegiato per **due** ragioni, non una sola: non solo
/// perché utente e organizzazione non esistono ancora (regola 27), ma perché
/// la prova di possesso del token è tutta dell'applicazione — hash e stato —
/// senza una Row Level Security a fare da seconda guardia, a differenza
/// dell'accettazione di un invito da inquilino dove il contesto autenticato
/// esiste già ([ADR-020](../../../../../docs/08-decisioni.md)).
///
/// **Tutti gli stati non validi rispondono allo stesso `401`**: token
/// inesistente, scaduto, revocato o già accettato. È la decisione 6 di
/// [ADR-053](../../../../../docs/08-decisioni.md) — vedi anche il commento su
/// [`OwnerInvitation::ensure_acceptable`](propflow_domain::identity::OwnerInvitation::ensure_acceptable):
/// distinguerli sarebbe un oracolo su quali token sono stati emessi.
pub struct AcceptOwnerInvitation {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    hasher: Arc<dyn PasswordHasher>,
    tokens: Arc<dyn TokenIssuer>,
    refresh_tokens: Arc<dyn RefreshTokenFactory>,
    invitation_tokens: Arc<dyn OpaqueTokenFactory>,
    clock: Arc<dyn Clock>,
}

impl AcceptOwnerInvitation {
    #[must_use]
    pub const fn new(
        unit_of_work: Arc<dyn UnitOfWorkFactory>,
        hasher: Arc<dyn PasswordHasher>,
        tokens: Arc<dyn TokenIssuer>,
        refresh_tokens: Arc<dyn RefreshTokenFactory>,
        invitation_tokens: Arc<dyn OpaqueTokenFactory>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            unit_of_work,
            hasher,
            tokens,
            refresh_tokens,
            invitation_tokens,
            clock,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        command: AcceptOwnerInvitationCommand,
    ) -> AppResult<AuthenticatedSession> {
        let now = self.clock.now();
        let token_hash = self.invitation_tokens.hash(&command.token);

        let mut uow = self.unit_of_work.begin_privileged().await?;

        let Some(mut invitation) = uow
            .owner_invitations()
            .find_pending_by_hash(&token_hash)
            .await?
        else {
            return Err(AppError::Unauthenticated);
        };
        // Un solo esito per scaduto, revocato e già accettato: vedi la decisione
        // 6 dell'ADR-053 e il commento su `OwnerInvitation::ensure_acceptable`.
        if invitation.ensure_acceptable(now).is_err() {
            return Err(AppError::Unauthenticated);
        }

        let email = invitation.email().clone();
        let account = create_owner_account(
            &mut uow,
            &self.hasher,
            &self.clock,
            context,
            email,
            &command.full_name,
            &command.password,
            command.organization_name.as_deref(),
        )
        .await?;

        // Riverificato per la stessa ragione per cui `AcceptInvitation` lo fa:
        // `accept` è l'unico punto che scrive `accepted_at`/`accepted_by`, e fra
        // la lettura sopra e qui non cambia nulla che `ensure_acceptable` non
        // abbia già guardato — ma un secondo controllo costa nulla ed evita di
        // scrivere due volte la stessa condizione in due punti diversi.
        invitation
            .accept(account.user.id(), now)
            .map_err(|_| AppError::Unauthenticated)?;
        uow.owner_invitations().mark_accepted(&invitation).await?;

        let session = issue_session(
            &mut uow,
            &self.tokens,
            &self.refresh_tokens,
            &account.user,
            Some((account.org_id, OrgRole::Owner)),
            now,
        )
        .await?;

        uow.commit().await?;
        Ok(session)
    }
}
