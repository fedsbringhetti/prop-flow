use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use propflow_application::use_cases::document::{
    ArchiveDocumentCommand, ListDocumentsCommand, UploadDocumentCommand,
};
use propflow_domain::DomainError;
use propflow_domain::document::{AttachmentOwner, Document, DocumentKind};
use propflow_domain::shared::{DocumentId, LeaseId, PropertyId, RoomId, TicketId};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::Authenticated;
use crate::state::AppState;

/// Rotte che **non** caricano nulla: valgono il limite di corpo generale.
///
/// Elenco e caricamento sono annidati sotto l'oggetto a cui il documento è
/// allegato, perché un documento non ha vita propria; download e archiviazione
/// vanno per id, perché il percorso dell'owner non aggiungerebbe nulla e
/// obbligherebbe il client a portarselo dietro. È la stessa scelta delle stanze.
///
/// Le rotte sulle segnalazioni sono arrivate con il modulo dei ticket **senza
/// toccare gli use case dei documenti**: `app.attachment_owner_type` aveva già
/// `ticket`, e `can_access_attachment` lo risolveva già su `can_access_ticket`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/properties/{property_id}/documents",
            get(list_for_property),
        )
        .route("/api/v1/rooms/{room_id}/documents", get(list_for_room))
        .route("/api/v1/leases/{lease_id}/documents", get(list_for_lease))
        .route(
            "/api/v1/tickets/{ticket_id}/documents",
            get(list_for_ticket),
        )
        .route("/api/v1/documents/{id}/content", get(content))
        .route("/api/v1/documents/{id}", delete(archive))
}

/// Rotte di caricamento, montate a parte.
///
/// **Non è una preferenza di stile.** `RequestBodyLimitLayer` è montato sull'intero
/// router in `crate::router`: un limite più largo dichiarato *sopra* questa rotta
/// non verrebbe mai raggiunto, perché il layer esterno rifiuta per primo, e il
/// sintomo sarebbe un `413` che nessun limite per rotta sembra spiegare. La via
/// corretta è montare il limite generale sul gruppo che **non** comprende queste
/// rotte, e dare a queste il proprio.
///
/// Il limite è **doppio** perché i limiti sono due, e il secondo non è quello del
/// piano: `RequestBodyLimitLayer` (tower-http) ferma i byte in transito, ma
/// `Multipart` di axum applica anche `DefaultBodyLimit`, che vale 2 MiB. Alzando
/// solo il primo, un file da 5 MiB fallisce lo stesso — e l'errore arriva dal
/// parser multipart, non dal layer, quindi cercarlo fra i layer non lo trova.
pub fn upload_router(limit_bytes: usize) -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/properties/{property_id}/documents",
            post(upload_to_property),
        )
        .route("/api/v1/rooms/{room_id}/documents", post(upload_to_room))
        .route("/api/v1/leases/{lease_id}/documents", post(upload_to_lease))
        .route(
            "/api/v1/tickets/{ticket_id}/documents",
            post(upload_to_ticket),
        )
        .layer(DefaultBodyLimit::max(limit_bytes))
        .layer(tower_http::limit::RequestBodyLimitLayer::new(limit_bytes))
}

// ---------------------------------------------------------------------------
// Rappresentazioni di trasporto
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DocumentKindDto {
    LeaseContract,
    IdDocument,
    UtilityBill,
    Invoice,
    Receipt,
    Photo,
    InspectionReport,
    EnergyCertificate,
    CadastralExtract,
    InsurancePolicy,
    Other,
}

impl DocumentKindDto {
    fn parse(raw: &str) -> Result<Self, DomainError> {
        Ok(match raw {
            "lease_contract" => Self::LeaseContract,
            "id_document" => Self::IdDocument,
            "utility_bill" => Self::UtilityBill,
            "invoice" => Self::Invoice,
            "receipt" => Self::Receipt,
            "photo" => Self::Photo,
            "inspection_report" => Self::InspectionReport,
            "energy_certificate" => Self::EnergyCertificate,
            "cadastral_extract" => Self::CadastralExtract,
            "insurance_policy" => Self::InsurancePolicy,
            "other" => Self::Other,
            _ => {
                return Err(DomainError::validation(
                    "kind",
                    "is not a supported document kind",
                ));
            }
        })
    }
}

impl From<DocumentKindDto> for DocumentKind {
    fn from(value: DocumentKindDto) -> Self {
        match value {
            DocumentKindDto::LeaseContract => Self::LeaseContract,
            DocumentKindDto::IdDocument => Self::IdDocument,
            DocumentKindDto::UtilityBill => Self::UtilityBill,
            DocumentKindDto::Invoice => Self::Invoice,
            DocumentKindDto::Receipt => Self::Receipt,
            DocumentKindDto::Photo => Self::Photo,
            DocumentKindDto::InspectionReport => Self::InspectionReport,
            DocumentKindDto::EnergyCertificate => Self::EnergyCertificate,
            DocumentKindDto::CadastralExtract => Self::CadastralExtract,
            DocumentKindDto::InsurancePolicy => Self::InsurancePolicy,
            DocumentKindDto::Other => Self::Other,
        }
    }
}

impl From<DocumentKind> for DocumentKindDto {
    fn from(value: DocumentKind) -> Self {
        match value {
            DocumentKind::LeaseContract => Self::LeaseContract,
            DocumentKind::IdDocument => Self::IdDocument,
            DocumentKind::UtilityBill => Self::UtilityBill,
            DocumentKind::Invoice => Self::Invoice,
            DocumentKind::Receipt => Self::Receipt,
            DocumentKind::Photo => Self::Photo,
            DocumentKind::InspectionReport => Self::InspectionReport,
            DocumentKind::EnergyCertificate => Self::EnergyCertificate,
            DocumentKind::CadastralExtract => Self::CadastralExtract,
            DocumentKind::InsurancePolicy => Self::InsurancePolicy,
            DocumentKind::Other => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentOwnerDto {
    Property,
    Room,
    Lease,
    Ticket,
}

/// Corpo `multipart/form-data` del caricamento.
///
/// Dichiarato solo per OpenAPI: il corpo lo legge l'estrattore `Multipart`, che
/// non deserializza in una struct. Tenerlo comunque significa che la specifica
/// descrive i campi invece di dire "binario, arrangiati".
#[derive(Debug, utoipa::ToSchema)]
pub struct UploadDocumentForm {
    /// Il file. Nome e tipo MIME si leggono dalle intestazioni della parte.
    #[schema(value_type = String, format = Binary)]
    pub file: Vec<u8>,
    /// Classificazione. Assente vale `other`.
    pub kind: Option<DocumentKindDto>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct DocumentResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub kind: DocumentKindDto,
    pub owner_type: AttachmentOwnerDto,
    pub owner_id: Uuid,
    pub original_filename: String,
    pub mime_type: String,
    pub size_bytes: u64,
    /// Digest SHA-256 esadecimale, calcolato dall'API sui byte ricevuti.
    pub checksum_sha256: String,
    /// Cifratura a riposo **confermata dal fornitore di storage**, non richiesta:
    /// su un bucket non configurato vale `false`, ed è la verità.
    pub is_encrypted: bool,
    pub uploaded_by: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl DocumentResponse {
    fn from_domain(document: &Document) -> Self {
        let owner = document.owner();
        Self {
            id: document.id().as_uuid(),
            organization_id: document.org_id().as_uuid(),
            kind: document.kind().into(),
            owner_type: match owner {
                AttachmentOwner::Property(_) => AttachmentOwnerDto::Property,
                AttachmentOwner::Room(_) => AttachmentOwnerDto::Room,
                AttachmentOwner::Lease(_) => AttachmentOwnerDto::Lease,
                AttachmentOwner::Ticket(_) => AttachmentOwnerDto::Ticket,
            },
            owner_id: owner.owner_id(),
            original_filename: document.original_filename().as_str().to_owned(),
            mime_type: document.mime_type().as_str().to_owned(),
            size_bytes: document.size_bytes().get(),
            checksum_sha256: document.checksum().to_hex(),
            is_encrypted: document.is_encrypted(),
            uploaded_by: document.uploaded_by().map(|id| id.as_uuid()),
            created_at: document.created_at(),
            updated_at: document.updated_at(),
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct DocumentPageResponse {
    pub items: Vec<DocumentResponse>,
    /// Da passare come `after` per la pagina successiva. `null` se non ce n'è.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListQueryParams {
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
    post, path = "/api/v1/properties/{property_id}/documents", tag = "documenti",
    params(("property_id" = Uuid, Path, description = "Identificatore dell'immobile")),
    request_body(content = UploadDocumentForm, content_type = "multipart/form-data"),
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Documento caricato", body = DocumentResponse),
        (status = 404, description = "Immobile non trovato o non visibile", body = crate::error::ProblemDetails),
        (status = 413, description = "Corpo oltre il limite di caricamento", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi", body = crate::error::ProblemDetails),
    )
)]
async fn upload_to_property(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(property_id): Path<Uuid>,
    multipart: Multipart,
) -> ApiResult<(StatusCode, Json<DocumentResponse>)> {
    upload(
        state,
        context,
        AttachmentOwner::Property(PropertyId::from_uuid(property_id)),
        multipart,
    )
    .await
}

#[utoipa::path(
    post, path = "/api/v1/rooms/{room_id}/documents", tag = "documenti",
    params(("room_id" = Uuid, Path, description = "Identificatore della stanza")),
    request_body(content = UploadDocumentForm, content_type = "multipart/form-data"),
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Documento caricato", body = DocumentResponse),
        (status = 404, description = "Stanza non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 413, description = "Corpo oltre il limite di caricamento", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi", body = crate::error::ProblemDetails),
    )
)]
async fn upload_to_room(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(room_id): Path<Uuid>,
    multipart: Multipart,
) -> ApiResult<(StatusCode, Json<DocumentResponse>)> {
    upload(
        state,
        context,
        AttachmentOwner::Room(RoomId::from_uuid(room_id)),
        multipart,
    )
    .await
}

#[utoipa::path(
    post, path = "/api/v1/leases/{lease_id}/documents", tag = "documenti",
    params(("lease_id" = Uuid, Path, description = "Identificatore del contratto")),
    request_body(content = UploadDocumentForm, content_type = "multipart/form-data"),
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Documento caricato", body = DocumentResponse),
        (status = 404, description = "Contratto non trovato o non visibile", body = crate::error::ProblemDetails),
        (status = 413, description = "Corpo oltre il limite di caricamento", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi", body = crate::error::ProblemDetails),
    )
)]
async fn upload_to_lease(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(lease_id): Path<Uuid>,
    multipart: Multipart,
) -> ApiResult<(StatusCode, Json<DocumentResponse>)> {
    upload(
        state,
        context,
        AttachmentOwner::Lease(LeaseId::from_uuid(lease_id)),
        multipart,
    )
    .await
}

#[utoipa::path(
    post, path = "/api/v1/tickets/{ticket_id}/documents", tag = "documenti",
    params(("ticket_id" = Uuid, Path, description = "Identificatore della segnalazione")),
    request_body(content = UploadDocumentForm, content_type = "multipart/form-data"),
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Documento caricato", body = DocumentResponse),
        (status = 404, description = "Segnalazione non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 413, description = "Corpo oltre il limite di caricamento", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi", body = crate::error::ProblemDetails),
    )
)]
async fn upload_to_ticket(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(ticket_id): Path<Uuid>,
    multipart: Multipart,
) -> ApiResult<(StatusCode, Json<DocumentResponse>)> {
    upload(
        state,
        context,
        AttachmentOwner::Ticket(TicketId::from_uuid(ticket_id)),
        multipart,
    )
    .await
}

async fn upload(
    state: AppState,
    context: propflow_domain::RequestContext,
    owner: AttachmentOwner,
    multipart: Multipart,
) -> ApiResult<(StatusCode, Json<DocumentResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let form = read_form(multipart)
        .await
        .map_err(|error| error.with_request_id(request_id.clone()))?;

    let document = state
        .upload_document
        .execute(
            &context,
            UploadDocumentCommand {
                owner,
                kind: form.kind.into(),
                original_filename: form.original_filename,
                mime_type: form.mime_type,
                content: form.content,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok((
        StatusCode::CREATED,
        Json(DocumentResponse::from_domain(&document)),
    ))
}

struct UploadForm {
    kind: DocumentKindDto,
    original_filename: String,
    mime_type: String,
    content: Vec<u8>,
}

/// Legge il corpo multipart.
///
/// Nome e tipo MIME si prendono dalle **intestazioni della parte** e non da campi
/// separati del form: sono ciò che il browser manda da sé, e chiederli due volte
/// vorrebbe dire poterli ricevere in disaccordo.
async fn read_form(mut multipart: Multipart) -> Result<UploadForm, ApiError> {
    let invalid = |field: &'static str, message: &'static str| {
        ApiError::from(propflow_application::AppError::Validation(
            DomainError::validation(field, message),
        ))
    };

    let mut kind = DocumentKindDto::Other;
    let mut file: Option<(String, String, Vec<u8>)> = None;

    loop {
        // Un corpo oltre il limite si manifesta **qui**, come errore del parser e
        // non come rifiuto di un layer: `Multipart` legge in streaming, quindi il
        // superamento si scopre leggendo. Diventa un `413`, che è la risposta che
        // il chiamante si aspetta.
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(error) => return Err(multipart_error(&error)),
        };

        match field.name().map(str::to_owned).unwrap_or_default().as_str() {
            "kind" => {
                let raw = field
                    .text()
                    .await
                    .map_err(|error| multipart_error(&error))?;
                kind = DocumentKindDto::parse(raw.trim()).map_err(|error| {
                    ApiError::from(propflow_application::AppError::Validation(error))
                })?;
            }
            "file" => {
                let filename = field
                    .file_name()
                    .map(str::to_owned)
                    .ok_or_else(|| invalid("file", "must carry a filename"))?;
                let mime = field
                    .content_type()
                    .map(str::to_owned)
                    // Nessun `content-type` sulla parte: `application/octet-stream`
                    // è ciò che dice RFC 7578, e mentire su un default più
                    // plausibile renderebbe il MIME memorizzato un'invenzione.
                    .unwrap_or_else(|| "application/octet-stream".to_owned());
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|error| multipart_error(&error))?;
                file = Some((filename, mime, bytes.to_vec()));
            }
            // Un campo sconosciuto si consuma e si ignora: rifiutarlo romperebbe
            // ogni client che aggiunge un campo suo, e accettarlo non costa nulla.
            _ => {
                let _ = field.bytes().await;
            }
        }
    }

    let (original_filename, mime_type, content) =
        file.ok_or_else(|| invalid("file", "is required"))?;

    Ok(UploadForm {
        kind,
        original_filename,
        mime_type,
        content,
    })
}

/// Traduce un errore del parser multipart.
///
/// Il corpo troppo grande esce come `413` e non come `422`: è la stessa condizione
/// che `RequestBodyLimitLayer` produrrebbe, e un client non deve vedere due codici
/// diversi a seconda di quale dei due limiti ha incontrato per primo.
fn multipart_error(error: &axum::extract::multipart::MultipartError) -> ApiError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return ApiError::payload_too_large();
    }
    ApiError::from(propflow_application::AppError::Validation(
        DomainError::validation("file", "is not a valid multipart body"),
    ))
}

#[utoipa::path(
    get, path = "/api/v1/properties/{property_id}/documents", tag = "documenti",
    params(("property_id" = Uuid, Path, description = "Identificatore dell'immobile"), ListQueryParams),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Pagina di documenti", body = DocumentPageResponse),
        (status = 404, description = "Immobile non trovato o non visibile", body = crate::error::ProblemDetails),
    )
)]
async fn list_for_property(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(property_id): Path<Uuid>,
    Query(params): Query<ListQueryParams>,
) -> ApiResult<Json<DocumentPageResponse>> {
    list(
        state,
        context,
        AttachmentOwner::Property(PropertyId::from_uuid(property_id)),
        params,
    )
    .await
}

#[utoipa::path(
    get, path = "/api/v1/rooms/{room_id}/documents", tag = "documenti",
    params(("room_id" = Uuid, Path, description = "Identificatore della stanza"), ListQueryParams),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Pagina di documenti", body = DocumentPageResponse),
        (status = 404, description = "Stanza non trovata o non visibile", body = crate::error::ProblemDetails),
    )
)]
async fn list_for_room(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(room_id): Path<Uuid>,
    Query(params): Query<ListQueryParams>,
) -> ApiResult<Json<DocumentPageResponse>> {
    list(
        state,
        context,
        AttachmentOwner::Room(RoomId::from_uuid(room_id)),
        params,
    )
    .await
}

#[utoipa::path(
    get, path = "/api/v1/leases/{lease_id}/documents", tag = "documenti",
    params(("lease_id" = Uuid, Path, description = "Identificatore del contratto"), ListQueryParams),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Pagina di documenti", body = DocumentPageResponse),
        (status = 404, description = "Contratto non trovato o non visibile", body = crate::error::ProblemDetails),
    )
)]
async fn list_for_lease(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(lease_id): Path<Uuid>,
    Query(params): Query<ListQueryParams>,
) -> ApiResult<Json<DocumentPageResponse>> {
    list(
        state,
        context,
        AttachmentOwner::Lease(LeaseId::from_uuid(lease_id)),
        params,
    )
    .await
}

#[utoipa::path(
    get, path = "/api/v1/tickets/{ticket_id}/documents", tag = "documenti",
    params(("ticket_id" = Uuid, Path, description = "Identificatore della segnalazione"), ListQueryParams),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Pagina di documenti", body = DocumentPageResponse),
        (status = 404, description = "Segnalazione non trovata o non visibile", body = crate::error::ProblemDetails),
    )
)]
async fn list_for_ticket(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(ticket_id): Path<Uuid>,
    Query(params): Query<ListQueryParams>,
) -> ApiResult<Json<DocumentPageResponse>> {
    list(
        state,
        context,
        AttachmentOwner::Ticket(TicketId::from_uuid(ticket_id)),
        params,
    )
    .await
}

async fn list(
    state: AppState,
    context: propflow_domain::RequestContext,
    owner: AttachmentOwner,
    params: ListQueryParams,
) -> ApiResult<Json<DocumentPageResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let page = state
        .list_documents
        .execute(
            &context,
            ListDocumentsCommand {
                owner,
                after: params.after.map(DocumentId::from_uuid),
                limit: params.limit,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(DocumentPageResponse {
        items: page
            .items
            .iter()
            .map(DocumentResponse::from_domain)
            .collect(),
        next_cursor: page.next_cursor.map(|id| id.as_uuid()),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/documents/{id}/content", tag = "documenti",
    params(("id" = Uuid, Path, description = "Identificatore del documento")),
    security(("bearer" = [])),
    responses(
        (status = 307, description = "Redirezione all'URL pre-firmato, in `Location`"),
        (status = 404, description = "Non trovato o non visibile", body = crate::error::ProblemDetails),
    )
)]
async fn content(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Response> {
    let request_id = context.request_id().map(str::to_owned);
    let (_, download) = state
        .download_document
        .execute(&context, DocumentId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    // `307` e non `302`: preserva il metodo e dice esplicitamente che la
    // redirezione è temporanea. L'URL scade, e un client che la mettesse in cache
    // si troverebbe con un permesso morto in mano.
    let location = header::HeaderValue::from_str(&download.url).map_err(|_| {
        ApiError::from(propflow_application::AppError::internal(
            "URL pre-firmato non rappresentabile in un header",
        ))
    })?;

    let mut response = StatusCode::TEMPORARY_REDIRECT.into_response();
    response.headers_mut().insert(header::LOCATION, location);
    // L'URL pre-firmato è un permesso a tempo: nessun intermediario deve
    // conservarlo, e nemmeno il browser.
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store, private"),
    );
    Ok(response)
}

#[utoipa::path(
    delete, path = "/api/v1/documents/{id}", tag = "documenti",
    params(("id" = Uuid, Path, description = "Identificatore del documento")),
    security(("bearer" = [])),
    responses(
        (status = 204, description = "Documento archiviato"),
        (status = 403, description = "Serve il ruolo di staff, o averlo caricato", body = crate::error::ProblemDetails),
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
        .archive_document
        .execute(
            &context,
            ArchiveDocumentCommand {
                id: DocumentId::from_uuid(id),
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;
    Ok(StatusCode::NO_CONTENT)
}
