use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::identity::{Invitation, InvitationParts, TokenHash};
use propflow_domain::ports::{InvitationRepository, RepositoryError, RepositoryResult};
use propflow_domain::shared::{Email, InvitationId, LeaseId, OrgId, UserId};

use super::user::try_get;
use crate::db::db_enums;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

macro_rules! select_invitation {
    () => {
        "
    SELECT id,
           org_id,
           email::text AS email,
           role::text  AS role,
           token_hash,
           lease_id,
           invited_by,
           expires_at,
           accepted_at,
           accepted_by,
           revoked_at,
           created_at
      FROM app.org_invitations
"
    };
}

#[async_trait]
impl InvitationRepository for PgUnitOfWork {
    async fn find_by_id(&mut self, id: InvitationId) -> RepositoryResult<Option<Invitation>> {
        // Nessun filtro su `deleted_at`: `org_invitations` non ha soft delete. Un
        // invito si revoca, e la revoca è uno stato dell'invito, non una
        // cancellazione — l'audit deve poter dire chi ha invitato chi e quando.
        let row = sqlx::query(concat!(select_invitation!(), " WHERE id = $1"))
            .bind(id.as_uuid())
            .fetch_optional(&mut *self.tx)
            .await
            .map_err(map_sqlx_error)?;
        row.map(map_invitation).transpose()
    }

    async fn find_by_token_hash(
        &mut self,
        token_hash: &TokenHash,
    ) -> RepositoryResult<Option<Invitation>> {
        let row = sqlx::query(concat!(select_invitation!(), " WHERE token_hash = $1"))
            .bind(token_hash.as_bytes().as_slice())
            .fetch_optional(&mut *self.tx)
            .await
            .map_err(map_sqlx_error)?;
        row.map(map_invitation).transpose()
    }

    async fn insert(&mut self, invitation: &Invitation) -> RepositoryResult<()> {
        sqlx::query(
            "INSERT INTO app.org_invitations (
                 id, org_id, email, role, token_hash, lease_id, invited_by,
                 expires_at, created_at
             ) VALUES (
                 $1, $2, $3::app.email_address, $4::app.org_role, $5, $6, $7, $8, $9
             )",
        )
        .bind(invitation.id().as_uuid())
        .bind(invitation.org_id().as_uuid())
        .bind(invitation.email().as_str())
        .bind(db_enums::org_role_to_db(invitation.role()))
        .bind(invitation.token_hash().as_bytes().as_slice())
        .bind(invitation.lease_id().map(|id| id.as_uuid()))
        .bind(invitation.invited_by().map(|id| id.as_uuid()))
        .bind(invitation.expires_at())
        .bind(invitation.created_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn update(&mut self, invitation: &Invitation) -> RepositoryResult<()> {
        // Si aggiornano soltanto gli esiti: email, ruolo e token di un invito
        // emesso non cambiano, e permettere di riscriverli significherebbe poter
        // dirottare un invito già spedito.
        let result = sqlx::query(
            "UPDATE app.org_invitations
                SET accepted_at = $2,
                    accepted_by = $3,
                    revoked_at  = $4
              WHERE id = $1",
        )
        .bind(invitation.id().as_uuid())
        .bind(invitation.accepted_at())
        .bind(invitation.accepted_by().map(|id| id.as_uuid()))
        .bind(invitation.revoked_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }
}

fn map_invitation(row: PgRow) -> RepositoryResult<Invitation> {
    let invalid = |field: &str, error: propflow_domain::DomainError| RepositoryError::Backend {
        details: format!("{field} non valido in database: {error}"),
    };

    let id: Uuid = try_get(&row, "id")?;
    let org_id: Uuid = try_get(&row, "org_id")?;
    let email: String = try_get(&row, "email")?;
    let role: String = try_get(&row, "role")?;
    let token_hash: Vec<u8> = try_get(&row, "token_hash")?;
    let lease_id: Option<Uuid> = try_get(&row, "lease_id")?;
    let invited_by: Option<Uuid> = try_get(&row, "invited_by")?;
    let accepted_by: Option<Uuid> = try_get(&row, "accepted_by")?;
    let expires_at: DateTime<Utc> = try_get(&row, "expires_at")?;
    let created_at: DateTime<Utc> = try_get(&row, "created_at")?;

    Ok(Invitation::from_persistence(InvitationParts {
        id: InvitationId::from_uuid(id),
        org_id: OrgId::from_uuid(org_id),
        email: Email::parse(&email).map_err(|error| invalid("email", error))?,
        role: db_enums::org_role_from_db(&role)?,
        token_hash: TokenHash::from_slice(&token_hash)
            .map_err(|error| invalid("token_hash", error))?,
        lease_id: lease_id.map(LeaseId::from_uuid),
        invited_by: invited_by.map(UserId::from_uuid),
        expires_at,
        accepted_at: try_get(&row, "accepted_at")?,
        accepted_by: accepted_by.map(UserId::from_uuid),
        revoked_at: try_get(&row, "revoked_at")?,
        created_at,
    }))
}
