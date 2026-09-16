use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::events::{
    DomainEvent, DomainEventPayload, EventMetadata, OrganizationCreated, UserRegistered,
};
use propflow_domain::identity::{OrgMember, OrgRole, Organization, User};
use propflow_domain::ports::{Clock, UnitOfWork};
use propflow_domain::shared::{Email, OrgId, UserId};

use crate::error::{AppError, AppResult, Conflict};
use crate::password_policy;
use crate::ports::PasswordHasher;

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
/// L'unico chiamante è [`AcceptOwnerInvitation`](super::AcceptOwnerInvitation):
/// prima di `F6` (ADR-053) esisteva anche `RegisterOwner`, la rotta aperta a
/// chiunque, e questa funzione era condivisa fra le due. Resta separata dal
/// caso d'uso invece di essere inglobata in lui perché è la parte che non
/// dipende dal token — costruire l'account — e la sua forma non cambierebbe se
/// domani nascesse un secondo modo di arrivarci. **Non apre né chiude la
/// transazione**: chi chiama decide quando fare `commit`, perché
/// `AcceptOwnerInvitation` deve ancora marcare l'invito prima di considerarsi
/// conclusa.
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
