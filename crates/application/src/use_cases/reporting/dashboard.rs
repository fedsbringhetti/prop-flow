use std::sync::Arc;

use propflow_domain::ports::UnitOfWorkFactory;
use propflow_domain::reporting::OwnerDashboard;
use propflow_domain::{Denial, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

/// Gli otto numeri del proprietario.
///
/// # `ensure_staff` e non `ensure_member`
///
/// La vista si chiama `v_owner_dashboard` e i numeri che porta sono quelli di chi
/// amministra: canoni attesi, insoluto totale, bollette da rivedere. A un
/// inquilino non servono e non lo riguardano — il suo è un altro elenco, e sarà
/// un'altra proiezione con la sua rotta.
///
/// La Row Level Security non lo fermerebbe da sola in modo utile: la vista passa
/// da `organizations_select`, che è `is_org_member` e non `is_org_staff`, quindi
/// un inquilino otterrebbe la riga con dentro **i numeri di tutta
/// l'organizzazione**. È uno dei casi in cui il controllo applicativo non duplica
/// la policy ma la stringe, ed è la ragione per cui va scritto qui e non lasciato
/// alla RLS.
///
/// # Nessun `404`, e perché
///
/// L'assenza della riga non è «dashboard non trovata»: la vista ha una riga per
/// ogni organizzazione non archiviata, quindi l'unico modo di non vederla è che
/// `organizations_select` la nasconda — cioè che il chiamante non sia (più) membro
/// di quella organizzazione. La risposta onesta è `403 org.not_a_member`, la
/// stessa che il middleware dà a chi chiede un `X-Org-Id` di un'organizzazione
/// altrui. Un `404` direbbe che la rotta non esiste, e la rotta esiste.
pub struct GetOwnerDashboard {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl GetOwnerDashboard {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(&self, context: &RequestContext) -> AppResult<OwnerDashboard> {
        let org_id = authorization::ensure_staff(context)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        uow.dashboard()
            .owner_dashboard(org_id)
            .await?
            .ok_or(AppError::Forbidden(Denial::NotAMember))
    }
}
