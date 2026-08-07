use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::identity::{RefreshSession, TokenHash};
use propflow_domain::ports::{RepositoryError, RepositoryResult, SessionRepository};
use propflow_domain::shared::{SessionId, UserId};

use super::user::try_get;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

#[async_trait]
impl SessionRepository for PgUnitOfWork {
    async fn find_by_token_hash(
        &mut self,
        token_hash: &TokenHash,
    ) -> RepositoryResult<Option<RefreshSession>> {
        let row = sqlx::query(
            "SELECT id, user_id, refresh_token_hash, issued_at, expires_at, revoked_at, rotated_to
               FROM app.user_sessions
              WHERE refresh_token_hash = $1",
        )
        .bind(token_hash.as_bytes().as_slice())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(map_session).transpose()
    }

    async fn insert(&mut self, session: &RefreshSession) -> RepositoryResult<()> {
        sqlx::query(
            "INSERT INTO app.user_sessions (
                 id, user_id, refresh_token_hash, issued_at, expires_at, revoked_at, rotated_to
             ) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(session.id().as_uuid())
        .bind(session.user_id().as_uuid())
        .bind(session.token_hash().as_bytes().as_slice())
        .bind(session.issued_at())
        .bind(session.expires_at())
        .bind(session.revoked_at())
        .bind(session.rotated_to().map(|id| id.as_uuid()))
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn update(&mut self, session: &RefreshSession) -> RepositoryResult<()> {
        let result = sqlx::query(
            "UPDATE app.user_sessions
                SET revoked_at = $2,
                    rotated_to = $3,
                    revoked_reason = COALESCE(revoked_reason, $4)
              WHERE id = $1",
        )
        .bind(session.id().as_uuid())
        .bind(session.revoked_at())
        .bind(session.rotated_to().map(|id| id.as_uuid()))
        .bind(if session.rotated_to().is_some() {
            Some("rotated")
        } else if session.revoked_at().is_some() {
            Some("logout")
        } else {
            None
        })
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::Backend {
                details: "aggiornamento sessione senza effetto: riga assente o non visibile"
                    .to_owned(),
            });
        }
        Ok(())
    }

    async fn revoke_all_for_user(&mut self, user_id: UserId) -> RepositoryResult<u64> {
        let result = sqlx::query(
            "UPDATE app.user_sessions
                SET revoked_at = now(),
                    revoked_reason = 'refresh token reuse detected'
              WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(user_id.as_uuid())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(result.rows_affected())
    }
}

fn map_session(row: PgRow) -> RepositoryResult<RefreshSession> {
    let id: Uuid = try_get(&row, "id")?;
    let user_id: Uuid = try_get(&row, "user_id")?;
    let hash: Vec<u8> = try_get(&row, "refresh_token_hash")?;
    let issued_at: DateTime<Utc> = try_get(&row, "issued_at")?;
    let expires_at: DateTime<Utc> = try_get(&row, "expires_at")?;
    let revoked_at: Option<DateTime<Utc>> = try_get(&row, "revoked_at")?;
    let rotated_to: Option<Uuid> = try_get(&row, "rotated_to")?;

    let token_hash = TokenHash::from_slice(&hash).map_err(|error| RepositoryError::Backend {
        details: format!("hash del refresh token non valido in database: {error}"),
    })?;

    Ok(RefreshSession::from_persistence(
        SessionId::from_uuid(id),
        UserId::from_uuid(user_id),
        token_hash,
        issued_at,
        expires_at,
        revoked_at,
        rotated_to.map(SessionId::from_uuid),
    ))
}
