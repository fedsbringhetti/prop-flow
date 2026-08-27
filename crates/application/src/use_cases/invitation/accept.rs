use std::sync::Arc;

use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, InvitationAccepted};
use propflow_domain::identity::{OrgMember, OrgRole};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::{LeaseId, OrgId};
use propflow_domain::{DomainError, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult, Conflict};
use crate::ports::OpaqueTokenFactory;

#[derive(Debug, Clone)]
pub struct AcceptInvitationCommand {
    /// Token in chiaro ricevuto dall'invitato. Non finisce nei log.
    pub token: String,
}

#[derive(Debug, Clone, Copy)]
pub struct AcceptedInvitation {
    pub org_id: OrgId,
    pub role: OrgRole,
    pub lease_id: Option<LeaseId>,
}

/// Accetta un invito: crea la membership, collega la parte contrattuale e marca
/// l'invito, **in una sola transazione**.
///
/// Non usa il pool privilegiato. La regola 20 riserva `begin_privileged()` ai
/// flussi in cui un contesto di sicurezza non *può* esistere, perché avvengono
/// prima dell'autenticazione; qui il contesto esiste — chi accetta è autenticato —
/// e manca solo la membership. La risposta corretta è estendere la Row Level
/// Security, ed è quello che fa la migration 0018 (ADR-020).
///
/// Sono due controlli indipendenti: l'applicazione prova che chi chiama **possiede**
/// il token, confrontandone lo hash; le policy provano che esiste davvero un invito
/// pendente per quell'email. Nessuno dei due da solo basterebbe.
///
/// L'ordine delle tre scritture non è arbitrario: `app.has_pending_invitation`
/// abilita le prime due, e diventa falso appena l'invito è marcato accettato.
/// Marcarlo per primo si toglierebbe il permesso di completare il resto.
pub struct AcceptInvitation {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    tokens: Arc<dyn OpaqueTokenFactory>,
    clock: Arc<dyn Clock>,
}

impl AcceptInvitation {
    #[must_use]
    pub const fn new(
        unit_of_work: Arc<dyn UnitOfWorkFactory>,
        tokens: Arc<dyn OpaqueTokenFactory>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            unit_of_work,
            tokens,
            clock,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        command: AcceptInvitationCommand,
    ) -> AppResult<AcceptedInvitation> {
        // `ensure_authenticated` e non `ensure_member`: l'intero senso
        // dell'operazione è che il chiamante non è ancora membro.
        authorization::ensure_authenticated(context)?;
        let user_id = context.require_user()?;
        let now = self.clock.now();
        let token_hash = self.tokens.hash(&command.token);

        let mut uow = self.unit_of_work.begin(context).await?;

        // Un token sconosciuto e un token valido per un altro utente danno la
        // stessa risposta: distinguerli sarebbe un oracolo per capire quali inviti
        // esistono (ADR-017).
        let mut invitation = uow
            .invitations()
            .find_by_token_hash(&token_hash)
            .await?
            .ok_or(AppError::Unauthenticated)?;

        invitation
            .ensure_acceptable(now)
            .map_err(|error| match error {
                // Scaduto o revocato: è una credenziale che non vale più, come un
                // refresh token oltre la scadenza. "Già accettato" invece è uno stato,
                // e chi possiede il token merita di saperlo.
                DomainError::Forbidden(_) => AppError::Unauthenticated,
                other => AppError::Validation(other),
            })?;

        let user = uow
            .users()
            .find_by_id(user_id)
            .await?
            .ok_or(AppError::Unauthenticated)?;
        // Le policy imporrebbero comunque la corrispondenza, ma con un rifiuto della
        // RLS invece di una risposta comprensibile: il service layer serve alla
        // semantica, la RLS all'autorità (ADR-016).
        if user.email() != invitation.email() {
            return Err(AppError::Unauthenticated);
        }

        let org_id = invitation.org_id();
        let membership = OrgMember::active(org_id, user_id, invitation.role(), now);
        uow.members()
            .insert(&membership)
            .await
            .map_err(|error| map_membership_conflict(error.into()))?;

        // Collegare la parte contrattuale è ciò che rende visibile il contratto:
        // `app.can_access_lease` passa da `lease_parties.user_id`.
        if let Some(lease_id) = invitation.lease_id() {
            let parties = uow.lease_parties().list_for_lease(lease_id).await?;
            let mut party = parties
                .into_iter()
                .find(|party| party.email() == Some(invitation.email()))
                .ok_or_else(|| {
                    // L'invito nomina un contratto in cui non esiste una riga con
                    // questa email: nessuna scrittura, e un errore interno, perché
                    // è `InviteTenant` a doverla avere creata.
                    AppError::internal(
                        "invito legato a un contratto senza la corrispondente parte contrattuale",
                    )
                })?;
            party.link_user(user_id, now)?;
            uow.lease_parties().update(&party).await?;
        }

        invitation.accept(user_id, now)?;
        uow.invitations().update(&invitation).await?;

        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::InvitationAccepted(InvitationAccepted {
                    invitation_id: invitation.id(),
                    org_id,
                    user_id,
                    lease_id: invitation.lease_id(),
                    role: invitation.role().as_str(),
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        // Le tre scritture e l'evento vivono o cadono insieme: una membership senza
        // il collegamento al contratto darebbe un inquilino che non vede nulla, e un
        // invito marcato senza membership lo lascerebbe fuori per sempre.
        uow.commit().await?;

        Ok(AcceptedInvitation {
            org_id,
            role: invitation.role(),
            lease_id: invitation.lease_id(),
        })
    }
}

/// Chi è già membro dell'organizzazione non ha bisogno di questo invito.
fn map_membership_conflict(error: AppError) -> AppError {
    match &error {
        AppError::Integrity { details } if details.contains("org_members_org_user") => {
            AppError::Conflict(Conflict::MembershipAlreadyExists)
        }
        _ => error,
    }
}
