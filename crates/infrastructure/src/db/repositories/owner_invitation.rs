use async_trait::async_trait;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::identity::{OwnerInvitation, OwnerInvitationParts, TokenHash};
use propflow_domain::ports::{OwnerInvitationRepository, RepositoryError, RepositoryResult};
use propflow_domain::shared::{Email, OwnerInvitationId, UserId};

use super::user::try_get;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

macro_rules! select_owner_invitation {
    () => {
        "
    SELECT id,
           email::text AS email,
           token_hash,
           expires_at,
           accepted_at,
           accepted_by,
           revoked_at,
           created_at
      FROM app.owner_invitations
"
    };
}

#[async_trait]
impl OwnerInvitationRepository for PgUnitOfWork {
    async fn find_pending_by_hash(
        &mut self,
        token_hash: &TokenHash,
    ) -> RepositoryResult<Option<OwnerInvitation>> {
        // Nessun filtro di stato: come `InvitationRepository::find_by_token_hash`,
        // è `OwnerInvitation::ensure_acceptable` a decidere se la riga letta è
        // ancora spendibile. `token_hash` è unico (`owner_invitations_token_uk`),
        // quindi la riga — se esiste — è una sola.
        let row = sqlx::query(concat!(
            select_owner_invitation!(),
            " WHERE token_hash = $1"
        ))
        .bind(token_hash.as_bytes().as_slice())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(map_owner_invitation).transpose()
    }

    async fn mark_accepted(&mut self, invitation: &OwnerInvitation) -> RepositoryResult<()> {
        // `accepted_at IS NULL` nella `WHERE`, come
        // `PasswordResetCodeRepository::mark_used`: due richieste concorrenti con
        // lo stesso token passerebbero entrambe la verifica in memoria (fatta dal
        // caso d'uso su una copia letta prima), e a decidere quale vince è
        // questa riga.
        let result = sqlx::query(
            "UPDATE app.owner_invitations
                SET accepted_at = $2,
                    accepted_by = $3
              WHERE id = $1 AND accepted_at IS NULL",
        )
        .bind(invitation.id().as_uuid())
        .bind(invitation.accepted_at())
        .bind(invitation.accepted_by().map(|id| id.as_uuid()))
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }
}

fn map_owner_invitation(row: PgRow) -> RepositoryResult<OwnerInvitation> {
    let invalid = |field: &str, error: propflow_domain::DomainError| RepositoryError::Backend {
        details: format!("{field} non valido in database: {error}"),
    };

    let id: Uuid = try_get(&row, "id")?;
    let email: String = try_get(&row, "email")?;
    let token_hash: Vec<u8> = try_get(&row, "token_hash")?;
    let accepted_by: Option<Uuid> = try_get(&row, "accepted_by")?;

    Ok(OwnerInvitation::from_persistence(OwnerInvitationParts {
        id: OwnerInvitationId::from_uuid(id),
        email: Email::parse(&email).map_err(|error| invalid("email", error))?,
        token_hash: TokenHash::from_slice(&token_hash)
            .map_err(|error| invalid("token_hash", error))?,
        expires_at: try_get(&row, "expires_at")?,
        accepted_at: try_get(&row, "accepted_at")?,
        accepted_by: accepted_by.map(UserId::from_uuid),
        revoked_at: try_get(&row, "revoked_at")?,
        created_at: try_get(&row, "created_at")?,
    }))
}
