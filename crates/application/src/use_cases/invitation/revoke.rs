use std::sync::Arc;

use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, InvitationRevoked};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::InvitationId;
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy)]
pub struct RevokeInvitationCommand {
    pub id: InvitationId,
}

/// Revoca un invito non ancora accettato.
///
/// L'invito resta in database: la revoca è uno stato, non una cancellazione, e
/// l'audit deve poter dire chi ha invitato chi e quando ha cambiato idea.
pub struct RevokeInvitation {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl RevokeInvitation {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>, clock: Arc<dyn Clock>) -> Self {
        Self {
            unit_of_work,
            clock,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        command: RevokeInvitationCommand,
    ) -> AppResult<()> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut invitation = uow
            .invitations()
            .find_by_id(command.id)
            .await?
            .ok_or(AppError::not_found(Entity::Invitation))?;

        invitation.revoke(now)?;
        uow.invitations().update(&invitation).await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::InvitationRevoked(InvitationRevoked {
                    invitation_id: invitation.id(),
                    org_id,
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(())
    }
}
