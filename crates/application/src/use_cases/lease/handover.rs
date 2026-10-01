use std::sync::Arc;

use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, LeaseHandedOver};
use propflow_domain::lease::{Handover, JoiningParty};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::{Email, LeaseId, LeasePartyId};
use propflow_domain::{Entity, RequestContext};
use rust_decimal::Decimal;

use crate::authorization;
use crate::error::{AppError, AppResult};
use crate::ports::{Notifier, OpaqueTokenFactory};
use crate::use_cases::invitation::{
    IssuedInvitation, TenantInvitationTarget, deliver_tenant_invitation, issue_tenant_invitation,
};

use super::LeaseDetail;

#[derive(Debug, Clone)]
pub struct HandoverLeaseCommand {
    pub lease_id: LeaseId,
    pub leaving_party_id: LeasePartyId,
    pub joining_full_name: String,
    pub joining_email: Email,
    pub joining_share_percent: Decimal,
    /// Le quote di chi resta. Chi non compare non cambia.
    pub shares: Vec<(LeasePartyId, Decimal)>,
}

/// Il contratto dopo il subentro, e l'invito per chi entra.
#[derive(Debug, Clone)]
pub struct HandedOverLease {
    pub detail: LeaseDetail,
    pub invitation: IssuedInvitation,
}

/// Il subentro: la parte che esce lascia oggi il suo posto a una nuova, che riceve
/// l'invito nella stessa transazione (ADR-056).
///
/// **Non passa da `AddLeaseParty`**, che rifiuta su un contratto attivo, e deve
/// continuare a farlo: aggiungere una parte resta vietato, e il subentro non è
/// un'aggiunta. **Non passa nemmeno da `InviteTenant`**, che creerebbe una seconda
/// parte con `remaining_share`: la parte nuova la costruisce il dominio, con la
/// quota proposta e l'intestazione di chi esce.
///
/// Tutto si calcola e si verifica in [`propflow_domain::lease::Lease::hand_over`]
/// **prima** di scrivere; qui resta l'ordine delle scritture, che è una delle
/// trappole: chi esce per primo, perché ha appena perso l'intestazione che chi
/// entra riceve, e `lease_parties_primary_uk` non guarda `left_at`.
pub struct HandoverLease {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    tokens: Arc<dyn OpaqueTokenFactory>,
    clock: Arc<dyn Clock>,
    notifier: Arc<dyn Notifier>,
}

impl HandoverLease {
    #[must_use]
    pub const fn new(
        unit_of_work: Arc<dyn UnitOfWorkFactory>,
        tokens: Arc<dyn OpaqueTokenFactory>,
        clock: Arc<dyn Clock>,
        notifier: Arc<dyn Notifier>,
    ) -> Self {
        Self {
            unit_of_work,
            tokens,
            clock,
            notifier,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        command: HandoverLeaseCommand,
    ) -> AppResult<HandedOverLease> {
        let org_id = authorization::ensure_staff(context)?;
        let invited_by = context.require_user()?;
        let now = self.clock.now();
        // Il giorno dello stesso orologio che scrive `joined_at` all'accettazione:
        // è ciò che rende `left_at >= joined_at` vero per costruzione. È il giorno
        // UTC, con la finestra notturna di R54.
        let today = now.date_naive();

        let material = self.tokens.generate().map_err(AppError::internal)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        // Serve all'email dell'invito, come in `InviteTenant`.
        let organization = uow
            .organizations()
            .find_by_id(org_id)
            .await?
            .ok_or_else(|| {
                AppError::internal("organizzazione non leggibile da chi la amministra")
            })?;
        let lease = uow
            .leases()
            .find_by_id(command.lease_id)
            .await?
            .ok_or(AppError::not_found(Entity::Lease))?;
        let parties = uow.lease_parties().list_for_lease(lease.id()).await?;

        let joining_email = command.joining_email.clone();
        let outcome = lease.hand_over(
            &parties,
            Handover {
                leaving_party_id: command.leaving_party_id,
                joining: JoiningParty {
                    id: LeasePartyId::generate(),
                    full_name: command.joining_full_name,
                    email: command.joining_email,
                    share_percent: command.joining_share_percent,
                },
                shares: command.shares,
            },
            today,
            now,
        )?;

        uow.lease_parties().update(&outcome.left).await?;
        for party in &outcome.reshared {
            uow.lease_parties().update(party).await?;
        }
        uow.lease_parties().insert(&outcome.joined).await?;

        let invitation = issue_tenant_invitation(
            uow.as_mut(),
            material,
            TenantInvitationTarget {
                org_id,
                lease_id: lease.id(),
                email: joining_email,
                invited_by,
            },
            context,
            now,
        )
        .await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::LeaseHandedOver(LeaseHandedOver {
                    lease_id: lease.id(),
                    org_id,
                    leaving_party_id: outcome.left.id(),
                    joining_party_id: outcome.joined.id(),
                    effective_date: today,
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;

        // Riletti nella stessa transazione: la risposta è ciò che il database ha
        // accettato, non ciò che il dominio ha calcolato.
        let parties = uow.lease_parties().list_for_lease(lease.id()).await?;
        let labels = uow.leases().labels(&[lease.id()]).await?.into_iter().next();
        uow.commit().await?;

        deliver_tenant_invitation(self.notifier.as_ref(), &invitation, organization.name()).await;

        Ok(HandedOverLease {
            detail: LeaseDetail {
                effective_status: lease.effective_status(today),
                lease,
                parties,
                labels,
            },
            invitation,
        })
    }
}
