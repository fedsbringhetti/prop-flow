use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::identity::OrgMember;
use propflow_domain::ports::{OrgMemberRepository, RepositoryResult};
use propflow_domain::shared::{OrgId, UserId};

use super::user::try_get;
use crate::db::db_enums;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

macro_rules! select_member {
    () => {
        "
    SELECT org_id,
           user_id,
           role::text   AS role,
           status::text AS status,
           joined_at
      FROM app.org_members
"
    };
}

#[async_trait]
impl OrgMemberRepository for PgUnitOfWork {
    async fn find(
        &mut self,
        org_id: OrgId,
        user_id: UserId,
    ) -> RepositoryResult<Option<OrgMember>> {
        let row = sqlx::query(concat!(
            select_member!(),
            " WHERE org_id = $1 AND user_id = $2 AND deleted_at IS NULL"
        ))
        .bind(org_id.as_uuid())
        .bind(user_id.as_uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(map_member).transpose()
    }

    async fn list_for_user(&mut self, user_id: UserId) -> RepositoryResult<Vec<OrgMember>> {
        let rows = sqlx::query(concat!(
            select_member!(),
            " WHERE user_id = $1 AND deleted_at IS NULL ORDER BY created_at"
        ))
        .bind(user_id.as_uuid())
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter().map(map_member).collect()
    }

    async fn insert(&mut self, member: &OrgMember) -> RepositoryResult<()> {
        sqlx::query(
            "INSERT INTO app.org_members (org_id, user_id, role, status, joined_at)
             VALUES ($1, $2, $3::app.org_role, $4::app.member_status, $5)",
        )
        .bind(member.org_id().as_uuid())
        .bind(member.user_id().as_uuid())
        .bind(db_enums::org_role_to_db(member.role()))
        .bind(db_enums::member_status_to_db(member.status()))
        .bind(member.joined_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }
}

fn map_member(row: PgRow) -> RepositoryResult<OrgMember> {
    let org_id: Uuid = try_get(&row, "org_id")?;
    let user_id: Uuid = try_get(&row, "user_id")?;
    let role: String = try_get(&row, "role")?;
    let status: String = try_get(&row, "status")?;
    let joined_at: Option<DateTime<Utc>> = try_get(&row, "joined_at")?;

    Ok(OrgMember::from_persistence(
        OrgId::from_uuid(org_id),
        UserId::from_uuid(user_id),
        db_enums::org_role_from_db(&role)?,
        db_enums::member_status_from_db(&status)?,
        joined_at,
    ))
}
