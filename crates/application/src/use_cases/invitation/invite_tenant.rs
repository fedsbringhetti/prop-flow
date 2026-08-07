use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, TenantInvited};
use propflow_domain::identity::{Invitation, NewInvitation, OrgRole};
use propflow_domain::lease::{LeaseParty, LeasePartyRole, NewLeaseParty};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::{Email, InvitationId, LeaseId, LeasePartyId};
use rust_decimal::Decimal;

use crate::authorization;
use crate::error::{AppError, AppResult};
use crate::ports::OpaqueTokenFactory;

#[derive(Debug, Clone)]
pub struct InviteTenantCommand {
    pub lease_id: LeaseId,
    pub email: Email,
    /// Usato solo se la parte contrattuale con questa email non esiste ancora.
    pub full_name: String,
}

/// Invito emesso. Il token in chiaro è qui e da nessun'altra parte.
#[derive(Debug, Clone)]
pub struct IssuedInvitation {
    pub invitation: Invitation,
    /// Da consegnare all'invitato **una sola volta**: in database ne resta lo
    /// hash, come per i refresh token. Non va registrato nei log.
    pub token: String,
}

/// Invita un inquilino su un contratto.
///
/// L'invio dell'email non avviene qui: la porta [`Notifier`](crate::ports::Notifier)
/// è dichiarata e non implementata, quindi il token torna nella risposta e il
/// proprietario lo consegna come preferisce. È deliberato: consegnare il token in
/// due modi diversi a seconda che il fornitore di posta risponda o no avrebbe reso
/// il comportamento dipendente da un servizio esterno.
pub struct InviteTenant {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    tokens: Arc<dyn OpaqueTokenFactory>,
    clock: Arc<dyn Clock>,
}

impl InviteTenant {
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
        command: InviteTenantCommand,
    ) -> AppResult<IssuedInvitation> {
        let org_id = authorization::ensure_staff(context)?;
        let invited_by = context.require_user()?;
        let now = self.clock.now();

        let material = self.tokens.generate().map_err(AppError::internal)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        let lease = uow
            .leases()
            .find_by_id(command.lease_id)
            .await?
            .ok_or(AppError::not_found("contratto"))?;

        // La parte contrattuale deve esistere prima dell'accettazione: è la riga
        // che l'invitato collegherà a sé, e senza di essa accettare non lo
        // renderebbe parte di nulla.
        let parties = uow.lease_parties().list_for_lease(lease.id()).await?;
        if !parties
            .iter()
            .any(|party| party.email() == Some(&command.email))
        {
            let party = LeaseParty::new(
                NewLeaseParty {
                    id: LeasePartyId::generate(),
                    org_id,
                    lease_id: lease.id(),
                    role: LeasePartyRole::Tenant,
                    full_name: command.full_name,
                    email: Some(command.email.clone()),
                    share_percent: remaining_share(&parties),
                    is_primary: parties.is_empty(),
                },
                now,
            )?;
            uow.lease_parties().insert(&party).await?;
        }

        let invitation = Invitation::issue(
            NewInvitation {
                id: InvitationId::generate(),
                org_id,
                email: command.email,
                role: OrgRole::Tenant,
                token_hash: material.hash,
                lease_id: Some(lease.id()),
                invited_by: Some(invited_by),
            },
            now,
        )?;

        uow.invitations()
            .insert(&invitation)
            .await
            .map_err(|error| map_pending_conflict(error.into()))?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::TenantInvited(TenantInvited {
                    invitation_id: invitation.id(),
                    org_id,
                    lease_id: invitation.lease_id(),
                    role: invitation.role().as_str(),
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(IssuedInvitation {
            invitation,
            token: material.plaintext,
        })
    }
}

/// Quota residua da assegnare a una parte creata dall'invito.
///
/// L'attivazione richiede che le quote degli inquilini sommino a 100, quindi il
/// residuo è l'unico valore che non rompe l'invariante appena assegnato. Se le
/// quote sono già complete la nuova parte nasce a zero, e chi amministra la
/// correggerà: meglio una quota visibilmente sbagliata che un contratto non più
/// attivabile senza capire perché.
fn remaining_share(parties: &[LeaseParty]) -> Decimal {
    let assigned: Decimal = parties
        .iter()
        .filter(|party| party.role().shares_rent() && party.left_at().is_none())
        .map(LeaseParty::share_percent)
        .sum();
    (Decimal::from(100) - assigned).max(Decimal::ZERO)
}

/// Il vincolo `org_invitations_pending_uk` ammette un solo invito pendente per
/// email e organizzazione. Tradurlo evita che un doppio invito esca come "vincolo
/// violato" senza dire quale.
fn map_pending_conflict(error: AppError) -> AppError {
    match &error {
        AppError::Integrity { details } if details.contains("org_invitations_pending") => {
            AppError::Conflict {
                resource: "invito pendente per questa email",
            }
        }
        _ => error,
    }
}
