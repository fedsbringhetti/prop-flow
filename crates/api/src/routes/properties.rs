use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use propflow_application::use_cases::property::{
    ArchivePropertyCommand, CreatePropertyCommand, ListPropertiesQuery, UpdatePropertyCommand,
};
use propflow_domain::property::{
    EnergyClass, Patch, Property, PropertyChanges, PropertyStatus, PropertyType,
};
use propflow_domain::shared::{Address, CountryCode, PropertyId};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::Authenticated;
use crate::routes::explicit_null;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/properties", post(create).get(list))
        .route(
            "/api/v1/properties/{id}",
            get(detail).patch(update).delete(archive),
        )
}

// ---------------------------------------------------------------------------
// Rappresentazioni di trasporto
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PropertyTypeDto {
    Apartment,
    House,
    Studio,
    Villa,
    Office,
    Retail,
    Garage,
    Land,
    Other,
}

impl From<PropertyTypeDto> for PropertyType {
    fn from(value: PropertyTypeDto) -> Self {
        match value {
            PropertyTypeDto::Apartment => Self::Apartment,
            PropertyTypeDto::House => Self::House,
            PropertyTypeDto::Studio => Self::Studio,
            PropertyTypeDto::Villa => Self::Villa,
            PropertyTypeDto::Office => Self::Office,
            PropertyTypeDto::Retail => Self::Retail,
            PropertyTypeDto::Garage => Self::Garage,
            PropertyTypeDto::Land => Self::Land,
            PropertyTypeDto::Other => Self::Other,
        }
    }
}

impl From<PropertyType> for PropertyTypeDto {
    fn from(value: PropertyType) -> Self {
        match value {
            PropertyType::Apartment => Self::Apartment,
            PropertyType::House => Self::House,
            PropertyType::Studio => Self::Studio,
            PropertyType::Villa => Self::Villa,
            PropertyType::Office => Self::Office,
            PropertyType::Retail => Self::Retail,
            PropertyType::Garage => Self::Garage,
            PropertyType::Land => Self::Land,
            PropertyType::Other => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PropertyStatusDto {
    Draft,
    Available,
    PartiallyRented,
    Rented,
    Maintenance,
    Archived,
}

impl From<PropertyStatusDto> for PropertyStatus {
    fn from(value: PropertyStatusDto) -> Self {
        match value {
            PropertyStatusDto::Draft => Self::Draft,
            PropertyStatusDto::Available => Self::Available,
            PropertyStatusDto::PartiallyRented => Self::PartiallyRented,
            PropertyStatusDto::Rented => Self::Rented,
            PropertyStatusDto::Maintenance => Self::Maintenance,
            PropertyStatusDto::Archived => Self::Archived,
        }
    }
}

impl From<PropertyStatus> for PropertyStatusDto {
    fn from(value: PropertyStatus) -> Self {
        match value {
            PropertyStatus::Draft => Self::Draft,
            PropertyStatus::Available => Self::Available,
            PropertyStatus::PartiallyRented => Self::PartiallyRented,
            PropertyStatus::Rented => Self::Rented,
            PropertyStatus::Maintenance => Self::Maintenance,
            PropertyStatus::Archived => Self::Archived,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
pub enum EnergyClassDto {
    A4,
    A3,
    A2,
    A1,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
}

impl From<EnergyClassDto> for EnergyClass {
    fn from(value: EnergyClassDto) -> Self {
        match value {
            EnergyClassDto::A4 => Self::A4,
            EnergyClassDto::A3 => Self::A3,
            EnergyClassDto::A2 => Self::A2,
            EnergyClassDto::A1 => Self::A1,
            EnergyClassDto::A => Self::A,
            EnergyClassDto::B => Self::B,
            EnergyClassDto::C => Self::C,
            EnergyClassDto::D => Self::D,
            EnergyClassDto::E => Self::E,
            EnergyClassDto::F => Self::F,
            EnergyClassDto::G => Self::G,
        }
    }
}

impl From<EnergyClass> for EnergyClassDto {
    fn from(value: EnergyClass) -> Self {
        match value {
            EnergyClass::A4 => Self::A4,
            EnergyClass::A3 => Self::A3,
            EnergyClass::A2 => Self::A2,
            EnergyClass::A1 => Self::A1,
            EnergyClass::A => Self::A,
            EnergyClass::B => Self::B,
            EnergyClass::C => Self::C,
            EnergyClass::D => Self::D,
            EnergyClass::E => Self::E,
            EnergyClass::F => Self::F,
            EnergyClass::G => Self::G,
        }
    }
}

#[derive(Debug, Clone, Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct AddressDto {
    #[schema(example = "Via Roma 1")]
    pub street: String,
    #[serde(default)]
    pub extra: Option<String>,
    #[schema(example = "Milano")]
    pub city: String,
    #[serde(default)]
    pub province: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
    /// ISO 3166-1 alpha-2. Se omesso, `IT`.
    #[serde(default)]
    pub country: Option<String>,
}

impl AddressDto {
    fn into_domain(self) -> Result<Address, propflow_domain::DomainError> {
        let country = match self.country {
            Some(code) => CountryCode::parse(code)?,
            None => CountryCode::IT,
        };
        Address::new(
            self.street,
            self.extra,
            self.city,
            self.province,
            self.postal_code,
            country,
        )
    }

    fn from_domain(address: &Address) -> Self {
        Self {
            street: address.street().to_owned(),
            extra: address.extra().map(str::to_owned),
            city: address.city().to_owned(),
            province: address.province().map(str::to_owned),
            postal_code: address.postal_code().map(str::to_owned),
            country: Some(address.country().as_str().to_owned()),
        }
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreatePropertyRequest {
    #[schema(example = "Bilocale centro")]
    pub label: String,
    pub property_type: PropertyTypeDto,
    pub address: AddressDto,
}

/// Modifica parziale. Un campo assente resta invariato; un campo presente con
/// valore `null` viene azzerato. È la differenza che rende un PATCH capace di
/// cancellare un valore, e non solo di sovrascriverlo.
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
pub struct UpdatePropertyRequest {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub code: Patch<String>,
    #[serde(default)]
    pub property_type: Option<PropertyTypeDto>,
    #[serde(default)]
    pub status: Option<PropertyStatusDto>,
    #[serde(default)]
    pub is_coliving: Option<bool>,
    #[serde(default)]
    pub address: Option<AddressDto>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<f64>, nullable)]
    pub size_sqm: Patch<Decimal>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<i32>, nullable)]
    pub rooms_count: Patch<i16>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<i32>, nullable)]
    pub bathrooms_count: Patch<i16>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<i32>, nullable)]
    pub floor: Patch<i16>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<bool>, nullable)]
    pub has_elevator: Patch<bool>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub energy_class: Patch<EnergyClassDto>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub notes: Patch<String>,
}

impl UpdatePropertyRequest {
    fn into_changes(self) -> Result<PropertyChanges, propflow_domain::DomainError> {
        let address = self.address.map(AddressDto::into_domain).transpose()?;
        Ok(PropertyChanges {
            label: self.label,
            code: self.code,
            property_type: self.property_type.map(Into::into),
            status: self.status.map(Into::into),
            is_coliving: self.is_coliving,
            address,
            size_sqm: self.size_sqm,
            rooms_count: self.rooms_count,
            bathrooms_count: self.bathrooms_count,
            floor: self.floor,
            has_elevator: self.has_elevator,
            energy_class: self.energy_class.map(|inner| inner.map(Into::into)),
            notes: self.notes,
        })
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PropertyResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub owner_id: Uuid,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub property_type: PropertyTypeDto,
    pub status: PropertyStatusDto,
    pub is_coliving: bool,
    pub address: AddressDto,
    #[schema(value_type = Option<f64>)]
    pub size_sqm: Option<Decimal>,
    pub rooms_count: Option<i16>,
    pub bathrooms_count: Option<i16>,
    pub floor: Option<i16>,
    pub has_elevator: Option<bool>,
    pub energy_class: Option<EnergyClassDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl PropertyResponse {
    fn from_domain(property: &Property) -> Self {
        Self {
            id: property.id().as_uuid(),
            organization_id: property.org_id().as_uuid(),
            owner_id: property.owner_id().as_uuid(),
            label: property.label().to_owned(),
            code: property.code().map(str::to_owned),
            property_type: property.property_type().into(),
            status: property.status().into(),
            is_coliving: property.is_coliving(),
            address: AddressDto::from_domain(property.address()),
            size_sqm: property.size_sqm(),
            rooms_count: property.rooms_count(),
            bathrooms_count: property.bathrooms_count(),
            floor: property.floor(),
            has_elevator: property.has_elevator(),
            energy_class: property.energy_class().map(Into::into),
            notes: property.notes().map(str::to_owned),
            created_at: property.created_at(),
            updated_at: property.updated_at(),
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PropertyPageResponse {
    pub items: Vec<PropertyResponse>,
    /// Da passare come `after` per la pagina successiva. `null` se non ce n'è.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListQueryParams {
    #[serde(default)]
    pub status: Option<PropertyStatusDto>,
    #[serde(default)]
    pub city: Option<String>,
    /// Cursore keyset: ultimo id della pagina precedente.
    #[serde(default)]
    pub after: Option<Uuid>,
    #[serde(default)]
    pub limit: Option<u16>,
}

// ---------------------------------------------------------------------------
// Handler: deserializzare, chiamare lo use case, serializzare. Nulla di più.
// ---------------------------------------------------------------------------

#[utoipa::path(
    post, path = "/api/v1/properties", tag = "immobili",
    request_body = CreatePropertyRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Immobile creato", body = PropertyResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi", body = crate::error::ProblemDetails),
    )
)]
async fn create(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Json(request): Json<CreatePropertyRequest>,
) -> ApiResult<(StatusCode, Json<PropertyResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let address = request.address.into_domain().map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;

    let property = state
        .create_property
        .execute(
            &context,
            CreatePropertyCommand {
                label: request.label,
                property_type: request.property_type.into(),
                address,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok((
        StatusCode::CREATED,
        Json(PropertyResponse::from_domain(&property)),
    ))
}

#[utoipa::path(
    get, path = "/api/v1/properties", tag = "immobili",
    params(ListQueryParams),
    security(("bearer" = [])),
    responses((status = 200, description = "Pagina di immobili", body = PropertyPageResponse))
)]
async fn list(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Query(params): Query<ListQueryParams>,
) -> ApiResult<Json<PropertyPageResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let page = state
        .list_properties
        .execute(
            &context,
            ListPropertiesQuery {
                status: params.status.map(Into::into),
                city: params.city,
                after: params.after.map(PropertyId::from_uuid),
                limit: params.limit,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(PropertyPageResponse {
        items: page
            .items
            .iter()
            .map(PropertyResponse::from_domain)
            .collect(),
        next_cursor: page.next_cursor.map(|id| id.as_uuid()),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/properties/{id}", tag = "immobili",
    params(("id" = Uuid, Path, description = "Identificatore dell'immobile")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Immobile", body = PropertyResponse),
        (status = 404, description = "Non trovato o non visibile", body = crate::error::ProblemDetails),
    )
)]
async fn detail(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<PropertyResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let property = state
        .get_property
        .execute(&context, PropertyId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(Json(PropertyResponse::from_domain(&property)))
}

#[utoipa::path(
    patch, path = "/api/v1/properties/{id}", tag = "immobili",
    params(("id" = Uuid, Path, description = "Identificatore dell'immobile")),
    request_body = UpdatePropertyRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Immobile aggiornato", body = PropertyResponse),
        (status = 409, description = "Immobile archiviato: non modificabile", body = crate::error::ProblemDetails),
    )
)]
async fn update(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdatePropertyRequest>,
) -> ApiResult<Json<PropertyResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let changes = request.into_changes().map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;

    let property = state
        .update_property
        .execute(
            &context,
            UpdatePropertyCommand {
                id: PropertyId::from_uuid(id),
                changes,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(Json(PropertyResponse::from_domain(&property)))
}

#[utoipa::path(
    delete, path = "/api/v1/properties/{id}", tag = "immobili",
    params(("id" = Uuid, Path, description = "Identificatore dell'immobile")),
    security(("bearer" = [])),
    responses(
        (status = 204, description = "Immobile archiviato"),
        (status = 404, description = "Non trovato o non visibile", body = crate::error::ProblemDetails),
    )
)]
async fn archive(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let request_id = context.request_id().map(str::to_owned);
    state
        .archive_property
        .execute(
            &context,
            ArchivePropertyCommand {
                id: PropertyId::from_uuid(id),
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(StatusCode::NO_CONTENT)
}
