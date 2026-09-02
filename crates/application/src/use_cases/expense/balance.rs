use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::expense::GroupBalance;
use propflow_domain::ports::UnitOfWorkFactory;
use propflow_domain::shared::ExpenseGroupId;

use super::shared::visible_group;
use crate::authorization;
use crate::error::{AppError, AppResult};

/// I saldi di un gruppo, **letti** da `app.v_expense_group_balances`.
///
/// # Si leggono e non si ricalcolano
///
/// Vale la ragione di ADR-032: due definizioni dello stesso numero si
/// contraddicono al primo caso limite, e quella che conta è la definizione che ha
/// prodotto il numero mostrato all'utente. Qui la fonte è la vista, e il dominio
/// non ha un metodo che rifaccia quella somma.
///
/// # Che cosa la migration `0027` ha cambiato, e perché si vede da qui
///
/// La vista partiva da `app.expense_group_members` con `WHERE m.left_at IS NULL`.
/// Chi lasciava il gruppo **spariva dai saldi portandosi via il debito**: il
/// coinquilino che deve 200 euro esce, e quei 200 euro non risultano più a
/// nessuno. La somma dei saldi netti del gruppo, che deve fare zero, smetteva di
/// farlo, e il credito di chi aveva anticipato restava senza contropartita.
///
/// Dalla `0027` chi è uscito resta finché il suo netto è diverso da zero. Chi esce
/// in pari sparisce, che è il comportamento voluto.
///
/// **Ne segue una cosa da sapere prima di leggere questi numeri da un client:**
/// la vista è `security_invoker`, quindi le righe passano da
/// `expense_group_members_select`, che chiede `can_access_expense_group` — vera per
/// lo staff e per chi ha una membership **attiva**. Il debitore uscito compare
/// quindi nei saldi che leggono gli **altri** e non nei propri: per lui il gruppo è
/// invisibile. È corretto rispetto all'isolamento, ed è deliberatamente fuori dal
/// perimetro di questa fetta — farlo vedere anche a chi è uscito è un allargamento
/// della visibilità, cioè una decisione di prodotto con la sua policy.
///
/// # I saldi di un gruppo archiviato si leggono
///
/// `visible_group` e non `open_group`: un conto chiuso si consulta, ed è
/// l'operazione per cui più spesso lo si riapre.
pub struct GetGroupBalances {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl GetGroupBalances {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        group_id: ExpenseGroupId,
    ) -> AppResult<Vec<GroupBalance>> {
        authorization::ensure_member(context)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        // Il gruppo si rilegge prima: senza, un identificatore inesistente o
        // invisibile darebbe un elenco vuoto invece di un `404`, e un elenco vuoto
        // di saldi è indistinguibile da un gruppo in cui nessuno deve niente.
        let group = visible_group(uow.as_mut(), group_id).await?;

        uow.expense_groups()
            .list_balances(group.id())
            .await
            .map_err(AppError::from)
    }
}
