use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use propflow_domain::identity::{Organization, Slug};
use propflow_domain::ports::{OrganizationRepository, RepositoryError, RepositoryResult};
use propflow_domain::shared::{CountryCode, Currency, OrgId};

use super::user::try_get;
use crate::db::db_enums;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

#[async_trait]
impl OrganizationRepository for PgUnitOfWork {
    async fn find_by_id(&mut self, id: OrgId) -> RepositoryResult<Option<Organization>> {
        let row = sqlx::query(
            "SELECT id,
                    name,
                    slug::text             AS slug,
                    kind::text             AS kind,
                    country::text          AS country,
                    timezone,
                    default_currency::text AS default_currency,
                    created_at
               FROM app.organizations
              WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id.as_uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        let Some(row) = row else {
            return Ok(None);
        };

        let invalid = |field: &str, error: propflow_domain::DomainError| RepositoryError::Backend {
            details: format!("{field} non valido in database: {error}"),
        };

        let id: Uuid = try_get(&row, "id")?;
        let slug: String = try_get(&row, "slug")?;
        let kind: String = try_get(&row, "kind")?;
        let country: String = try_get(&row, "country")?;
        let currency: String = try_get(&row, "default_currency")?;
        let created_at: DateTime<Utc> = try_get(&row, "created_at")?;

        Ok(Some(Organization::from_persistence(
            OrgId::from_uuid(id),
            try_get(&row, "name")?,
            Slug::parse(&slug).map_err(|error| invalid("slug", error))?,
            db_enums::org_kind_from_db(&kind)?,
            CountryCode::parse(&country).map_err(|error| invalid("country", error))?,
            try_get(&row, "timezone")?,
            Currency::parse(&currency).map_err(|error| invalid("default_currency", error))?,
            created_at,
        )))
    }

    async fn insert(&mut self, organization: &Organization) -> RepositoryResult<()> {
        sqlx::query(
            "INSERT INTO app.organizations (
                 id, name, slug, kind, country, timezone, default_currency, created_at
             ) VALUES (
                 $1, $2, $3::citext, $4::app.org_kind, $5::app.country_code, $6,
                 $7::app.currency_code, $8
             )",
        )
        .bind(organization.id().as_uuid())
        .bind(organization.name())
        .bind(organization.slug().as_str())
        .bind(db_enums::org_kind_to_db(organization.kind()))
        .bind(organization.country().as_str())
        .bind(organization.timezone())
        .bind(organization.default_currency().as_str())
        .bind(organization.created_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }
}
