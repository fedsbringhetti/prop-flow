use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::identity::OrganizationMembership;
use propflow_domain::ports::UnitOfWorkFactory;

use crate::authorization;
use crate::error::{AppError, AppResult};

/// «Quali organizzazioni sono mie», con il **nome** e non con l'identificatore.
///
/// # `ensure_authenticated`, e non `ensure_member`
///
/// Perché la domanda **non appartiene a nessuna organizzazione**: chiederne una
/// attiva per poterla porre è circolare, e rompe proprio il caso che l'endpoint
/// esiste per servire — chi non ne ha una selezionata, o non ne ha più.
///
/// Non è una speranza: `app.member_role(p_org_id)` interroga `app.org_members`
/// filtrando su `app.current_user_id()` e sull'organizzazione **passata come
/// argomento**, e `app.current_org_id()` non vi compare. `organizations_select` è
/// `is_org_member(id)`, `org_members_select` è
/// `user_id = current_user_id() OR is_org_member(org_id)`, e
/// `app.set_request_context` accetta già un `org_id` nullo. La query gira quindi
/// con un contesto privo di organizzazione, e restituisce le proprie.
///
/// `ensure_member` funzionerebbe quasi sempre — il token porta l'organizzazione
/// predefinita — e fallirebbe esattamente quando serve.
///
/// # Il nome è il punto, non una comodità
///
/// Prima di questo use case nessun endpoint restituiva il nome di
/// un'organizzazione a chi non ne fosse staff, e un selettore poteva quindi
/// mostrare solo l'UUID. Per chi ne ha una è indistinguibile da un elenco vero;
/// per un inquilino sono due UUID fra cui indovinare.
///
/// # Nessun cursore
///
/// Le organizzazioni di una persona sono poche per costruzione — la propria, più
/// quelle di chi le affitta casa — e una paginazione su un elenco che sta in una
/// riga è cerimonia. La restrizione la fa la Row Level Security, non un `WHERE`
/// sull'organizzazione attiva.
pub struct ListOrganizations {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl ListOrganizations {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
    ) -> AppResult<Vec<OrganizationMembership>> {
        authorization::ensure_authenticated(context)?;
        let user_id = context.require_user()?;

        let mut uow = self.unit_of_work.begin(context).await?;
        uow.members()
            .list_organizations_for_user(user_id)
            .await
            .map_err(AppError::from)
    }
}
