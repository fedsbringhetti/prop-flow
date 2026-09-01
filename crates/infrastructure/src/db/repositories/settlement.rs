use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::expense::{Settlement, SettlementParts};
use propflow_domain::ports::{
    RepositoryError, RepositoryResult, SettlementPage, SettlementQuery, SettlementRepository,
};
use propflow_domain::shared::{Currency, ExpenseGroupId, Money, OrgId, SettlementId, UserId};

use super::user::try_get;
use crate::db::db_enums;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

/// Colonne lette di un regolamento.
///
/// **Non c'è `deleted_at`**: `app.settlements` non ha quella colonna, e nessuna
/// query di questo file la filtra. È la prima cosa da sapere prima di copiarne una
/// da un altro repository — è la terza tabella di questo modulo senza soft delete,
/// dopo `expense_group_members` e `expense_shares`.
///
/// **Non c'è `payment_id`**: resta nullo, e la ragione non è la simmetria. Un
/// regolamento fra coinquilini non è denaro che l'organizzazione incassa, e farne
/// una riga in `app.payments` metterebbe nel registro degli incassi del
/// proprietario un movimento che non lo riguarda. Vedi la nota su `Settlement`.
macro_rules! select_settlement {
    () => {
        "
    SELECT id,
           org_id,
           group_id,
           from_user_id,
           to_user_id,
           amount::numeric AS amount,
           currency::text  AS currency,
           status::text    AS status,
           method::text    AS method,
           note,
           settled_at,
           confirmed_at,
           confirmed_by,
           created_at,
           updated_at
      FROM app.settlements
"
    };
}

#[async_trait]
impl SettlementRepository for PgUnitOfWork {
    async fn find_by_id(&mut self, id: SettlementId) -> RepositoryResult<Option<Settlement>> {
        let row = sqlx::query(concat!(select_settlement!(), " WHERE id = $1"))
            .bind(id.as_uuid())
            .fetch_optional(&mut *self.tx)
            .await
            .map_err(map_sqlx_error)?;
        row.as_ref().map(map_settlement).transpose()
    }

    async fn list_for_group(
        &mut self,
        query: &SettlementQuery,
    ) -> RepositoryResult<SettlementPage> {
        let fetch = i64::from(query.limit) + 1;
        let rows = sqlx::query(concat!(
            select_settlement!(),
            " WHERE group_id = $1
                AND ($2::text IS NULL OR status = $2::app.settlement_status)
                AND ($3::uuid IS NULL OR id > $3)
              ORDER BY id
              LIMIT $4"
        ))
        .bind(query.group_id.as_uuid())
        .bind(query.status.map(db_enums::settlement_status_to_db))
        .bind(query.after.map(SettlementId::as_uuid))
        .bind(fetch)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        let has_more = rows.len() > usize::from(query.limit);
        let items = rows
            .iter()
            .take(usize::from(query.limit))
            .map(map_settlement)
            .collect::<RepositoryResult<Vec<Settlement>>>()?;
        let next_cursor = if has_more {
            items.last().map(Settlement::id)
        } else {
            None
        };

        Ok(SettlementPage { items, next_cursor })
    }

    async fn insert(&mut self, settlement: &Settlement) -> RepositoryResult<()> {
        // Nessun `RETURNING`: nessuna colonna è assegnata dal database, e
        // `settlements_select` chiede `can_access_expense_group(group_id)` — che
        // interroga un'**altra** tabella, quindi qui il muro delle segnalazioni non
        // ci sarebbe. Non usarlo comunque, per non far credere che sia una scelta.
        //
        // `payment_id` resta nullo: vedi la nota sulla macro.
        sqlx::query(
            "INSERT INTO app.settlements (
                 id, org_id, group_id, from_user_id, to_user_id,
                 amount, currency, status, method, note,
                 settled_at, confirmed_at, confirmed_by,
                 created_at, updated_at
             ) VALUES (
                 $1, $2, $3, $4, $5,
                 $6::app.money_amount, $7::app.currency_code,
                 $8::app.settlement_status, $9::app.payment_method, $10,
                 $11, $12, $13,
                 $14, $15
             )",
        )
        .bind(settlement.id().as_uuid())
        .bind(settlement.org_id().as_uuid())
        .bind(settlement.group_id().as_uuid())
        .bind(settlement.from_user_id().as_uuid())
        .bind(settlement.to_user_id().as_uuid())
        .bind(settlement.amount().amount())
        .bind(settlement.currency().as_str())
        .bind(db_enums::settlement_status_to_db(settlement.status()))
        .bind(db_enums::payment_method_to_db(settlement.method()))
        .bind(settlement.note())
        .bind(settlement.settled_at())
        .bind(settlement.confirmed_at())
        .bind(settlement.confirmed_by().map(UserId::as_uuid))
        .bind(settlement.created_at())
        .bind(settlement.updated_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn update_status(&mut self, settlement: &Settlement) -> RepositoryResult<()> {
        // Quattro colonne, ed è l'elenco delle tre transizioni. **Su
        // `app.settlements` non ci sono privilegi di colonna**, a differenza di
        // `app.payments`: nominare qui `amount` o `to_user_id` non fallirebbe con
        // `permission denied`, **passerebbe** — la `WITH CHECK` di
        // `settlements_update` nomina il solo `org_id`. L'elenco corto è quindi
        // l'unica difesa, e va lasciato corto.
        //
        // `settled_at` è fra esse perché le due uscite negative la **azzerano**:
        // `settlements_confirmed_ck` vincola una sola direzione — obbliga ad avere
        // `confirmed_at` quando lo stato è `confirmed`, non a togliere niente
        // muovendosi altrove — quindi l'azzeramento tocca al dominio e la colonna
        // deve restare scrivibile. È la quarta occorrenza di questa forma dopo
        // `tickets_closed_ck`, `utility_bills_validated_ck` e
        // `payments_succeeded_ck`.
        //
        // `updated_at` non compare perché la scrive il trigger `set_updated_at`.
        let result = sqlx::query(
            "UPDATE app.settlements
                SET status = $2::app.settlement_status,
                    settled_at = $3,
                    confirmed_at = $4,
                    confirmed_by = $5
              WHERE id = $1",
        )
        .bind(settlement.id().as_uuid())
        .bind(db_enums::settlement_status_to_db(settlement.status()))
        .bind(settlement.settled_at())
        .bind(settlement.confirmed_at())
        .bind(settlement.confirmed_by().map(UserId::as_uuid))
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }
}

fn invalid(field: &str, error: propflow_domain::DomainError) -> RepositoryError {
    RepositoryError::Backend {
        details: format!("{field} non valido in database: {error}"),
    }
}

fn map_settlement(row: &PgRow) -> RepositoryResult<Settlement> {
    let id: Uuid = try_get(row, "id")?;
    let org_id: Uuid = try_get(row, "org_id")?;
    let group_id: Uuid = try_get(row, "group_id")?;
    let from_user_id: Uuid = try_get(row, "from_user_id")?;
    let to_user_id: Uuid = try_get(row, "to_user_id")?;
    let amount: Decimal = try_get(row, "amount")?;
    let currency_code: String = try_get(row, "currency")?;
    let status: String = try_get(row, "status")?;
    let method: String = try_get(row, "method")?;
    let confirmed_by: Option<Uuid> = try_get(row, "confirmed_by")?;

    let currency = Currency::parse(&currency_code).map_err(|error| invalid("currency", error))?;

    Ok(Settlement::from_persistence(SettlementParts {
        id: SettlementId::from_uuid(id),
        org_id: OrgId::from_uuid(org_id),
        group_id: ExpenseGroupId::from_uuid(group_id),
        from_user_id: UserId::from_uuid(from_user_id),
        to_user_id: UserId::from_uuid(to_user_id),
        amount: Money::new(amount, currency).map_err(|error| invalid("amount", error))?,
        currency,
        status: db_enums::settlement_status_from_db(&status)?,
        method: db_enums::payment_method_from_db(&method)?,
        note: try_get(row, "note")?,
        settled_at: try_get(row, "settled_at")?,
        confirmed_at: try_get(row, "confirmed_at")?,
        confirmed_by: confirmed_by.map(UserId::from_uuid),
        created_at: try_get(row, "created_at")?,
        updated_at: try_get(row, "updated_at")?,
    }))
}
