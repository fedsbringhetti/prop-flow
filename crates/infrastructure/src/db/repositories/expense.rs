use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::expense::{Expense, ExpenseParts, ExpenseShare, ExpenseShareParts, Weight};
use propflow_domain::ports::{ExpensePage, ExpenseQuery, ExpenseRepository, RepositoryResult};
use propflow_domain::shared::{
    Currency, ExpenseGroupId, ExpenseId, ExpenseShareId, Money, OrgId, UserId,
};

use super::user::try_get;
use crate::db::db_enums;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

/// Colonne lette di una spesa.
///
/// **Non ci sono `utility_bill_id` né `document_id`.** La ripartizione automatica
/// di una bolletta e gli allegati alle spese sono fuori dal perimetro della fetta
/// (`docs/09-stato-e-roadmap.md`), il dominio non li modella, e leggere una colonna
/// che nessuno sa interpretare significa doverla mappare su un tipo inventato per
/// l'occasione. Restano nulle, e la migration `0028` nega l'`UPDATE` su entrambe.
macro_rules! select_expense {
    () => {
        "
    SELECT id,
           org_id,
           group_id,
           paid_by_user_id,
           category::text     AS category,
           description,
           amount::numeric    AS amount,
           currency::text     AS currency,
           expense_date,
           split_method::text AS split_method,
           notes,
           created_by,
           created_at,
           updated_at
      FROM app.expenses
"
    };
}

/// Colonne lette di una quota.
///
/// **Non ci sono `is_settled` né `settled_at`**, e non è un'omissione: restano ai
/// loro default perché `app.v_expense_group_balances` non li guarda. Scriverli
/// sarebbe una seconda definizione di «saldato» che nessuno riconcilia — vedi la
/// nota su `ExpenseShare`. **Non c'è `deleted_at`**: `app.expense_shares` non ha
/// quella colonna.
macro_rules! select_share {
    () => {
        "
    SELECT id,
           org_id,
           expense_id,
           user_id,
           share_amount::numeric AS share_amount,
           share_weight::numeric AS share_weight,
           created_at,
           updated_at
      FROM app.expense_shares
"
    };
}

#[async_trait]
impl ExpenseRepository for PgUnitOfWork {
    async fn find_by_id(&mut self, id: ExpenseId) -> RepositoryResult<Option<Expense>> {
        let row = sqlx::query(concat!(
            select_expense!(),
            " WHERE id = $1 AND deleted_at IS NULL"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.as_ref().map(map_expense).transpose()
    }

    async fn list_for_group(&mut self, query: &ExpenseQuery) -> RepositoryResult<ExpensePage> {
        let fetch = i64::from(query.limit) + 1;
        let rows = sqlx::query(concat!(
            select_expense!(),
            " WHERE group_id = $1
                AND deleted_at IS NULL
                AND ($2::text IS NULL OR category = $2::app.expense_category)
                AND ($3::uuid IS NULL OR id > $3)
              ORDER BY id
              LIMIT $4"
        ))
        .bind(query.group_id.as_uuid())
        .bind(query.category.map(db_enums::expense_category_to_db))
        .bind(query.after.map(ExpenseId::as_uuid))
        .bind(fetch)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        let has_more = rows.len() > usize::from(query.limit);
        let items = rows
            .iter()
            .take(usize::from(query.limit))
            .map(map_expense)
            .collect::<RepositoryResult<Vec<Expense>>>()?;
        let next_cursor = if has_more {
            items.last().map(Expense::id)
        } else {
            None
        };

        Ok(ExpensePage { items, next_cursor })
    }

    async fn insert(&mut self, expense: &Expense, shares: &[ExpenseShare]) -> RepositoryResult<()> {
        // La spesa prima delle quote, e non per eleganza: il trigger
        // `app.tg_expense_shares_balanced` legge `app.expenses.amount` come prima
        // cosa e restituisce `NULL` — cioè non verifica niente — se non trova la
        // riga. Inserire le quote per prime non darebbe un errore: darebbe una
        // verifica **saltata**, che è peggio.
        //
        // Nessun `RETURNING`: nessuna colonna è assegnata dal database.
        // `utility_bill_id`, `document_id` e `metadata` restano ai loro default,
        // per il perimetro della fetta.
        sqlx::query(
            "INSERT INTO app.expenses (
                 id, org_id, group_id, paid_by_user_id, category, description,
                 amount, currency, expense_date, split_method, notes, created_by,
                 created_at, updated_at
             ) VALUES (
                 $1, $2, $3, $4, $5::app.expense_category, $6,
                 $7::app.money_amount, $8::app.currency_code, $9,
                 $10::app.expense_split_method, $11, $12,
                 $13, $14
             )",
        )
        .bind(expense.id().as_uuid())
        .bind(expense.org_id().as_uuid())
        .bind(expense.group_id().as_uuid())
        .bind(expense.paid_by_user_id().as_uuid())
        .bind(db_enums::expense_category_to_db(expense.category()))
        .bind(expense.description())
        .bind(expense.amount().amount())
        .bind(expense.currency().as_str())
        .bind(expense.expense_date())
        .bind(db_enums::split_method_to_db(expense.split_method()))
        .bind(expense.notes())
        .bind(expense.created_by().map(UserId::as_uuid))
        .bind(expense.created_at())
        .bind(expense.updated_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        // **Un `INSERT` solo per tutte le quote**, con gli array srotolati da
        // `unnest`. Il trigger è `FOR EACH ROW`: con quattro coinquilini scatta
        // quattro volte e ogni volta risomma tutte le quote, il che è irrilevante
        // a queste dimensioni ma va saputo prima di inserire quote a centinaia.
        // Un `INSERT` per riga moltiplicherebbe anche i round trip, che invece si
        // possono evitare senza costruire SQL dinamico — la stringa resta un
        // letterale statico, come vuole la regola 15.
        if shares.is_empty() {
            return Ok(());
        }

        let ids: Vec<Uuid> = shares.iter().map(|share| share.id().as_uuid()).collect();
        let user_ids: Vec<Uuid> = shares
            .iter()
            .map(|share| share.user_id().as_uuid())
            .collect();
        let amounts: Vec<Decimal> = shares
            .iter()
            .map(|share| share.share_amount().amount())
            .collect();
        // `Option<Decimal>` per colonna annullabile: su una divisione in parti
        // uguali il peso non ha partecipato al calcolo, e `share_weight` resta
        // nullo perché scriverne uno direbbe il contrario.
        let weights: Vec<Option<Decimal>> = shares
            .iter()
            .map(|share| share.share_weight().map(Weight::value))
            .collect();

        sqlx::query(
            "INSERT INTO app.expense_shares (
                 id, org_id, expense_id, user_id, share_amount, share_weight,
                 created_at, updated_at
             )
             SELECT s.id, $2, $3, s.user_id,
                    s.share_amount::app.money_amount, s.share_weight,
                    $7, $8
               FROM unnest($1::uuid[], $4::uuid[], $5::numeric[], $6::numeric[])
                 AS s(id, user_id, share_amount, share_weight)",
        )
        .bind(&ids)
        .bind(expense.org_id().as_uuid())
        .bind(expense.id().as_uuid())
        .bind(&user_ids)
        .bind(&amounts)
        .bind(&weights)
        .bind(expense.created_at())
        .bind(expense.updated_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        // **L'errore del trigger non esce da qui.** `expense_shares_balanced` è
        // `DEFERRABLE INITIALLY DEFERRED`: la verifica avviene al `COMMIT`, e la
        // violazione esce da `uow.commit()`. Chi chiama deve mapparla là — vedi
        // `commit_expense` nel service layer.
        Ok(())
    }

    async fn list_shares(&mut self, expense_id: ExpenseId) -> RepositoryResult<Vec<ExpenseShare>> {
        // `ORDER BY user_id`, come l'elenco dei membri e come la ripartizione:
        // leggere le quote nello stesso ordine in cui sono state calcolate rende
        // verificabile a occhio chi ha preso il centesimo di resto.
        let rows = sqlx::query(concat!(
            select_share!(),
            " WHERE expense_id = $1 ORDER BY user_id"
        ))
        .bind(expense_id.as_uuid())
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        rows.iter().map(map_share).collect()
    }
}

fn invalid(
    field: &str,
    error: propflow_domain::DomainError,
) -> propflow_domain::ports::RepositoryError {
    propflow_domain::ports::RepositoryError::Backend {
        details: format!("{field} non valido in database: {error}"),
    }
}

fn map_expense(row: &PgRow) -> RepositoryResult<Expense> {
    let id: Uuid = try_get(row, "id")?;
    let org_id: Uuid = try_get(row, "org_id")?;
    let group_id: Uuid = try_get(row, "group_id")?;
    let paid_by_user_id: Uuid = try_get(row, "paid_by_user_id")?;
    let category: String = try_get(row, "category")?;
    let split_method: String = try_get(row, "split_method")?;
    let currency_code: String = try_get(row, "currency")?;
    let amount: Decimal = try_get(row, "amount")?;
    let created_by: Option<Uuid> = try_get(row, "created_by")?;

    let currency = Currency::parse(&currency_code).map_err(|error| invalid("currency", error))?;

    Ok(Expense::from_persistence(ExpenseParts {
        id: ExpenseId::from_uuid(id),
        org_id: OrgId::from_uuid(org_id),
        group_id: ExpenseGroupId::from_uuid(group_id),
        paid_by_user_id: UserId::from_uuid(paid_by_user_id),
        category: db_enums::expense_category_from_db(&category)?,
        description: try_get(row, "description")?,
        amount: Money::new(amount, currency).map_err(|error| invalid("amount", error))?,
        currency,
        expense_date: try_get::<NaiveDate>(row, "expense_date")?,
        split_method: db_enums::split_method_from_db(&split_method)?,
        notes: try_get(row, "notes")?,
        created_by: created_by.map(UserId::from_uuid),
        created_at: try_get(row, "created_at")?,
        updated_at: try_get(row, "updated_at")?,
    }))
}

fn map_share(row: &PgRow) -> RepositoryResult<ExpenseShare> {
    let id: Uuid = try_get(row, "id")?;
    let org_id: Uuid = try_get(row, "org_id")?;
    let expense_id: Uuid = try_get(row, "expense_id")?;
    let user_id: Uuid = try_get(row, "user_id")?;
    let share_amount: Decimal = try_get(row, "share_amount")?;
    let share_weight: Option<Decimal> = try_get(row, "share_weight")?;

    Ok(ExpenseShare::from_persistence(ExpenseShareParts {
        id: ExpenseShareId::from_uuid(id),
        org_id: OrgId::from_uuid(org_id),
        expense_id: ExpenseId::from_uuid(expense_id),
        user_id: UserId::from_uuid(user_id),
        // `non_negative` e non `new`: `app.expense_shares.share_amount` è
        // `CHECK (share_amount >= 0)`, e una quota negativa sarebbe una spesa che
        // rimborsa, che questo modulo non modella.
        share_amount: Money::non_negative(share_amount, Currency::EUR)
            .map_err(|error| invalid("share_amount", error))?,
        share_weight: share_weight.map(Weight::from_persistence),
        created_at: try_get(row, "created_at")?,
        updated_at: try_get(row, "updated_at")?,
    }))
}
