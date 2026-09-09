use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::lease::{Lease, LeaseParts};
use propflow_domain::ports::{
    LeaseLabels, LeasePage, LeaseQuery, LeaseRepository, RepositoryError, RepositoryResult,
};
use propflow_domain::shared::{Currency, LeaseId, Money, OrgId, PropertyId, RoomId};

use super::user::try_get;
use crate::db::db_enums;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

macro_rules! select_lease {
    () => {
        "
    SELECT id,
           org_id,
           property_id,
           room_id,
           lease_type::text          AS lease_type,
           status::text              AS status,
           start_date,
           end_date,
           notice_months,
           auto_renew,
           renewal_months,
           rent_amount::numeric      AS rent_amount,
           currency::text            AS currency,
           rent_frequency::text      AS rent_frequency,
           rent_due_day,
           deposit_amount::numeric   AS deposit_amount,
           utilities_included,
           istat_indexation,
           signed_at,
           terminated_at,
           termination_reason,
           notes,
           created_at,
           updated_at,
           deleted_at
      FROM app.leases
"
    };
}

#[async_trait]
impl LeaseRepository for PgUnitOfWork {
    async fn find_by_id(&mut self, id: LeaseId) -> RepositoryResult<Option<Lease>> {
        let row = sqlx::query(concat!(
            select_lease!(),
            " WHERE id = $1 AND deleted_at IS NULL"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(map_lease).transpose()
    }

    async fn list(&mut self, query: &LeaseQuery) -> RepositoryResult<LeasePage> {
        // `in_force` usa `app.lease_in_force`, la stessa funzione delle viste: due
        // definizioni di "in essere" farebbero rispondere l'elenco e la dashboard
        // in modo diverso sullo stesso contratto.
        let fetch_limit = i64::from(query.limit) + 1;
        let rows = sqlx::query(concat!(
            select_lease!(),
            " WHERE deleted_at IS NULL
                AND ($1::uuid IS NULL OR property_id = $1)
                AND ($2::text IS NULL OR status = $2::app.lease_status)
                AND ($3::bool IS NULL
                     OR app.lease_in_force(status, start_date, end_date) = $3)
                AND ($4::uuid IS NULL OR id > $4)
              ORDER BY id
              LIMIT $5"
        ))
        .bind(query.property_id.map(|id| id.as_uuid()))
        .bind(query.status.map(db_enums::lease_status_to_db))
        .bind(query.in_force)
        .bind(query.after.map(|id| id.as_uuid()))
        .bind(fetch_limit)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        let has_more = rows.len() > usize::from(query.limit);
        let items = rows
            .into_iter()
            .take(usize::from(query.limit))
            .map(map_lease)
            .collect::<RepositoryResult<Vec<Lease>>>()?;
        let next_cursor = if has_more {
            items.last().map(Lease::id)
        } else {
            None
        };

        Ok(LeasePage { items, next_cursor })
    }

    async fn insert(&mut self, lease: &Lease) -> RepositoryResult<()> {
        sqlx::query(
            "INSERT INTO app.leases (
                 id, org_id, property_id, room_id, lease_type, status,
                 start_date, end_date, notice_months, auto_renew, renewal_months,
                 rent_amount, currency, rent_frequency, rent_due_day, deposit_amount,
                 utilities_included, istat_indexation, signed_at, terminated_at,
                 termination_reason, notes, created_at, updated_at
             ) VALUES (
                 $1, $2, $3, $4, $5::app.lease_type, $6::app.lease_status,
                 $7, $8, $9, $10, $11,
                 $12::app.money_amount, $13::app.currency_code, $14::app.rent_frequency,
                 $15, $16::app.money_amount,
                 $17, $18, $19, $20,
                 $21, $22, $23, $24
             )",
        )
        .bind(lease.id().as_uuid())
        .bind(lease.org_id().as_uuid())
        .bind(lease.property_id().as_uuid())
        .bind(lease.room_id().map(|id| id.as_uuid()))
        .bind(db_enums::lease_type_to_db(lease.lease_type()))
        .bind(db_enums::lease_status_to_db(lease.status()))
        .bind(lease.start_date())
        .bind(lease.end_date())
        .bind(lease.notice_months())
        .bind(lease.auto_renew())
        .bind(lease.renewal_months())
        .bind(lease.rent().amount())
        .bind(lease.currency().as_str())
        .bind(db_enums::rent_frequency_to_db(lease.rent_frequency()))
        .bind(lease.rent_due_day())
        .bind(lease.deposit().amount())
        .bind(lease.utilities_included())
        .bind(lease.istat_indexation())
        .bind(lease.signed_at())
        .bind(lease.terminated_at())
        .bind(lease.termination_reason())
        .bind(lease.notes())
        .bind(lease.created_at())
        .bind(lease.updated_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn update(&mut self, lease: &Lease) -> RepositoryResult<()> {
        // `property_id`, `room_id` e `lease_type` non si aggiornano: cambiare
        // l'oggetto di un contratto non è una modifica, è un contratto diverso.
        let result = sqlx::query(
            "UPDATE app.leases
                SET status             = $2::app.lease_status,
                    start_date         = $3,
                    end_date           = $4,
                    notice_months      = $5,
                    auto_renew         = $6,
                    renewal_months     = $7,
                    rent_amount        = $8::app.money_amount,
                    rent_frequency     = $9::app.rent_frequency,
                    rent_due_day       = $10,
                    deposit_amount     = $11::app.money_amount,
                    utilities_included = $12,
                    istat_indexation   = $13,
                    signed_at          = $14,
                    terminated_at      = $15,
                    termination_reason = $16,
                    notes              = $17
              WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(lease.id().as_uuid())
        .bind(db_enums::lease_status_to_db(lease.status()))
        .bind(lease.start_date())
        .bind(lease.end_date())
        .bind(lease.notice_months())
        .bind(lease.auto_renew())
        .bind(lease.renewal_months())
        .bind(lease.rent().amount())
        .bind(db_enums::rent_frequency_to_db(lease.rent_frequency()))
        .bind(lease.rent_due_day())
        .bind(lease.deposit().amount())
        .bind(lease.utilities_included())
        .bind(lease.istat_indexation())
        .bind(lease.signed_at())
        .bind(lease.terminated_at())
        .bind(lease.termination_reason())
        .bind(lease.notes())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }

    async fn soft_delete(&mut self, lease: &Lease) -> RepositoryResult<()> {
        let result = sqlx::query(
            "UPDATE app.leases SET deleted_at = $2 WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(lease.id().as_uuid())
        .bind(lease.archived_at().unwrap_or_else(Utc::now))
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }

    async fn labels(&mut self, ids: &[LeaseId]) -> RepositoryResult<Vec<LeaseLabels>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        // **`LEFT JOIN` su entrambi, e non è pignoleria.** Con un `INNER` un
        // contratto sull'immobile intero — che non ha stanza — sparirebbe
        // dall'elenco, e con lui ogni contratto il cui immobile è stato
        // archiviato: due modi di far scomparire righe che esistono, per andare a
        // prendere un nome.
        let uuids: Vec<Uuid> = ids.iter().map(|id| id.as_uuid()).collect();
        let rows = sqlx::query(
            "SELECT l.id                AS lease_id,
                    p.label             AS property_label,
                    r.name              AS room_name
               FROM app.leases l
               LEFT JOIN app.properties p
                      ON p.id = l.property_id AND p.deleted_at IS NULL
               LEFT JOIN app.rooms r
                      ON r.id = l.room_id AND r.deleted_at IS NULL
              WHERE l.id = ANY($1) AND l.deleted_at IS NULL",
        )
        .bind(&uuids)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        rows.into_iter()
            .map(|row| {
                let lease_id: Uuid = try_get(&row, "lease_id")?;
                Ok(LeaseLabels {
                    lease_id: LeaseId::from_uuid(lease_id),
                    property_label: try_get(&row, "property_label")?,
                    room_name: try_get(&row, "room_name")?,
                })
            })
            .collect()
    }
}

fn map_lease(row: PgRow) -> RepositoryResult<Lease> {
    let invalid = |field: &str, error: propflow_domain::DomainError| RepositoryError::Backend {
        details: format!("{field} non valido in database: {error}"),
    };

    let id: Uuid = try_get(&row, "id")?;
    let org_id: Uuid = try_get(&row, "org_id")?;
    let property_id: Uuid = try_get(&row, "property_id")?;
    let room_id: Option<Uuid> = try_get(&row, "room_id")?;
    let lease_type: String = try_get(&row, "lease_type")?;
    let status: String = try_get(&row, "status")?;
    let currency_code: String = try_get(&row, "currency")?;
    let rent_frequency: String = try_get(&row, "rent_frequency")?;
    let rent_amount: Decimal = try_get(&row, "rent_amount")?;
    let deposit_amount: Decimal = try_get(&row, "deposit_amount")?;
    let start_date: NaiveDate = try_get(&row, "start_date")?;
    let end_date: Option<NaiveDate> = try_get(&row, "end_date")?;
    let created_at: DateTime<Utc> = try_get(&row, "created_at")?;
    let updated_at: DateTime<Utc> = try_get(&row, "updated_at")?;
    let archived_at: Option<DateTime<Utc>> = try_get(&row, "deleted_at")?;

    let currency = Currency::parse(&currency_code).map_err(|error| invalid("currency", error))?;
    let rent = Money::new(rent_amount, currency).map_err(|error| invalid("rent_amount", error))?;
    let deposit =
        Money::new(deposit_amount, currency).map_err(|error| invalid("deposit_amount", error))?;

    Ok(Lease::from_persistence(LeaseParts {
        id: LeaseId::from_uuid(id),
        org_id: OrgId::from_uuid(org_id),
        property_id: PropertyId::from_uuid(property_id),
        room_id: room_id.map(RoomId::from_uuid),
        lease_type: db_enums::lease_type_from_db(&lease_type)?,
        status: db_enums::lease_status_from_db(&status)?,
        start_date,
        end_date,
        notice_months: try_get(&row, "notice_months")?,
        auto_renew: try_get(&row, "auto_renew")?,
        renewal_months: try_get(&row, "renewal_months")?,
        rent,
        rent_frequency: db_enums::rent_frequency_from_db(&rent_frequency)?,
        rent_due_day: try_get(&row, "rent_due_day")?,
        deposit,
        utilities_included: try_get(&row, "utilities_included")?,
        istat_indexation: try_get(&row, "istat_indexation")?,
        signed_at: try_get(&row, "signed_at")?,
        terminated_at: try_get(&row, "terminated_at")?,
        termination_reason: try_get(&row, "termination_reason")?,
        notes: try_get(&row, "notes")?,
        created_at,
        updated_at,
        archived_at,
    }))
}
