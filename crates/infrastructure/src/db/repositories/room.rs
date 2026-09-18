use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::ports::{RepositoryError, RepositoryResult, RoomOccupancy, RoomRepository};
use propflow_domain::property::{Room, RoomParts};
use propflow_domain::shared::{Currency, Money, OrgId, PropertyId, RoomId};

use super::user::try_get;
use crate::db::db_enums;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

macro_rules! select_room {
    () => {
        "
    SELECT id,
           org_id,
           property_id,
           name,
           status::text                   AS status,
           size_sqm::numeric              AS size_sqm,
           floor,
           has_private_bathroom,
           is_furnished,
           monthly_rent::numeric          AS monthly_rent,
           currency::text                 AS currency,
           utility_share_percent::numeric AS utility_share_percent,
           created_at,
           updated_at,
           deleted_at
      FROM app.rooms
"
    };
}

#[async_trait]
impl RoomRepository for PgUnitOfWork {
    async fn find_by_id(&mut self, id: RoomId) -> RepositoryResult<Option<Room>> {
        let row = sqlx::query(concat!(
            select_room!(),
            " WHERE id = $1 AND deleted_at IS NULL"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(map_room).transpose()
    }

    async fn list_for_property(&mut self, property_id: PropertyId) -> RepositoryResult<Vec<Room>> {
        // Ordine per id: gli id sono UUIDv7, quindi equivale all'ordine di
        // creazione senza una colonna in più da indicizzare.
        let rows = sqlx::query(concat!(
            select_room!(),
            " WHERE property_id = $1 AND deleted_at IS NULL ORDER BY id"
        ))
        .bind(property_id.as_uuid())
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        rows.into_iter().map(map_room).collect()
    }

    async fn occupancy(&mut self, ids: &[RoomId]) -> RepositoryResult<Vec<RoomOccupancy>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        // La definizione sta **nel database**, in `app.room_is_occupied`, e usa
        // `app.lease_in_force`: qui non si confrontano date, o sarebbe la seconda
        // definizione di «in essere» che la `0030` esiste per impedire.
        let uuids: Vec<Uuid> = ids.iter().map(|id| id.as_uuid()).collect();
        let rows = sqlx::query(
            "SELECT id, app.room_is_occupied(id) AS occupied
               FROM app.rooms
              WHERE id = ANY($1) AND deleted_at IS NULL",
        )
        .bind(&uuids)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        rows.into_iter()
            .map(|row| {
                let id: Uuid = try_get(&row, "id")?;
                Ok(RoomOccupancy {
                    room_id: RoomId::from_uuid(id),
                    occupied: try_get::<Option<bool>>(&row, "occupied")?.unwrap_or(false),
                })
            })
            .collect()
    }

    async fn insert(&mut self, room: &Room) -> RepositoryResult<()> {
        sqlx::query(
            "INSERT INTO app.rooms (
                 id, org_id, property_id, name, status, size_sqm, floor,
                 has_private_bathroom, is_furnished, monthly_rent, currency,
                 utility_share_percent, created_at, updated_at
             ) VALUES (
                 $1, $2, $3, $4, $5::app.room_status, $6, $7,
                 $8, $9, $10::app.money_amount, $11::app.currency_code,
                 $12::app.percentage, $13, $14
             )",
        )
        .bind(room.id().as_uuid())
        .bind(room.org_id().as_uuid())
        .bind(room.property_id().as_uuid())
        .bind(room.name())
        .bind(db_enums::room_status_to_db(room.status()))
        .bind(room.size_sqm())
        .bind(room.floor())
        .bind(room.has_private_bathroom())
        .bind(room.is_furnished())
        .bind(room.monthly_rent().map(|money| money.amount()))
        .bind(room.currency().as_str())
        .bind(room.utility_share_percent())
        .bind(room.created_at())
        .bind(room.updated_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn update(&mut self, room: &Room) -> RepositoryResult<()> {
        let result = sqlx::query(
            "UPDATE app.rooms
                SET name                  = $2,
                    status                = $3::app.room_status,
                    size_sqm              = $4,
                    floor                 = $5,
                    has_private_bathroom  = $6,
                    is_furnished          = $7,
                    monthly_rent          = $8::app.money_amount,
                    utility_share_percent = $9::app.percentage
              WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(room.id().as_uuid())
        .bind(room.name())
        .bind(db_enums::room_status_to_db(room.status()))
        .bind(room.size_sqm())
        .bind(room.floor())
        .bind(room.has_private_bathroom())
        .bind(room.is_furnished())
        .bind(room.monthly_rent().map(|money| money.amount()))
        .bind(room.utility_share_percent())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        // Zero righe con RLS attiva significa "non visibile a questo contesto":
        // il service layer avrebbe dovuto fermarsi prima, quindi è un difetto.
        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }

    async fn soft_delete(&mut self, room: &Room) -> RepositoryResult<()> {
        let result = sqlx::query(
            "UPDATE app.rooms SET deleted_at = $2 WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(room.id().as_uuid())
        .bind(room.archived_at().unwrap_or_else(Utc::now))
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }
}

fn map_room(row: PgRow) -> RepositoryResult<Room> {
    let invalid = |field: &str, error: propflow_domain::DomainError| RepositoryError::Backend {
        details: format!("{field} non valido in database: {error}"),
    };

    let id: Uuid = try_get(&row, "id")?;
    let org_id: Uuid = try_get(&row, "org_id")?;
    let property_id: Uuid = try_get(&row, "property_id")?;
    let status: String = try_get(&row, "status")?;
    let currency_code: String = try_get(&row, "currency")?;
    let monthly_rent: Option<Decimal> = try_get(&row, "monthly_rent")?;
    let created_at: DateTime<Utc> = try_get(&row, "created_at")?;
    let updated_at: DateTime<Utc> = try_get(&row, "updated_at")?;
    let archived_at: Option<DateTime<Utc>> = try_get(&row, "deleted_at")?;

    let currency = Currency::parse(&currency_code).map_err(|error| invalid("currency", error))?;
    let monthly_rent = monthly_rent
        .map(|amount| Money::new(amount, currency))
        .transpose()
        .map_err(|error| invalid("monthly_rent", error))?;

    Ok(Room::from_persistence(RoomParts {
        id: RoomId::from_uuid(id),
        org_id: OrgId::from_uuid(org_id),
        property_id: PropertyId::from_uuid(property_id),
        name: try_get(&row, "name")?,
        status: db_enums::room_status_from_db(&status)?,
        size_sqm: try_get(&row, "size_sqm")?,
        floor: try_get(&row, "floor")?,
        has_private_bathroom: try_get(&row, "has_private_bathroom")?,
        is_furnished: try_get(&row, "is_furnished")?,
        monthly_rent,
        currency,
        utility_share_percent: try_get(&row, "utility_share_percent")?,
        created_at,
        updated_at,
        archived_at,
    }))
}
