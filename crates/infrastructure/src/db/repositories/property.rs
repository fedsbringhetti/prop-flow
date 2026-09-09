use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::ports::{
    PropertyOccupancy, PropertyPage, PropertyQuery, PropertyRepository, RepositoryError,
    RepositoryResult,
};
use propflow_domain::property::{Property, PropertyParts, PropertyResident};
use propflow_domain::shared::{Address, CountryCode, Currency, Money, OrgId, PropertyId, UserId};

use super::user::try_get;
use crate::db::db_enums;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

macro_rules! select_property {
    () => {
        "
    SELECT id,
           org_id,
           owner_id,
           code,
           label,
           property_type::text  AS property_type,
           status::text         AS status,
           is_coliving,
           address_street,
           address_extra,
           city,
           province,
           postal_code,
           country::text        AS country,
           size_sqm::numeric    AS size_sqm,
           rooms_count,
           bathrooms_count,
           floor,
           has_elevator,
           energy_class::text   AS energy_class,
           purchase_price::numeric AS purchase_price,
           current_value::numeric  AS current_value,
           currency::text       AS currency,
           notes,
           created_at,
           updated_at,
           deleted_at
      FROM app.properties
"
    };
}

#[async_trait]
impl PropertyRepository for PgUnitOfWork {
    async fn find_by_id(&mut self, id: PropertyId) -> RepositoryResult<Option<Property>> {
        let row = sqlx::query(concat!(
            select_property!(),
            " WHERE id = $1 AND deleted_at IS NULL"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(map_property).transpose()
    }

    async fn list(&mut self, query: &PropertyQuery) -> RepositoryResult<PropertyPage> {
        // I filtri sono opzionali senza costruire SQL dinamico: un solo testo di
        // query significa un solo piano da analizzare e nessun rischio di
        // concatenazione. Si legge una riga in più del limite per sapere se esiste
        // una pagina successiva, invece di fare un COUNT separato.
        let fetch_limit = i64::from(query.limit) + 1;
        let rows = sqlx::query(concat!(
            select_property!(),
            " WHERE deleted_at IS NULL
                AND ($1::text IS NULL OR status = $1::app.property_status)
                AND ($2::text IS NULL OR city = $2)
                AND ($3::uuid IS NULL OR id > $3)
                -- `occupied` è il filtro che sostituisce `?status=rented`, che non
                -- può funzionare: quel valore non è memorizzato. La funzione
                -- risponde `NULL` quando non c'è niente da derivare, e «niente da
                -- derivare» è esattamente «non occupato».
                AND ($5::boolean IS NULL
                     OR (app.property_occupancy_status(id) IS NOT NULL) = $5)
              ORDER BY id
              LIMIT $4"
        ))
        .bind(query.status.map(db_enums::property_status_to_db))
        .bind(query.city.as_deref())
        .bind(query.after.map(|id| id.as_uuid()))
        .bind(fetch_limit)
        .bind(query.occupied)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        let has_more = rows.len() > usize::from(query.limit);
        let items = rows
            .into_iter()
            .take(usize::from(query.limit))
            .map(map_property)
            .collect::<RepositoryResult<Vec<Property>>>()?;
        let next_cursor = if has_more {
            items.last().map(Property::id)
        } else {
            None
        };

        Ok(PropertyPage { items, next_cursor })
    }

    async fn insert(&mut self, property: &Property) -> RepositoryResult<()> {
        sqlx::query(
            "INSERT INTO app.properties (
                 id, org_id, owner_id, code, label, property_type, status, is_coliving,
                 address_street, address_extra, city, province, postal_code, country,
                 size_sqm, rooms_count, bathrooms_count, floor, has_elevator, energy_class,
                 purchase_price, current_value, currency, notes, created_at, updated_at
             ) VALUES (
                 $1, $2, $3, $4, $5, $6::app.property_type, $7::app.property_status, $8,
                 $9, $10, $11, $12, $13, $14::app.country_code,
                 $15::app.money_amount, $16, $17, $18, $19, $20::app.energy_class,
                 $21::app.money_amount, $22::app.money_amount, $23::app.currency_code, $24, $25, $26
             )",
        )
        .bind(property.id().as_uuid())
        .bind(property.org_id().as_uuid())
        .bind(property.owner_id().as_uuid())
        .bind(property.code())
        .bind(property.label())
        .bind(db_enums::property_type_to_db(property.property_type()))
        .bind(db_enums::property_status_to_db(property.status()))
        .bind(property.is_coliving())
        .bind(property.address().street())
        .bind(property.address().extra())
        .bind(property.address().city())
        .bind(property.address().province())
        .bind(property.address().postal_code())
        .bind(property.address().country().as_str())
        .bind(property.size_sqm())
        .bind(property.rooms_count())
        .bind(property.bathrooms_count())
        .bind(property.floor())
        .bind(property.has_elevator())
        .bind(property.energy_class().map(db_enums::energy_class_to_db))
        .bind(property.purchase_price().map(|money| money.amount()))
        .bind(property.current_value().map(|money| money.amount()))
        .bind(property.currency().as_str())
        .bind(property.notes())
        .bind(property.created_at())
        .bind(property.updated_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn update(&mut self, property: &Property) -> RepositoryResult<()> {
        let result = sqlx::query(
            "UPDATE app.properties
                SET code            = $2,
                    label           = $3,
                    property_type   = $4::app.property_type,
                    status          = $5::app.property_status,
                    is_coliving     = $6,
                    address_street  = $7,
                    address_extra   = $8,
                    city            = $9,
                    province        = $10,
                    postal_code     = $11,
                    country         = $12::app.country_code,
                    size_sqm        = $13::app.money_amount,
                    rooms_count     = $14,
                    bathrooms_count = $15,
                    floor           = $16,
                    has_elevator    = $17,
                    energy_class    = $18::app.energy_class,
                    notes           = $19
              WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(property.id().as_uuid())
        .bind(property.code())
        .bind(property.label())
        .bind(db_enums::property_type_to_db(property.property_type()))
        .bind(db_enums::property_status_to_db(property.status()))
        .bind(property.is_coliving())
        .bind(property.address().street())
        .bind(property.address().extra())
        .bind(property.address().city())
        .bind(property.address().province())
        .bind(property.address().postal_code())
        .bind(property.address().country().as_str())
        .bind(property.size_sqm())
        .bind(property.rooms_count())
        .bind(property.bathrooms_count())
        .bind(property.floor())
        .bind(property.has_elevator())
        .bind(property.energy_class().map(db_enums::energy_class_to_db))
        .bind(property.notes())
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

    async fn soft_delete(&mut self, property: &Property) -> RepositoryResult<()> {
        let result = sqlx::query(
            "UPDATE app.properties
                SET status     = 'archived'::app.property_status,
                    deleted_at = $2
              WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(property.id().as_uuid())
        .bind(property.archived_at().unwrap_or_else(Utc::now))
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }

    async fn occupancy(&mut self, ids: &[PropertyId]) -> RepositoryResult<Vec<PropertyOccupancy>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        // `app.property_occupancy_status` restituisce `NULL` quando non c'è niente
        // da derivare, e quel `NULL` è informazione: significa «lo stato memorizzato
        // è ancora l'ultima parola», non «disponibile».
        let uuids: Vec<Uuid> = ids.iter().map(|id| id.as_uuid()).collect();
        let rows = sqlx::query(
            "SELECT id,
                    app.property_occupancy_status(id)::text AS occupancy_status
               FROM app.properties
              WHERE id = ANY($1) AND deleted_at IS NULL",
        )
        .bind(&uuids)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        rows.into_iter()
            .map(|row| {
                let id: Uuid = try_get(&row, "id")?;
                let status: Option<String> = try_get(&row, "occupancy_status")?;
                let status = status
                    .map(|value| db_enums::property_status_from_db(&value))
                    .transpose()?;
                Ok(PropertyOccupancy {
                    property_id: PropertyId::from_uuid(id),
                    status,
                })
            })
            .collect()
    }

    async fn list_residents(
        &mut self,
        property_id: PropertyId,
    ) -> RepositoryResult<Vec<PropertyResident>> {
        // **Una `SELECT` dalla funzione, e non i join che sembrerebbero ovvi.**
        // `app.lease_parties` unita ad `app.leases` — e unirle serve, perché
        // l'immobile e «in essere» stanno sul contratto — passa da
        // `leases_select`, che è `can_access_lease(id)`: l'elenco si riduce al
        // solo chiamante e non c'è alcun errore a dirlo. Vedi la migration `0030`.
        //
        // Nessun `deleted_at IS NULL` qui, ed è l'unica query del progetto a non
        // averlo: i tre filtri di soft delete — immobile, contratto, parte — sono
        // dentro la funzione, insieme al confine. Ripeterli fuori li duplicherebbe
        // senza aggiungere nulla, perché da qui le righe scartate non si vedono.
        //
        // L'ordinamento è qui e non nella funzione: è una scelta di presentazione,
        // e il confine nella funzione è una scelta di sicurezza.
        let rows = sqlx::query(
            "SELECT user_id, full_name, room_name
               FROM app.property_residents($1)
              ORDER BY full_name, room_name NULLS FIRST",
        )
        .bind(property_id.as_uuid())
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        rows.into_iter().map(map_resident).collect()
    }
}

fn map_resident(row: PgRow) -> RepositoryResult<PropertyResident> {
    let user_id: Option<Uuid> = try_get(&row, "user_id")?;
    Ok(PropertyResident {
        // Nullo finché l'invitato non accetta: l'elenco lo mostra invece di
        // ometterlo, altrimenti chi guarda non capirebbe perché una persona che
        // sa di avere in casa non si può aggiungere a un gruppo.
        user_id: user_id.map(UserId::from_uuid),
        full_name: try_get(&row, "full_name")?,
        room_name: try_get(&row, "room_name")?,
    })
}

#[allow(clippy::too_many_lines)]
fn map_property(row: PgRow) -> RepositoryResult<Property> {
    let invalid = |field: &str, error: propflow_domain::DomainError| RepositoryError::Backend {
        details: format!("{field} non valido in database: {error}"),
    };

    let id: Uuid = try_get(&row, "id")?;
    let org_id: Uuid = try_get(&row, "org_id")?;
    let owner_id: Uuid = try_get(&row, "owner_id")?;
    let property_type: String = try_get(&row, "property_type")?;
    let status: String = try_get(&row, "status")?;
    let country: String = try_get(&row, "country")?;
    let currency_code: String = try_get(&row, "currency")?;
    let energy_class: Option<String> = try_get(&row, "energy_class")?;
    let purchase_price: Option<Decimal> = try_get(&row, "purchase_price")?;
    let current_value: Option<Decimal> = try_get(&row, "current_value")?;
    let created_at: DateTime<Utc> = try_get(&row, "created_at")?;
    let updated_at: DateTime<Utc> = try_get(&row, "updated_at")?;
    let archived_at: Option<DateTime<Utc>> = try_get(&row, "deleted_at")?;

    let currency = Currency::parse(&currency_code).map_err(|error| invalid("currency", error))?;
    let country = CountryCode::parse(&country).map_err(|error| invalid("country", error))?;

    let address = Address::new(
        try_get::<String>(&row, "address_street")?,
        try_get(&row, "address_extra")?,
        try_get::<String>(&row, "city")?,
        try_get(&row, "province")?,
        try_get(&row, "postal_code")?,
        country,
    )
    .map_err(|error| invalid("address", error))?;

    let money = |amount: Option<Decimal>, field: &'static str| -> RepositoryResult<Option<Money>> {
        amount
            .map(|value| Money::new(value, currency))
            .transpose()
            .map_err(|error| invalid(field, error))
    };

    let energy_class = energy_class
        .as_deref()
        .map(db_enums::energy_class_from_db)
        .transpose()?;

    Ok(Property::from_persistence(PropertyParts {
        id: PropertyId::from_uuid(id),
        org_id: OrgId::from_uuid(org_id),
        owner_id: UserId::from_uuid(owner_id),
        code: try_get(&row, "code")?,
        label: try_get(&row, "label")?,
        property_type: db_enums::property_type_from_db(&property_type)?,
        status: db_enums::property_status_from_db(&status)?,
        is_coliving: try_get(&row, "is_coliving")?,
        address,
        size_sqm: try_get(&row, "size_sqm")?,
        rooms_count: try_get(&row, "rooms_count")?,
        bathrooms_count: try_get(&row, "bathrooms_count")?,
        floor: try_get(&row, "floor")?,
        has_elevator: try_get(&row, "has_elevator")?,
        energy_class,
        purchase_price: money(purchase_price, "purchase_price")?,
        current_value: money(current_value, "current_value")?,
        currency,
        notes: try_get(&row, "notes")?,
        created_at,
        updated_at,
        archived_at,
    }))
}
