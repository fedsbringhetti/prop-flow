use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::identity::{OrgMember, OrganizationMembership, Slug};
use propflow_domain::ports::{OrgMemberRepository, RepositoryError, RepositoryResult};
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

    /// Il join con `app.organizations`, e non una lettura per ogni appartenenza.
    ///
    /// `organizations_select` è `app.is_org_member(id)` e `org_members_select` è
    /// `user_id = app.current_user_id() OR app.is_org_member(org_id)`: **nessuna
    /// delle due guarda `app.current_org_id()`**, quindi questa query risponde
    /// anche a chi non ha un'organizzazione attiva — che è esattamente il caso per
    /// cui l'endpoint esiste.
    ///
    /// `deleted_at IS NULL` va scritto su **entrambe** le tabelle: dalla migration
    /// `0016` le policy di SELECT non filtrano più il soft delete, che è un filtro
    /// di business e vive nel `WHERE` (ADR-008).
    async fn list_organizations_for_user(
        &mut self,
        user_id: UserId,
    ) -> RepositoryResult<Vec<OrganizationMembership>> {
        let rows = sqlx::query(
            "SELECT o.id           AS org_id,
                    o.name         AS name,
                    o.slug         AS slug,
                    m.role::text   AS role
               FROM app.org_members m
               JOIN app.organizations o ON o.id = m.org_id
              WHERE m.user_id = $1
                -- Solo le attive, e non per prudenza: `app.member_role` esige
                -- `status = 'active'`, quindi un'appartenenza sospesa o lasciata
                -- comparirebbe nel menù e poi non concederebbe una sola riga.
                AND m.status = 'active'
                AND m.deleted_at IS NULL
                AND o.deleted_at IS NULL
              ORDER BY m.created_at",
        )
        .bind(user_id.as_uuid())
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter().map(map_membership).collect()
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

    async fn update(&mut self, member: &OrgMember) -> RepositoryResult<()> {
        // **Due colonne, ed è l'elenco che la `0039` concede.** Nominare `role`
        // farebbe fallire la query con `permission denied for table org_members`,
        // ed è la difesa che si vuole: uscire e promuoversi a `owner` sarebbero
        // altrimenti la stessa `UPDATE`, e la `WITH CHECK` non può accorgersene
        // perché non vede la riga vecchia.
        //
        // `updated_at` non compare perché la scrive il trigger `set_updated_at`.
        let result = sqlx::query(
            "UPDATE app.org_members
                SET status    = $3::app.member_status,
                    joined_at = $4
              WHERE org_id = $1 AND user_id = $2 AND deleted_at IS NULL",
        )
        .bind(member.org_id().as_uuid())
        .bind(member.user_id().as_uuid())
        .bind(db_enums::member_status_to_db(member.status()))
        .bind(member.joined_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        // Zero righe qui significa che una `WITH CHECK` ha rifiutato: chi chiama ha
        // appena letto la riga nella stessa transazione, quindi esiste. Il caso
        // vero è l'uscita con un contratto in essere, che `org_members_update_self`
        // nega — e il caso d'uso lo intercetta **prima**, così che questo rifiuto
        // resti quello che deve essere: il segnale di un difetto nostro.
        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }

    async fn count_active_owners(&mut self, org_id: OrgId) -> RepositoryResult<u64> {
        // `org_members_select` è `user_id = current_user_id() OR is_org_member(org_id)`:
        // un membro qualunque vede le righe della propria organizzazione, quindi
        // anche un inquilino che sta uscendo ottiene la conta vera.
        let row: (i64,) = sqlx::query_as(
            "SELECT count(*)
               FROM app.org_members
              WHERE org_id = $1
                AND role = 'owner'
                AND status = 'active'
                AND deleted_at IS NULL",
        )
        .bind(org_id.as_uuid())
        .fetch_one(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(u64::try_from(row.0).unwrap_or(0))
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

fn map_membership(row: PgRow) -> RepositoryResult<OrganizationMembership> {
    let org_id: Uuid = try_get(&row, "org_id")?;
    let name: String = try_get(&row, "name")?;
    let slug: String = try_get(&row, "slug")?;
    let role: String = try_get(&row, "role")?;

    Ok(OrganizationMembership::from_persistence(
        OrgId::from_uuid(org_id),
        name,
        // Stessa lettura del repository delle organizzazioni: uno slug che il
        // database contiene e il dominio non riconosce è un dato illeggibile, non
        // una validazione fallita — chi legge non ha nulla da correggere.
        Slug::parse(&slug).map_err(|error| RepositoryError::Backend {
            details: format!("slug non valido in database: {error}"),
        })?,
        db_enums::org_role_from_db(&role)?,
    ))
}
