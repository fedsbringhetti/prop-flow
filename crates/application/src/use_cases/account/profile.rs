use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::events::{DomainEvent, DomainEventPayload, EventMetadata, UserProfileUpdated};
use propflow_domain::identity::{User, UserProfileChanges};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};

use crate::authorization;
use crate::error::{AppError, AppResult};

/// I propri dati, letti dal token e non dal percorso.
///
/// # `ensure_authenticated`, e non `ensure_member`
///
/// Per la stessa ragione di `ListOrganizations`: il proprio account **non
/// appartiene a nessuna organizzazione**, e pretenderne una attiva romperebbe
/// esattamente il caso che questa rotta serve — chi è appena uscito dall'unica in
/// cui abitava, e ha bisogno di guardare i propri dati proprio allora.
///
/// Non è una speranza: `users_select` ammette `id = app.current_user_id()` come
/// primo ramo, e quel ramo non guarda `app.current_org_id()`.
///
/// # Restituisce l'aggregato, e il bordo HTTP ne mostra una parte
///
/// `platform_role`, `failed_login_count` e `locked_until` **non escono nel DTO**:
/// non sono affare di chi guarda, e mostrarli darebbe a un client una superficie
/// che non deve avere. La proiezione è nel DTO e non qui perché il caso d'uso ha
/// già in mano l'utente per averlo letto, e un modello di lettura in più per
/// nascondere tre campi sarebbe cerimonia.
pub struct GetMyAccount {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl GetMyAccount {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(&self, context: &RequestContext) -> AppResult<User> {
        authorization::ensure_authenticated(context)?;
        let user_id = context.require_user()?;

        let mut uow = self.unit_of_work.begin(context).await?;
        uow.users()
            .find_by_id(user_id)
            .await?
            // La riga di chi chiama, letta con il suo stesso contesto: se non
            // torna, l'account è stato disattivato mentre il token era ancora
            // valido. Non è un `404` — nessuna risorsa è stata chiesta per
            // identificatore — è una sessione che non vale più.
            .ok_or(AppError::Unauthenticated)
    }
}

#[derive(Debug, Clone, Default)]
pub struct UpdateMyProfileCommand {
    pub changes: UserProfileChanges,
}

/// La correzione dei propri dati anagrafici.
///
/// # Passa da `update_profile`, e non da `update`
///
/// È la trappola della migration `0038`, ed è il motivo per cui questo caso d'uso
/// è anche la sua dimostrazione: `UserRepository::update` nomina tutte le colonne
/// che il dominio muove — `status`, `last_login_at`, `failed_login_count`,
/// `locked_until` — e da quella migration il ruolo applicativo non ha il
/// privilegio di scriverle. Chiamarlo da qui darebbe `permission denied for table
/// users`, e siccome quel metodo lo usano solo flussi **privilegiati**, senza
/// questa rotta la migration non lo avrebbe mai dimostrato.
///
/// # Nessun campo menzionato non è un errore
///
/// Un `PATCH {}` risponde `200` con i dati invariati e **non scrive**: non c'è
/// nulla da rifiutare, e un `422` costringerebbe un client a distinguere un form
/// senza modifiche da un form sbagliato.
pub struct UpdateMyProfile {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl UpdateMyProfile {
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
        command: UpdateMyProfileCommand,
    ) -> AppResult<User> {
        authorization::ensure_authenticated(context)?;
        let user_id = context.require_user()?;
        let now = self.clock.now();
        let changed_fields = command.changes.changed_field_names();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut user = uow
            .users()
            .find_by_id(user_id)
            .await?
            .ok_or(AppError::Unauthenticated)?;

        if !user.apply_profile(command.changes)? {
            return Ok(user);
        }

        uow.users().update_profile(&user).await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::UserProfileUpdated(UserProfileUpdated {
                    user_id,
                    // I **nomi** dei campi e non i valori: un nome e un telefono
                    // sono dati personali, e un evento pubblicato viaggia e resta.
                    changed_fields,
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(user)
    }
}
