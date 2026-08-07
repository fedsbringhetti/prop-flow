use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::lease::{LeaseParty, LeasePartyParts};
use propflow_domain::ports::{LeasePartyRepository, RepositoryError, RepositoryResult};
use propflow_domain::shared::{Email, LeaseId, LeasePartyId, OrgId, UserId};

use super::user::try_get;
use crate::db::db_enums;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

macro_rules! select_lease_party {
    () => {
        "
    SELECT id,
           org_id,
           lease_id,
           user_id,
           role::text          AS role,
           full_name,
           email::text         AS email,
           phone,
           share_percent::numeric AS share_percent,
           is_primary,
           joined_at,
           left_at,
           created_at,
           updated_at,
           deleted_at
      FROM app.lease_parties
"
    };
}

#[async_trait]
impl LeasePartyRepository for PgUnitOfWork {
    async fn list_for_lease(&mut self, lease_id: LeaseId) -> RepositoryResult<Vec<LeaseParty>> {
        let rows = sqlx::query(concat!(
            select_lease_party!(),
            " WHERE lease_id = $1 AND deleted_at IS NULL ORDER BY is_primary DESC, id"
        ))
        .bind(lease_id.as_uuid())
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter().map(map_party).collect()
    }

    async fn insert(&mut self, party: &LeaseParty) -> RepositoryResult<()> {
        sqlx::query(
            "INSERT INTO app.lease_parties (
                 id, org_id, lease_id, user_id, role, full_name, email, phone,
                 share_percent, is_primary, joined_at, left_at, created_at, updated_at
             ) VALUES (
                 $1, $2, $3, $4, $5::app.lease_party_role, $6,
                 $7::app.email_address, $8,
                 $9::app.percentage, $10, $11, $12, $13, $14
             )",
        )
        .bind(party.id().as_uuid())
        .bind(party.org_id().as_uuid())
        .bind(party.lease_id().as_uuid())
        .bind(party.user_id().map(|id| id.as_uuid()))
        .bind(db_enums::lease_party_role_to_db(party.role()))
        .bind(party.full_name())
        .bind(party.email().map(Email::as_str))
        .bind(party.phone())
        .bind(party.share_percent())
        .bind(party.is_primary())
        .bind(party.joined_at())
        .bind(party.left_at())
        .bind(party.created_at())
        .bind(party.updated_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn update(&mut self, party: &LeaseParty) -> RepositoryResult<()> {
        let result = sqlx::query(
            "UPDATE app.lease_parties
                SET user_id       = $2,
                    role          = $3::app.lease_party_role,
                    full_name     = $4,
                    email         = $5::app.email_address,
                    phone         = $6,
                    share_percent = $7::app.percentage,
                    is_primary    = $8,
                    joined_at     = $9,
                    left_at       = $10
              WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(party.id().as_uuid())
        .bind(party.user_id().map(|id| id.as_uuid()))
        .bind(db_enums::lease_party_role_to_db(party.role()))
        .bind(party.full_name())
        .bind(party.email().map(Email::as_str))
        .bind(party.phone())
        .bind(party.share_percent())
        .bind(party.is_primary())
        .bind(party.joined_at())
        .bind(party.left_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }
}

fn map_party(row: PgRow) -> RepositoryResult<LeaseParty> {
    let id: Uuid = try_get(&row, "id")?;
    let org_id: Uuid = try_get(&row, "org_id")?;
    let lease_id: Uuid = try_get(&row, "lease_id")?;
    let user_id: Option<Uuid> = try_get(&row, "user_id")?;
    let role: String = try_get(&row, "role")?;
    let email: Option<String> = try_get(&row, "email")?;
    let share_percent: Decimal = try_get(&row, "share_percent")?;
    let joined_at: Option<NaiveDate> = try_get(&row, "joined_at")?;
    let left_at: Option<NaiveDate> = try_get(&row, "left_at")?;
    let created_at: DateTime<Utc> = try_get(&row, "created_at")?;
    let updated_at: DateTime<Utc> = try_get(&row, "updated_at")?;
    let archived_at: Option<DateTime<Utc>> = try_get(&row, "deleted_at")?;

    let email = email
        .as_deref()
        .map(Email::parse)
        .transpose()
        .map_err(|error| RepositoryError::Backend {
            details: format!("email non valida in database: {error}"),
        })?;

    Ok(LeaseParty::from_persistence(LeasePartyParts {
        id: LeasePartyId::from_uuid(id),
        org_id: OrgId::from_uuid(org_id),
        lease_id: LeaseId::from_uuid(lease_id),
        user_id: user_id.map(UserId::from_uuid),
        role: db_enums::lease_party_role_from_db(&role)?,
        full_name: try_get(&row, "full_name")?,
        email,
        phone: try_get(&row, "phone")?,
        share_percent,
        is_primary: try_get(&row, "is_primary")?,
        joined_at,
        left_at,
        created_at,
        updated_at,
        archived_at,
    }))
}
