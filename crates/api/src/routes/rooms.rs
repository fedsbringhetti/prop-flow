use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, patch};
use axum::{Json, Router};
use propflow_application::use_cases::room::{
    ArchiveRoomCommand, CreateRoomCommand, UpdateRoomCommand,
};
use propflow_domain::DomainError;
use propflow_domain::property::{Patch, Room, RoomChanges, RoomStatus};
use propflow_domain::shared::{PropertyId, RoomId};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::Authenticated;
use crate::routes::explicit_null;
use crate::state::AppState;

/// Le stanze si creano ed elencano *dentro* l'immobile, perché non hanno vita
/// propria; una volta identificate, si modificano per id, perché il percorso
/// dell'immobile non aggiunge nulla e obbligherebbe il client a portarselo dietro.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/properties/{property_id}/rooms",
            get(list).post(create),
        )
        .route("/api/v1/rooms/{id}", patch(update).delete(archive))
}

// ---------------------------------------------------------------------------
// Rappresentazioni di trasporto
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoomStatusDto {
    Available,
    Occupied,
    Unavailable,
}

impl From<RoomStatusDto> for RoomStatus {
    fn from(value: RoomStatusDto) -> Self {
        match value {
            RoomStatusDto::Available => Self::Available,
            RoomStatusDto::Occupied => Self::Occupied,
            RoomStatusDto::Unavailable => Self::Unavailable,
        }
    }
}

impl From<RoomStatus> for RoomStatusDto {
    fn from(value: RoomStatus) -> Self {
        match value {
            RoomStatus::Available => Self::Available,
            RoomStatus::Occupied => Self::Occupied,
            RoomStatus::Unavailable => Self::Unavailable,
        }
    }
}

#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
pub struct CreateRoomRequest {
    #[schema(example = "Camera matrimoniale")]
    pub name: String,
    #[serde(default)]
    pub status: Option<RoomStatusDto>,
    #[serde(default)]
    #[schema(value_type = Option<f64>)]
    pub size_sqm: Option<Decimal>,
    #[serde(default)]
    pub floor: Option<i16>,
    #[serde(default)]
    pub has_private_bathroom: Option<bool>,
    #[serde(default)]
    pub is_furnished: Option<bool>,
    /// Canone della stanza come stringa decimale (`"450.00"`).
    #[serde(default)]
    pub monthly_rent: Option<String>,
    /// Quota di ripartizione delle utenze, 0–100. Assente significa "in parti uguali".
    #[serde(default)]
    #[schema(value_type = Option<f64>)]
    pub utility_share_percent: Option<Decimal>,
}

impl CreateRoomRequest {
    /// Gli attributi opzionali viaggiano come `RoomChanges`: creazione e
    /// aggiornamento condividono le invarianti dell'aggregato, e due percorsi
    /// distinti le farebbero divergere alla prima modifica.
    fn into_attributes(self) -> Result<(String, RoomChanges), DomainError> {
        let monthly_rent = parse_amount(self.monthly_rent)?;
        Ok((
            self.name,
            RoomChanges {
                name: None,
                status: self.status.map(Into::into),
                size_sqm: self.size_sqm.map(Some),
                floor: self.floor.map(Some),
                has_private_bathroom: self.has_private_bathroom,
                is_furnished: self.is_furnished.map(Some),
                monthly_rent: monthly_rent.map(Some),
                utility_share_percent: self.utility_share_percent.map(Some),
            },
        ))
    }
}

/// Modifica parziale. Un campo assente resta invariato; un campo presente con
/// valore `null` viene azzerato.
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
pub struct UpdateRoomRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub status: Option<RoomStatusDto>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<f64>, nullable)]
    pub size_sqm: Patch<Decimal>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<i32>, nullable)]
    pub floor: Patch<i16>,
    #[serde(default)]
    pub has_private_bathroom: Option<bool>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<bool>, nullable)]
    pub is_furnished: Patch<bool>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub monthly_rent: Patch<String>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<f64>, nullable)]
    pub utility_share_percent: Patch<Decimal>,
}

impl UpdateRoomRequest {
    fn into_changes(self) -> Result<RoomChanges, DomainError> {
        let monthly_rent = match self.monthly_rent {
            None => None,
            Some(raw) => Some(parse_amount(raw)?),
        };
        Ok(RoomChanges {
            name: self.name,
            status: self.status.map(Into::into),
            size_sqm: self.size_sqm,
            floor: self.floor,
            has_private_bathroom: self.has_private_bathroom,
            is_furnished: self.is_furnished,
            monthly_rent,
            utility_share_percent: self.utility_share_percent,
        })
    }
}

fn parse_amount(raw: Option<String>) -> Result<Option<Decimal>, DomainError> {
    raw.map(|value| crate::routes::amount("monthly_rent", &value))
        .transpose()
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct RoomResponse {
    pub id: Uuid,
    pub property_id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub status: RoomStatusDto,
    #[schema(value_type = Option<f64>)]
    pub size_sqm: Option<Decimal>,
    pub floor: Option<i16>,
    pub has_private_bathroom: bool,
    pub is_furnished: Option<bool>,
    /// Importo come stringa decimale, oppure assente se la stanza non ha un
    /// canone proprio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monthly_rent: Option<String>,
    pub currency: String,
    #[schema(value_type = Option<f64>)]
    pub utility_share_percent: Option<Decimal>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl RoomResponse {
    fn from_domain(room: &Room) -> Self {
        Self {
            id: room.id().as_uuid(),
            property_id: room.property_id().as_uuid(),
            organization_id: room.org_id().as_uuid(),
            name: room.name().to_owned(),
            status: room.status().into(),
            size_sqm: room.size_sqm(),
            floor: room.floor(),
            has_private_bathroom: room.has_private_bathroom(),
            is_furnished: room.is_furnished(),
            monthly_rent: room
                .monthly_rent()
                .map(|money| crate::routes::format_amount(money.amount())),
            currency: room.currency().as_str().to_owned(),
            utility_share_percent: room.utility_share_percent(),
            created_at: room.created_at(),
            updated_at: room.updated_at(),
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct RoomListResponse {
    pub items: Vec<RoomResponse>,
}

// ---------------------------------------------------------------------------
// Handler: deserializzare, chiamare lo use case, serializzare. Nulla di più.
// ---------------------------------------------------------------------------

#[utoipa::path(
    post, path = "/api/v1/properties/{property_id}/rooms", tag = "stanze",
    params(("property_id" = Uuid, Path, description = "Identificatore dell'immobile")),
    request_body = CreateRoomRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Stanza creata", body = RoomResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Immobile non trovato o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Nome già usato nell'immobile", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi", body = crate::error::ProblemDetails),
    )
)]
async fn create(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(property_id): Path<Uuid>,
    Json(request): Json<CreateRoomRequest>,
) -> ApiResult<(StatusCode, Json<RoomResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let (name, attributes) = request.into_attributes().map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;

    let room = state
        .create_room
        .execute(
            &context,
            CreateRoomCommand {
                property_id: PropertyId::from_uuid(property_id),
                name,
                attributes,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok((StatusCode::CREATED, Json(RoomResponse::from_domain(&room))))
}

#[utoipa::path(
    get, path = "/api/v1/properties/{property_id}/rooms", tag = "stanze",
    params(("property_id" = Uuid, Path, description = "Identificatore dell'immobile")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Stanze dell'immobile", body = RoomListResponse),
        (status = 404, description = "Immobile non trovato o non visibile", body = crate::error::ProblemDetails),
    )
)]
async fn list(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(property_id): Path<Uuid>,
) -> ApiResult<Json<RoomListResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let rooms = state
        .list_rooms
        .execute(&context, PropertyId::from_uuid(property_id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(RoomListResponse {
        items: rooms.iter().map(RoomResponse::from_domain).collect(),
    }))
}

#[utoipa::path(
    patch, path = "/api/v1/rooms/{id}", tag = "stanze",
    params(("id" = Uuid, Path, description = "Identificatore della stanza")),
    request_body = UpdateRoomRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Stanza aggiornata", body = RoomResponse),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Stanza archiviata, o nome già usato", body = crate::error::ProblemDetails),
    )
)]
async fn update(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateRoomRequest>,
) -> ApiResult<Json<RoomResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let changes = request.into_changes().map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;

    let room = state
        .update_room
        .execute(
            &context,
            UpdateRoomCommand {
                id: RoomId::from_uuid(id),
                changes,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(Json(RoomResponse::from_domain(&room)))
}

#[utoipa::path(
    delete, path = "/api/v1/rooms/{id}", tag = "stanze",
    params(("id" = Uuid, Path, description = "Identificatore della stanza")),
    security(("bearer" = [])),
    responses(
        (status = 204, description = "Stanza archiviata"),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
    )
)]
async fn archive(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let request_id = context.request_id().map(str::to_owned);
    state
        .archive_room
        .execute(
            &context,
            ArchiveRoomCommand {
                id: RoomId::from_uuid(id),
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(StatusCode::NO_CONTENT)
}
