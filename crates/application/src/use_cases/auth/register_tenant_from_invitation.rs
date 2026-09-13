use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, UserRegistered};
use propflow_domain::identity::User;
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::UserId;

use super::register_owner::map_email_conflict;
use super::{AuthenticatedSession, issue_session};
use crate::error::{AppError, AppResult, Conflict};
use crate::password_policy;
use crate::ports::{OpaqueTokenFactory, PasswordHasher, RefreshTokenFactory, TokenIssuer};
use crate::use_cases::invitation::link_accepted_invitation;

#[derive(Debug, Clone)]
pub struct RegisterTenantFromInvitationCommand {
    /// Token in chiaro dell'invito da inquilino. Non finisce nei log.
    pub token: String,
    pub full_name: String,
    pub password: String,
    // Nessun campo `email`: è quella dell'invito, non del corpo (ADR-053,
    // decisione 6). Il bordo HTTP la rifiuta se il corpo la porta.
}

/// Registra un inquilino **a partire dall'invito**, invece che in due passi
/// come oggi (`register` più `accept-invitation`): utente, membership, parte
/// contrattuale e invito marcato accettato nascono nella stessa transazione,
/// così non esiste un istante in cui l'inquilino è registrato e senza
/// organizzazione (ADR-053, decisione 1).
///
/// Gira sul pool privilegiato: prima della registrazione non c'è nessun
/// contesto di sicurezza da cui la Row Level Security possa dedurre una
/// visibilità (regola 27). **Condivide le tre scritture con
/// [`AcceptInvitation`](crate::use_cases::invitation::AcceptInvitation)**
/// attraverso [`link_accepted_invitation`], nello stesso ordine: qui il pool
/// privilegiato non è soggetto alla Row Level Security che quell'ordine
/// protegge in `AcceptInvitation`, ma un secondo ordine da spiegare
/// costerebbe più di quanto risparmierebbe.
///
/// **Non chiama `AcceptInvitation`**: quello legge sotto
/// `app.has_pending_invitation`, che confronta l'email dell'utente autenticato
/// con quella dell'invito — e qui non c'è ancora un utente autenticato che
/// possa soddisfarla.
///
/// **Tutti gli stati non validi del token rispondono allo stesso `401`**:
/// inesistente, scaduto, usato o revocato (ADR-053, decisione 6) — la stessa
/// forma di [`AcceptOwnerInvitation`](super::AcceptOwnerInvitation). L'unica
/// eccezione è un token da **proprietario**, pendente e valido: quello dà
/// `409 invitation.owner_token`, perché il client deve chiedere il nome
/// dell'organizzazione e rimandare a `accept-owner-invitation`.
pub struct RegisterTenantFromInvitation {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    hasher: Arc<dyn PasswordHasher>,
    tokens: Arc<dyn TokenIssuer>,
    refresh_tokens: Arc<dyn RefreshTokenFactory>,
    invitation_tokens: Arc<dyn OpaqueTokenFactory>,
    clock: Arc<dyn Clock>,
}

impl RegisterTenantFromInvitation {
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
        command: RegisterTenantFromInvitationCommand,
    ) -> AppResult<AuthenticatedSession> {
        let now = self.clock.now();
        let token_hash = self.invitation_tokens.hash(&command.token);

        let mut uow = self.unit_of_work.begin_privileged().await?;

        let Some(mut invitation) = uow.invitations().find_by_token_hash(&token_hash).await? else {
            // Non è un invito da inquilino. Potrebbe essere un invito da
            // proprietario, pendente e valido: in quel caso il client deve
            // chiedere il nome dell'organizzazione e rimandare a
            // `accept-owner-invitation`. **Solo** un token valido dà questo
            // `409`: uno inesistente, scaduto, usato o revocato — in
            // **entrambe** le tabelle — resta `401`, identico in ogni caso.
            let is_valid_owner_token = uow
                .owner_invitations()
                .find_pending_by_hash(&token_hash)
                .await?
                .is_some_and(|owner_invitation| owner_invitation.ensure_acceptable(now).is_ok());
            if is_valid_owner_token {
                return Err(AppError::Conflict(Conflict::OwnerInvitationToken));
            }
            return Err(AppError::Unauthenticated);
        };
        invitation
            .ensure_acceptable(now)
            .map_err(|_| AppError::Unauthenticated)?;

        password_policy::validate(&command.password)?;
        let password_hash = self
            .hasher
            .hash(&command.password)
            .await
            .map_err(AppError::internal)?;

        let user_id = UserId::generate();
        // L'email è quella dell'invito, non un campo del corpo: un invito è per
        // una persona, e chi vuole un altro indirizzo chiede un altro invito
        // (ADR-053, decisione 6).
        let user = User::register(
            user_id,
            invitation.email().clone(),
            &command.full_name,
            password_hash,
            now,
        )?;

        uow.users()
            .insert(&user)
            .await
            .map_err(|error| map_email_conflict(error.into()))?;

        let metadata = EventMetadata::from_context(context);
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::UserRegistered(UserRegistered {
                    user_id,
                    registered_at: now,
                }),
                now,
                metadata.clone(),
            ))
            .await?;

        link_accepted_invitation(&mut uow, &mut invitation, user_id, now, metadata).await?;

        let org_id = invitation.org_id();
        let role = invitation.role();
        let session = issue_session(
            &mut uow,
            &self.tokens,
            &self.refresh_tokens,
            &user,
            Some((org_id, role)),
            now,
        )
        .await?;

        uow.commit().await?;
        Ok(session)
    }
}
