use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::events::{
    DomainEvent, DomainEventPayload, EventMetadata, OrganizationCreated, UserRegistered,
};
use propflow_domain::identity::{OrgMember, OrgRole, Organization, User};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::{Email, OrgId, UserId};

use crate::error::{AppError, AppResult};
use crate::password_policy;
use crate::ports::PasswordHasher;

#[derive(Debug, Clone)]
pub struct RegisterOwnerCommand {
    pub email: String,
    pub full_name: String,
    pub password: String,
    /// Nome dell'organizzazione personale. Se assente si usa il nome dell'utente.
    pub organization_name: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct RegisterOwnerResult {
    pub user_id: UserId,
    pub org_id: OrgId,
}

/// Registra un proprietario e la sua organizzazione personale.
///
/// È l'unico use case che apre una transazione privilegiata: crea utente,
/// organizzazione e membership quando nessuna delle tre esiste ancora, quindi non
/// esiste un contesto di sicurezza da cui la Row Level Security possa dedurre una
/// visibilità. Tutto avviene in una sola transazione: un utente senza
/// organizzazione non potrebbe fare nulla, e un'organizzazione senza owner non
/// sarebbe amministrabile da nessuno.
pub struct RegisterOwner {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    hasher: Arc<dyn PasswordHasher>,
    clock: Arc<dyn Clock>,
}

impl RegisterOwner {
    #[must_use]
    pub const fn new(
        unit_of_work: Arc<dyn UnitOfWorkFactory>,
        hasher: Arc<dyn PasswordHasher>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            unit_of_work,
            hasher,
            clock,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        command: RegisterOwnerCommand,
    ) -> AppResult<RegisterOwnerResult> {
        let email = Email::parse(&command.email)?;
        password_policy::validate(&command.password)?;

        let now = self.clock.now();
        let password_hash = self
            .hasher
            .hash(&command.password)
            .await
            .map_err(AppError::internal)?;

        let user_id = UserId::generate();
        let user = User::register(user_id, email, &command.full_name, password_hash, now)?;

        let org_id = OrgId::generate();
        let org_name = command
            .organization_name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(user.full_name());
        let organization = Organization::personal(org_id, org_name, now)?;
        let membership = OrgMember::active(org_id, user_id, OrgRole::Owner, now);

        let mut uow = self.unit_of_work.begin_privileged().await?;

        // Il controllo esplicito dà un 409 parlante; il vincolo unique sul
        // database resta l'autorità in caso di richieste concorrenti.
        if uow.users().find_by_email(user.email()).await?.is_some() {
            return Err(AppError::Conflict {
                resource: "utente con questa email",
            });
        }

        uow.users()
            .insert(&user)
            .await
            .map_err(|error| map_email_conflict(error.into()))?;
        uow.organizations().insert(&organization).await?;
        uow.members().insert(&membership).await?;

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
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::OrganizationCreated(OrganizationCreated {
                    org_id,
                    owner_id: user_id,
                    kind: organization.kind().as_str(),
                }),
                now,
                metadata,
            ))
            .await?;

        uow.commit().await?;

        Ok(RegisterOwnerResult { user_id, org_id })
    }
}

/// Una violazione di unicità sull'email è un conflitto comprensibile, non un 500.
fn map_email_conflict(error: AppError) -> AppError {
    match &error {
        AppError::Integrity { details } if details.contains("users_email") => AppError::Conflict {
            resource: "utente con questa email",
        },
        _ => error,
    }
}
