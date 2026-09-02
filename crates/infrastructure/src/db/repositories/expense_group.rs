use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::expense::{
    ExpenseGroup, ExpenseGroupMember, ExpenseGroupMemberParts, ExpenseGroupParts, GroupBalance,
    Weight,
};
use propflow_domain::ports::{
    ExpenseGroupPage, ExpenseGroupQuery, ExpenseGroupRepository, RepositoryError, RepositoryResult,
};
use propflow_domain::shared::{
    Currency, ExpenseGroupId, ExpenseGroupMemberId, Money, OrgId, PropertyId, RoomId, UserId,
};

use super::user::try_get;
use crate::db::db_enums;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

/// Colonne lette di un gruppo di spesa.
macro_rules! select_group {
    () => {
        "
    SELECT id,
           org_id,
           property_id,
           kind::text                 AS kind,
           name,
           currency::text             AS currency,
           default_split_method::text AS default_split_method,
           created_by,
           is_archived,
           created_at,
           updated_at
      FROM app.expense_groups
"
    };
}

/// Colonne lette di una membership.
///
/// **Non c'è `deleted_at`**: `app.expense_group_members` non ha quella colonna.
/// L'uscita dal gruppo è `left_at`, che è un fatto del gruppo e non una
/// cancellazione — la riga deve restare, perché le quote intestate a quella
/// persona restano e perché `app.v_expense_group_balances` parte proprio da qui.
macro_rules! select_member {
    () => {
        "
    SELECT id,
           org_id,
           group_id,
           user_id,
           room_id,
           display_name,
           weight::numeric AS weight,
           joined_at,
           left_at,
           created_at,
           updated_at
      FROM app.expense_group_members
"
    };
}

#[async_trait]
impl ExpenseGroupRepository for PgUnitOfWork {
    async fn find_by_id(&mut self, id: ExpenseGroupId) -> RepositoryResult<Option<ExpenseGroup>> {
        let row = sqlx::query(concat!(
            select_group!(),
            " WHERE id = $1 AND deleted_at IS NULL"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.as_ref().map(map_group).transpose()
    }

    async fn list(&mut self, query: &ExpenseGroupQuery) -> RepositoryResult<ExpenseGroupPage> {
        // Cursore keyset sugli UUIDv7: ordinare per id equivale a ordinare per
        // data di creazione, senza una colonna in più. Si chiede una riga in più
        // del limite per sapere se esiste una pagina successiva senza un secondo
        // `COUNT`.
        let fetch = i64::from(query.limit) + 1;
        let rows = sqlx::query(concat!(
            select_group!(),
            " WHERE deleted_at IS NULL
                AND ($1::uuid IS NULL OR property_id = $1)
                AND ($2::boolean IS NULL OR is_archived = $2)
                AND ($3::uuid IS NULL OR id > $3)
              ORDER BY id
              LIMIT $4"
        ))
        .bind(query.property_id.map(PropertyId::as_uuid))
        .bind(query.is_archived)
        .bind(query.after.map(ExpenseGroupId::as_uuid))
        .bind(fetch)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        let has_more = rows.len() > usize::from(query.limit);
        let items = rows
            .iter()
            .take(usize::from(query.limit))
            .map(map_group)
            .collect::<RepositoryResult<Vec<ExpenseGroup>>>()?;
        let next_cursor = if has_more {
            items.last().map(ExpenseGroup::id)
        } else {
            None
        };

        Ok(ExpenseGroupPage { items, next_cursor })
    }

    async fn insert(&mut self, group: &ExpenseGroup) -> RepositoryResult<()> {
        // **Nessun `RETURNING`, e stavolta il muro c'è davvero.**
        // `expense_groups_select` chiama `can_access_expense_group(id)`, cioè
        // interroga la riga che si sta inserendo, e la funzione è `STABLE`: vede
        // lo snapshot di inizio statement, in cui la riga non c'è. È il muro già
        // incontrato sulle segnalazioni, e la regola generale vale ancora — il
        // muro c'è solo quando la policy di `SELECT` dipende dalla riga stessa.
        // Sugli addebiti non c'era, perché `charges_select` interroga
        // `app.leases`. Nessuna colonna è assegnata dal database, quindi non se ne
        // sente la mancanza.
        sqlx::query(
            "INSERT INTO app.expense_groups (
                 id, org_id, property_id, kind, name, currency,
                 default_split_method, created_by, is_archived,
                 created_at, updated_at
             ) VALUES (
                 $1, $2, $3, $4::app.expense_group_kind, $5, $6::app.currency_code,
                 $7::app.expense_split_method, $8, $9,
                 $10, $11
             )",
        )
        .bind(group.id().as_uuid())
        .bind(group.org_id().as_uuid())
        .bind(group.property_id().map(PropertyId::as_uuid))
        .bind(db_enums::expense_group_kind_to_db(group.kind()))
        .bind(group.name())
        .bind(group.currency().as_str())
        .bind(db_enums::split_method_to_db(group.default_split_method()))
        .bind(group.created_by().map(UserId::as_uuid))
        .bind(group.is_archived())
        .bind(group.created_at())
        .bind(group.updated_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn update(&mut self, group: &ExpenseGroup) -> RepositoryResult<()> {
        // Una colonna sola, perché una sola il dominio la muove. **Su
        // `app.expense_groups` non ci sono privilegi di colonna**, a differenza di
        // `app.expenses`: nominarne altre non fallirebbe, passerebbe — ed è la
        // ragione per cui l'elenco va scritto corto e lasciato corto.
        //
        // `updated_at` non compare perché la scrive il trigger `set_updated_at`.
        let result = sqlx::query(
            "UPDATE app.expense_groups
                SET is_archived = $2
              WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(group.id().as_uuid())
        .bind(group.is_archived())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }

    async fn list_members(
        &mut self,
        group_id: ExpenseGroupId,
        active_only: bool,
    ) -> RepositoryResult<Vec<ExpenseGroupMember>> {
        // `ORDER BY user_id` e non `joined_at`: è l'ordine della ripartizione, e
        // averlo già qui rende l'elenco leggibile accanto alle quote. La
        // ripartizione riordina comunque da sé — vedi `Split` — quindi questo
        // `ORDER BY` è una comodità e non un contratto, ed è importante che resti
        // tale: se lo fosse, cambiarlo cambierebbe chi paga il centesimo di resto.
        let rows = sqlx::query(concat!(
            select_member!(),
            " WHERE group_id = $1
                AND (NOT $2::boolean OR left_at IS NULL)
              ORDER BY user_id"
        ))
        .bind(group_id.as_uuid())
        .bind(active_only)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        rows.iter().map(map_member).collect()
    }

    async fn find_member(
        &mut self,
        group_id: ExpenseGroupId,
        user_id: UserId,
    ) -> RepositoryResult<Option<ExpenseGroupMember>> {
        // **Nessun filtro su `left_at`**, ed è il punto del metodo: restituisce
        // anche la riga di chi è uscito, perché
        // `expense_group_members_group_user_uk` è unico su `(group_id, user_id)` e
        // non è parziale. Chi rientra è un `UPDATE`, e senza questa lettura
        // sarebbe un conflitto sul vincolo.
        let row = sqlx::query(concat!(
            select_member!(),
            " WHERE group_id = $1 AND user_id = $2"
        ))
        .bind(group_id.as_uuid())
        .bind(user_id.as_uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.as_ref().map(map_member).transpose()
    }

    async fn insert_member(&mut self, member: &ExpenseGroupMember) -> RepositoryResult<()> {
        sqlx::query(
            "INSERT INTO app.expense_group_members (
                 id, org_id, group_id, user_id, room_id, display_name,
                 weight, joined_at, left_at, created_at, updated_at
             ) VALUES (
                 $1, $2, $3, $4, $5, $6,
                 $7::numeric, $8, $9, $10, $11
             )",
        )
        .bind(member.id().as_uuid())
        .bind(member.org_id().as_uuid())
        .bind(member.group_id().as_uuid())
        .bind(member.user_id().as_uuid())
        .bind(member.room_id().map(RoomId::as_uuid))
        .bind(member.display_name())
        .bind(member.weight().value())
        .bind(member.joined_at())
        .bind(member.left_at())
        .bind(member.created_at())
        .bind(member.updated_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn update_member(&mut self, member: &ExpenseGroupMember) -> RepositoryResult<()> {
        // Tre colonne: le due date dell'uscita e del rientro, e il peso.
        // `joined_at` è fra esse perché il rientro la riscrive —
        // `expense_group_members_dates_ck` è `left_at IS NULL OR left_at >=
        // joined_at`, e lasciare la vecchia data direbbe che la persona è nel
        // gruppo da prima di esserci tornata.
        let result = sqlx::query(
            "UPDATE app.expense_group_members
                SET left_at = $2,
                    joined_at = $3,
                    weight = $4::numeric
              WHERE id = $1",
        )
        .bind(member.id().as_uuid())
        .bind(member.left_at())
        .bind(member.joined_at())
        .bind(member.weight().value())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }

    async fn list_balances(
        &mut self,
        group_id: ExpenseGroupId,
    ) -> RepositoryResult<Vec<GroupBalance>> {
        // Dalla migration `0027` questo elenco comprende chi è **uscito** con un
        // saldo netto diverso da zero. Prima no, e il debito spariva con lui.
        //
        // `ORDER BY net_balance DESC, user_id`: i creditori prima, e a parità
        // l'ordine riproducibile della ripartizione. Un client che mostra «chi
        // deve a chi» legge dall'alto e dal basso.
        let rows = sqlx::query(
            "SELECT org_id,
                    group_id,
                    user_id,
                    total_paid::numeric        AS total_paid,
                    total_owed::numeric        AS total_owed,
                    total_settled_out::numeric AS total_settled_out,
                    total_settled_in::numeric  AS total_settled_in,
                    net_balance::numeric       AS net_balance
               FROM app.v_expense_group_balances
              WHERE group_id = $1
              ORDER BY net_balance DESC, user_id",
        )
        .bind(group_id.as_uuid())
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        rows.iter().map(map_balance).collect()
    }
}

fn invalid(field: &str, error: propflow_domain::DomainError) -> RepositoryError {
    RepositoryError::Backend {
        details: format!("{field} non valido in database: {error}"),
    }
}

fn map_group(row: &PgRow) -> RepositoryResult<ExpenseGroup> {
    let id: Uuid = try_get(row, "id")?;
    let org_id: Uuid = try_get(row, "org_id")?;
    let property_id: Option<Uuid> = try_get(row, "property_id")?;
    let kind: String = try_get(row, "kind")?;
    let currency: String = try_get(row, "currency")?;
    let default_split_method: String = try_get(row, "default_split_method")?;
    let created_by: Option<Uuid> = try_get(row, "created_by")?;

    Ok(ExpenseGroup::from_persistence(ExpenseGroupParts {
        id: ExpenseGroupId::from_uuid(id),
        org_id: OrgId::from_uuid(org_id),
        property_id: property_id.map(PropertyId::from_uuid),
        kind: db_enums::expense_group_kind_from_db(&kind)?,
        name: try_get(row, "name")?,
        currency: Currency::parse(&currency).map_err(|error| invalid("currency", error))?,
        default_split_method: db_enums::split_method_from_db(&default_split_method)?,
        created_by: created_by.map(UserId::from_uuid),
        is_archived: try_get(row, "is_archived")?,
        created_at: try_get(row, "created_at")?,
        updated_at: try_get(row, "updated_at")?,
    }))
}

fn map_member(row: &PgRow) -> RepositoryResult<ExpenseGroupMember> {
    let id: Uuid = try_get(row, "id")?;
    let org_id: Uuid = try_get(row, "org_id")?;
    let group_id: Uuid = try_get(row, "group_id")?;
    let user_id: Uuid = try_get(row, "user_id")?;
    let room_id: Option<Uuid> = try_get(row, "room_id")?;
    let weight: Decimal = try_get(row, "weight")?;

    Ok(ExpenseGroupMember::from_persistence(
        ExpenseGroupMemberParts {
            id: ExpenseGroupMemberId::from_uuid(id),
            org_id: OrgId::from_uuid(org_id),
            group_id: ExpenseGroupId::from_uuid(group_id),
            user_id: UserId::from_uuid(user_id),
            room_id: room_id.map(RoomId::from_uuid),
            display_name: try_get(row, "display_name")?,
            weight: Weight::from_persistence(weight),
            joined_at: try_get(row, "joined_at")?,
            left_at: try_get(row, "left_at")?,
            created_at: try_get(row, "created_at")?,
            updated_at: try_get(row, "updated_at")?,
        },
    ))
}

/// La valuta della vista non c'è, e non serve: ADR-030 la fissa a una sola, e la
/// vista somma importi di un gruppo che ne ha una per costruzione
/// (`expense_groups.currency`).
fn map_balance(row: &PgRow) -> RepositoryResult<GroupBalance> {
    let org_id: Uuid = try_get(row, "org_id")?;
    let group_id: Uuid = try_get(row, "group_id")?;
    let user_id: Uuid = try_get(row, "user_id")?;
    let currency = Currency::EUR;

    let money = |column: &'static str| -> RepositoryResult<Money> {
        let amount: Decimal = try_get(row, column)?;
        // `Money::new` e non `non_negative`: `net_balance` è **negativo** per chi
        // deve, ed è metà dell'informazione per cui la vista esiste.
        Money::new(amount, currency).map_err(|error| invalid(column, error))
    };

    Ok(GroupBalance {
        org_id: OrgId::from_uuid(org_id),
        group_id: ExpenseGroupId::from_uuid(group_id),
        user_id: UserId::from_uuid(user_id),
        total_paid: money("total_paid")?,
        total_owed: money("total_owed")?,
        total_settled_out: money("total_settled_out")?,
        total_settled_in: money("total_settled_in")?,
        net_balance: money("net_balance")?,
    })
}
