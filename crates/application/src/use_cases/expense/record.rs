use std::sync::Arc;

use chrono::NaiveDate;
use propflow_domain::events::{DomainEventPayload, ExpenseCancelled, ExpenseRecorded};
use propflow_domain::expense::{
    Expense, ExpenseCategory, ExpenseShare, NewExpense, RecordedExpense, SplitMethod,
};
use propflow_domain::ports::{Clock, ExpensePage, ExpenseQuery, UnitOfWorkFactory};
use propflow_domain::shared::{ExpenseGroupId, ExpenseId, UserId};
use propflow_domain::{Denial, Entity, RequestContext};
use rust_decimal::Decimal;

use super::shared::{
    append_all, commit_expense, ensure_payer_belongs, open_group, participants, visible_group,
};
use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct RecordExpenseCommand {
    pub group_id: ExpenseGroupId,
    /// Chi ha anticipato. `None` significa «io», che è il caso normale.
    ///
    /// Il campo esiste perché registrare la spesa di un coinquilino che non usa
    /// l'app è il caso co-living vero; deve comunque essere **un membro attivo**,
    /// e a verificarlo è questo use case perché lo schema non lo fa.
    pub paid_by_user_id: Option<UserId>,
    pub category: ExpenseCategory,
    pub description: String,
    pub amount: Decimal,
    /// Quando si è comprato. `None` significa «oggi».
    ///
    /// È una `DATE` e non un istante, quindi il fuso di chi guarda non entra in
    /// gioco: è la differenza con `paid_at` dei pagamenti, dove entra.
    pub expense_date: Option<NaiveDate>,
    /// Il metodo di ripartizione. `None` usa quello predefinito del gruppo.
    pub split_method: Option<SplitMethod>,
    pub notes: Option<String>,
}

/// Registra una spesa e ne scrive le quote, **nella stessa transazione**.
///
/// L'ordine dei passi non è negoziabile, e ognuno esiste per una ragione:
///
/// 1. **il gruppo si rilegge e si verifica che sia aperto.**
///    `app.expense_groups` non ha il trigger `tg_assert_org_matches_property` che
///    hanno `rooms`, `leases`, `tickets` e `utility_bills` — verificato nella
///    `0013` — quindi la rilettura sotto la RLS del chiamante è il solo controllo
///    che esista;
/// 2. **i membri attivi si leggono prima della ripartizione**, perché sono i
///    partecipanti;
/// 3. **chi ha anticipato si verifica contro quei membri.** Lo schema non lo
///    impone, e senza il controllo si scriverebbe un credito che
///    `app.v_expense_group_balances` non mostrerebbe a nessuno, perché la vista
///    parte dai membri. La somma dei saldi del gruppo smetterebbe di fare zero, in
///    silenzio;
/// 4. **spesa e quote si inseriscono insieme**, perché il trigger
///    `expense_shares_balanced` verifica la somma al `COMMIT`;
/// 5. **il commit si mappa.** L'errore del trigger esce da lì e non dalla query
///    che lo ha causato: scrivere `uow.commit().await?` per abitudine
///    trasformerebbe un errore di dominio in un `500`.
///
/// `ensure_member` e non `ensure_staff`: una spesa di casa la registra chi abita
/// la casa. La policy `expenses_insert` chiede `can_access_expense_group`, che è
/// esattamente questo.
pub struct RecordExpense {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl RecordExpense {
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
        command: RecordExpenseCommand,
    ) -> AppResult<RecordedExpense> {
        let org_id = authorization::ensure_member(context)?;
        let caller = context
            .user_id()
            .ok_or(AppError::Forbidden(Denial::AuthenticationRequired))?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let group = open_group(uow.as_mut(), command.group_id).await?;

        let members = uow.expense_groups().list_members(group.id(), true).await?;
        let paid_by_user_id = command.paid_by_user_id.unwrap_or(caller);
        ensure_payer_belongs(&members, paid_by_user_id)?;

        let recorded = Expense::record(
            NewExpense {
                id: ExpenseId::generate(),
                org_id,
                group_id: group.id(),
                paid_by_user_id,
                category: command.category,
                description: command.description,
                amount: command.amount,
                expense_date: command.expense_date.unwrap_or_else(|| now.date_naive()),
                // Il metodo del gruppo è il default e non un obbligo: una spesa
                // che si divide diversamente dalle altre esiste — la bombola del
                // gas di chi ha la stufa — e il gruppo dichiara la consuetudine,
                // non la regola.
                split_method: command
                    .split_method
                    .unwrap_or_else(|| group.default_split_method()),
                notes: command.notes,
                created_by: caller,
            },
            &participants(&members),
            now,
        )?;

        uow.expenses()
            .insert(&recorded.expense, &recorded.shares)
            .await?;

        append_all(
            uow.as_mut(),
            context,
            now,
            vec![DomainEventPayload::ExpenseRecorded(ExpenseRecorded::new(
                &recorded.expense,
                recorded.shares.len(),
            ))],
        )
        .await?;

        commit_expense(uow).await?;
        Ok(recorded)
    }
}

#[derive(Debug, Clone)]
pub struct ListExpensesQuery {
    pub group_id: ExpenseGroupId,
    pub category: Option<ExpenseCategory>,
    pub after: Option<ExpenseId>,
    pub limit: Option<u16>,
}

/// Elenca le spese di un gruppo.
///
/// Il gruppo si rilegge prima: senza, un `group_id` inesistente o invisibile
/// darebbe una **pagina vuota** invece di un `404`, che è il modo peggiore di dire
/// «non è tuo». È la stessa scelta di `ListPayments`.
///
/// **Le spese di un gruppo archiviato si leggono**, perché un conto chiuso si
/// consulta: qui si usa `visible_group` e non `open_group`.
pub struct ListExpenses {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl ListExpenses {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        query: ListExpensesQuery,
    ) -> AppResult<ExpensePage> {
        authorization::ensure_member(context)?;

        let limit = query
            .limit
            .unwrap_or(ExpenseQuery::DEFAULT_LIMIT)
            .clamp(1, ExpenseQuery::MAX_LIMIT);

        let mut uow = self.unit_of_work.begin(context).await?;
        let group = visible_group(uow.as_mut(), query.group_id).await?;

        uow.expenses()
            .list_for_group(&ExpenseQuery {
                group_id: group.id(),
                category: query.category,
                after: query.after,
                limit,
            })
            .await
            .map_err(AppError::from)
    }
}

/// Legge una spesa **con le sue quote**.
///
/// Le quote ci sono sempre e non dietro un parametro: una spesa senza le quote non
/// dice a nessuno quanto deve, che è l'unica domanda per cui la si guarda. Sono
/// poche per costruzione — quante sono le persone in una casa — quindi non c'è una
/// pagina da girare.
pub struct GetExpense {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl GetExpense {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        id: ExpenseId,
    ) -> AppResult<(Expense, Vec<ExpenseShare>)> {
        authorization::ensure_member(context)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        let expense = uow
            .expenses()
            .find_by_id(id)
            .await?
            .ok_or(AppError::not_found(Entity::Expense))?;
        let shares = uow.expenses().list_shares(expense.id()).await?;
        Ok((expense, shares))
    }
}

/// Annulla una spesa: `deleted_at`, ed **esce dai saldi da sola**.
///
/// È l'unico passo con del backend della sezione 7, e la decisione l'aveva già
/// presa la migration `0028`: `deleted_at` è fra le colonne concesse *«per
/// l'annullamento»*, scritto nel suo commento, e `app.v_expense_group_balances`
/// filtra `e.deleted_at IS NULL` dalla `0027`. Nessuna vista da toccare, nessuna
/// migration: la porta era preparata e mai aperta.
///
/// # Chi può, e perché non è la policy a dirlo
///
/// Chi l'ha registrata, chi l'ha anticipata, o lo staff. **La `USING` di
/// `expenses_update` è più larga** — ammette qualunque membro attivo del gruppo,
/// verificato eseguendo e non «chi l'ha registrata o lo staff» come il piano
/// diceva — quindi il confine sta qui, nel contratto dell'API, e non è un confine
/// di sicurezza: un coinquilino non deve poter far sparire dai saldi una spesa che
/// non ha né scritto né anticipato, ma se lo facesse con una query a mano la
/// policy lo lascerebbe fare. È la stessa forma di `LeaveGroup` prima della
/// `0029`. Chi ha anticipato entra perché è il suo credito che la riga afferma;
/// chi l'ha registrata potrebbe essere uscito dal gruppo, e allora resta chi ha
/// anticipato, o lo staff.
///
/// # Il gruppo deve essere aperto
///
/// `open_group` e non `visible_group`, come ogni scrittura del modulo: un conto
/// chiuso non accetta più spese né rimborsi, e disfare una spesa è una scrittura
/// sul conto. Chi trova una spesa sbagliata su un gruppo archiviato non ha una
/// via, ed è coerente con il fatto che l'archiviazione non si disfa.
///
/// # Annullare una spesa già regolata cambia i saldi
///
/// Ed è giusto: un regolamento salda un **saldo**, non una spesa, e togliere una
/// spesa sbagliata dopo un regolamento sposta il saldo esattamente come chi
/// annulla vuole. Non si controlla che nessun regolamento la «citi», perché
/// nessun regolamento cita una spesa. Il test del bordo HTTP prova che i saldi
/// scendono; il client lo dice nella conferma.
pub struct CancelExpense {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl CancelExpense {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>, clock: Arc<dyn Clock>) -> Self {
        Self {
            unit_of_work,
            clock,
        }
    }

    pub async fn execute(&self, context: &RequestContext, id: ExpenseId) -> AppResult<Expense> {
        authorization::ensure_member(context)?;
        let caller = context
            .user_id()
            .ok_or(AppError::Forbidden(Denial::AuthenticationRequired))?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut expense = uow
            .expenses()
            .find_by_id(id)
            .await?
            .ok_or(AppError::not_found(Entity::Expense))?;
        open_group(uow.as_mut(), expense.group_id()).await?;

        // Un `403` con un codice proprio e non un `404`: la riga il chiamante la
        // vede — `expenses_select` la mostra a tutto il gruppo — e nascondergliela
        // adesso sarebbe incoerente. Il codice gli dice chi può.
        let mine = expense.created_by() == Some(caller) || expense.paid_by_user_id() == caller;
        if !mine && !context.is_staff() {
            return Err(AppError::Forbidden(Denial::ExpenseNotYours));
        }

        expense.cancel(now)?;
        uow.expenses().cancel(&expense).await?;

        append_all(
            uow.as_mut(),
            context,
            now,
            vec![DomainEventPayload::ExpenseCancelled(ExpenseCancelled::new(
                &expense, caller,
            ))],
        )
        .await?;

        uow.commit().await?;
        Ok(expense)
    }
}
