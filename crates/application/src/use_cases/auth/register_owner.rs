use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::events::{
    DomainEvent, DomainEventPayload, EventMetadata, OrganizationCreated, UserRegistered,
};
use propflow_domain::identity::{OrgMember, OrgRole, Organization, User};
use propflow_domain::ports::{Clock, UnitOfWork, UnitOfWorkFactory};
use propflow_domain::shared::{Email, OrgId, UserId};

use crate::error::{AppError, AppResult, Conflict};
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

        let mut uow = self.unit_of_work.begin_privileged().await?;
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
        uow.commit().await?;

        Ok(RegisterOwnerResult {
            user_id: account.user.id(),
            org_id: account.org_id,
        })
    }
}

/// Utente e organizzazione appena creati da [`create_owner_account`], prima che
/// il chiamante li leghi a una sessione o a un invito appena marcato accettato.
pub(super) struct NewOwnerAccount {
    pub user: User,
    pub org_id: OrgId,
}

/// Il corpo di una registrazione da proprietario: crea utente, organizzazione
/// personale e membership `owner`, **dentro una transazione privilegiata già
/// aperta dal chiamante**.
///
/// Condivisa da [`RegisterOwner`] e da
/// [`AcceptOwnerInvitation`](super::AcceptOwnerInvitation) (ADR-053): il secondo
/// differisce solo per dove prende l'email — quella dell'invito, non quella del
/// corpo — e per la scrittura in più che fa dopo (marcare l'invito accettato),
/// nella stessa transazione. **Non apre né chiude la transazione**: chi chiama
/// decide quando fare `commit`, perché `AcceptOwnerInvitation` deve ancora
/// marcare l'invito prima di considerarsi conclusa.
#[allow(clippy::too_many_arguments)]
pub(super) async fn create_owner_account(
    uow: &mut Box<dyn UnitOfWork>,
    hasher: &Arc<dyn PasswordHasher>,
    clock: &Arc<dyn Clock>,
    context: &RequestContext,
    email: Email,
    full_name: &str,
    password: &str,
    organization_name: Option<&str>,
) -> AppResult<NewOwnerAccount> {
    password_policy::validate(password)?;

    let now = clock.now();
    let password_hash = hasher.hash(password).await.map_err(AppError::internal)?;

    let user_id = UserId::generate();
    let user = User::register(user_id, email, full_name, password_hash, now)?;

    let org_id = OrgId::generate();
    let org_name = organization_name
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(user.full_name());
    let organization = Organization::personal(org_id, org_name, now)?;
    let membership = OrgMember::active(org_id, user_id, OrgRole::Owner, now);

    // Il controllo esplicito dà un 409 parlante; il vincolo unique sul
    // database resta l'autorità in caso di richieste concorrenti.
    if uow.users().find_by_email(user.email()).await?.is_some() {
        return Err(AppError::Conflict(Conflict::EmailTaken));
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

    Ok(NewOwnerAccount { user, org_id })
}

/// Una violazione di unicità sull'email è un conflitto comprensibile, non un 500.
pub(super) fn map_email_conflict(error: AppError) -> AppError {
    match &error {
        AppError::Integrity { details } if details.contains("users_email") => {
            AppError::Conflict(Conflict::EmailTaken)
        }
        _ => error,
    }
}
