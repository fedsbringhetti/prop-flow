use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use propflow_application::use_cases::payment::{
    DeclarePaymentCommand, ListPaymentsQuery, RecordPaymentCommand, SubmittedPayment,
};
use propflow_domain::DomainError;
use propflow_domain::payment::{Payable, Payment, PaymentMethod, PaymentStatus};
use propflow_domain::shared::{ChargeId, PaymentId, UserId};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::{Authenticated, IdempotencyKeyHeader};
use crate::routes::{amount, format_amount};
use crate::state::AppState;

/// **Due rotte di creazione e non una sola con un ramo sul ruolo.**
///
/// Registrare un incasso e dichiarare di aver pagato producono due righe diverse —
/// `succeeded` la prima, `pending` la seconda — e chiamarle con lo stesso verbo sulla
/// stessa rotta obbligherebbe l'handler a scegliere guardando il ruolo di chi
/// chiama. Quel ramo è **autorizzazione**, e l'autorizzazione vive nel service
/// layer: un handler deserializza, chiama uno use case e serializza. La stessa
/// scelta ha già dato le sotto-risorse delle segnalazioni e delle bollette.
///
/// Ne segue anche una specifica OpenAPI che dice la verità: le due operazioni hanno
/// esiti diversi, e un client sa quale sta chiamando.
///
/// **La conferma e il rifiuto sono sotto-risorse del pagamento**, non un `PATCH`
/// sullo stato: valgono le ragioni delle transizioni degli altri aggregati, e in
/// più `payments_update` concede al ruolo applicativo le sole tre colonne che
/// quelle due transizioni scrivono (migration `0025`).
///
/// **Non esiste una rotta per ritirare una dichiarazione**, ed è controintuitivo
/// abbastanza da doverlo dire: `payments_update` è staff soltanto, quindi chi
/// dichiara può creare qualcosa che poi non può disfare. L'uscita esiste ed è il
/// rifiuto, che è dello staff.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/charges/{charge_id}/payments",
            get(list_payments).post(record_payment),
        )
        .route(
            "/api/v1/charges/{charge_id}/payment-declarations",
            post(declare_payment),
        )
        .route("/api/v1/payments/{id}", get(payment_detail))
        .route("/api/v1/payments/{id}/confirm", post(confirm_payment))
        .route("/api/v1/payments/{id}/reject", post(reject_payment))
}

// ---------------------------------------------------------------------------
// Rappresentazioni di trasporto
// ---------------------------------------------------------------------------

/// Strumento con cui il denaro si è mosso.
#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodDto {
    BankTransfer,
    SepaDirectDebit,
    Card,
    Cash,
    Check,
    Other,
}

impl From<PaymentMethodDto> for PaymentMethod {
    fn from(value: PaymentMethodDto) -> Self {
        match value {
            PaymentMethodDto::BankTransfer => Self::BankTransfer,
            PaymentMethodDto::SepaDirectDebit => Self::SepaDirectDebit,
            PaymentMethodDto::Card => Self::Card,
            PaymentMethodDto::Cash => Self::Cash,
            PaymentMethodDto::Check => Self::Check,
            PaymentMethodDto::Other => Self::Other,
        }
    }
}

impl From<PaymentMethod> for PaymentMethodDto {
    fn from(value: PaymentMethod) -> Self {
        match value {
            PaymentMethod::BankTransfer => Self::BankTransfer,
            PaymentMethod::SepaDirectDebit => Self::SepaDirectDebit,
            PaymentMethod::Card => Self::Card,
            PaymentMethod::Cash => Self::Cash,
            PaymentMethod::Check => Self::Check,
            PaymentMethod::Other => Self::Other,
        }
    }
}

/// Stato di un pagamento sul filo.
///
/// Quattro valori compaiono perché una riga letta può contenerli, ma **l'API non li
/// produce mai**: `authorized` e `failed` descrivono ciò che accade presso un
/// provider, e con `manual` un provider non c'è; `refunded` e `partially_refunded`
/// appartengono ai rimborsi, che sono fuori dal perimetro di questa fetta.
#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatusDto {
    /// Dichiarato e non ancora confermato. **Non muove il saldo.**
    Pending,
    /// Mai scritto dall'API.
    Authorized,
    Succeeded,
    /// Mai scritto dall'API.
    Failed,
    /// La dichiarazione è stata rifiutata.
    Cancelled,
    /// Mai scritto dall'API.
    Refunded,
    /// Mai scritto dall'API, e **contato dal saldo** se una riga lo portasse.
    PartiallyRefunded,
}

impl From<PaymentStatus> for PaymentStatusDto {
    fn from(value: PaymentStatus) -> Self {
        match value {
            PaymentStatus::Pending => Self::Pending,
            PaymentStatus::Authorized => Self::Authorized,
            PaymentStatus::Succeeded => Self::Succeeded,
            PaymentStatus::Failed => Self::Failed,
            PaymentStatus::Cancelled => Self::Cancelled,
            PaymentStatus::Refunded => Self::Refunded,
            PaymentStatus::PartiallyRefunded => Self::PartiallyRefunded,
        }
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RecordPaymentRequest {
    /// Importo, come stringa decimale: un float binario non rappresenta
    /// esattamente 0.10, e in contabilità la differenza si accumula.
    pub amount: String,
    pub method: PaymentMethodDto,
    /// Quando il denaro si è mosso. Assente significa «adesso».
    #[serde(default)]
    pub paid_at: Option<DateTime<Utc>>,
    /// Causale o riferimento bancario.
    #[serde(default)]
    pub reference: Option<String>,
    /// A chi attribuire l'incasso.
    ///
    /// **Non è anagrafica.** `payments_select` mostra la riga a chi ha pagato:
    /// senza questo campo il pagamento resta visibile allo staff soltanto, e
    /// l'inquilino non lo vedrà nel proprio elenco. Il totale incassato gli resta
    /// comunque visibile in `paid_amount` sul saldo dell'addebito.
    #[serde(default)]
    pub payer_user_id: Option<Uuid>,
}

impl RecordPaymentRequest {
    fn into_command(
        self,
        charge_id: ChargeId,
        idempotency_key: propflow_domain::payment::IdempotencyKey,
    ) -> Result<RecordPaymentCommand, DomainError> {
        Ok(RecordPaymentCommand {
            charge_id,
            amount: amount("amount", &self.amount)?,
            method: self.method.into(),
            paid_at: self.paid_at,
            reference: self.reference,
            payer_user_id: self.payer_user_id.map(UserId::from_uuid),
            idempotency_key,
        })
    }
}

/// **Non c'è `payer_user_id`**, e non è una dimenticanza: chi dichiara è sempre chi
/// chiama, perché `payments_insert` chiede `payer_user_id = current_user_id()` a
/// chi non è staff. Accettarlo qui produrrebbe zero righe inserite, cioè
/// `auth.denied_by_policy` — il codice riservato ai nostri difetti.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct DeclarePaymentRequest {
    pub amount: String,
    pub method: PaymentMethodDto,
    /// Quando si è pagato. È un istante con fuso, e va interpretato: «ieri» per chi
    /// dichiara e `CURRENT_DATE` del database non sono necessariamente lo stesso
    /// giorno. È lo stesso scarto già registrato per la scadenza degli addebiti.
    #[serde(default)]
    pub paid_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub reference: Option<String>,
}

impl DeclarePaymentRequest {
    fn into_command(
        self,
        charge_id: ChargeId,
        idempotency_key: propflow_domain::payment::IdempotencyKey,
    ) -> Result<DeclarePaymentCommand, DomainError> {
        Ok(DeclarePaymentCommand {
            charge_id,
            amount: amount("amount", &self.amount)?,
            method: self.method.into(),
            paid_at: self.paid_at,
            reference: self.reference,
            idempotency_key,
        })
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PaymentResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    /// L'addebito che il pagamento salda. **Solo `charge` in questa fetta**: quote
    /// e regolamenti appartengono al modulo delle spese co-living.
    pub charge_id: Uuid,
    pub status: PaymentStatusDto,
    pub amount: String,
    pub currency: String,
    pub method: PaymentMethodDto,
    /// Chi ha pagato, se l'incasso è stato attribuito.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payer_user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Quando il denaro si è mosso. **Nullo su una dichiarazione rifiutata**: un
    /// pagamento che non è avvenuto non ha una data in cui è avvenuto.
    pub paid_at: Option<DateTime<Utc>>,
    /// Quando una persona ha riconosciuto il movimento.
    pub reconciled_at: Option<DateTime<Utc>>,
    /// Quanto è stato rimborsato. **Sempre `"0.00"`**: i rimborsi sono fuori dal
    /// perimetro di questa fetta.
    pub refunded_amount: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PaymentResponse {
    fn from_domain(payment: &Payment) -> Self {
        let Payable::Charge(charge_id) = payment.payable();
        Self {
            id: payment.id().as_uuid(),
            organization_id: payment.org_id().as_uuid(),
            charge_id: charge_id.as_uuid(),
            status: payment.status().into(),
            amount: format_amount(payment.amount().amount()),
            currency: payment.currency().as_str().to_owned(),
            method: payment.method().into(),
            payer_user_id: payment.payer_user_id().map(|id| id.as_uuid()),
            reference: payment.reference().map(str::to_owned),
            paid_at: payment.paid_at(),
            reconciled_at: payment.reconciled_at(),
            refunded_amount: format_amount(payment.refunded_amount().amount()),
            created_at: payment.created_at(),
            updated_at: payment.updated_at(),
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PaymentPageResponse {
    pub items: Vec<PaymentResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListPaymentsParams {
    #[serde(default)]
    pub after: Option<Uuid>,
    #[serde(default)]
    pub limit: Option<u16>,
}

/// `201` se il pagamento è stato scritto adesso, `200` se la chiave di idempotenza
/// era già stata usata e si sta restituendo quello di prima.
///
/// **Non `409`**: costringerebbe il client a distinguere «l'ho già mandato io» da
/// «qualcun altro ha usato la mia chiave», distinzione che non può fare. Vedi
/// ADR-033.
fn submitted(result: SubmittedPayment) -> (StatusCode, Json<PaymentResponse>) {
    let status = if result.replayed {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    (status, Json(PaymentResponse::from_domain(&result.payment)))
}

// ---------------------------------------------------------------------------
// Handler: deserializzare, chiamare lo use case, serializzare. Nulla di più.
// ---------------------------------------------------------------------------

#[utoipa::path(
    post, path = "/api/v1/charges/{charge_id}/payments", tag = "pagamenti",
    params(
        ("charge_id" = Uuid, Path, description = "Identificatore dell'addebito"),
        ("Idempotency-Key" = String, Header, description = "Chiave fornita dal client, obbligatoria"),
    ),
    request_body = RecordPaymentRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Incasso registrato, in stato `succeeded`", body = PaymentResponse),
        (status = 200, description = "Chiave di idempotenza già usata: il pagamento di prima", body = PaymentResponse),
        (status = 400, description = "Header `Idempotency-Key` assente o malformato", body = crate::error::ProblemDetails),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Addebito non trovato", body = crate::error::ProblemDetails),
        (status = 409, description = "Addebito annullato o stornato, o chiave già usata per un altro pagamento", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi", body = crate::error::ProblemDetails),
    )
)]
async fn record_payment(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(charge_id): Path<Uuid>,
    IdempotencyKeyHeader(key): IdempotencyKeyHeader,
    Json(request): Json<RecordPaymentRequest>,
) -> ApiResult<(StatusCode, Json<PaymentResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let command = request
        .into_command(ChargeId::from_uuid(charge_id), key)
        .map_err(|error| {
            ApiError::from(propflow_application::AppError::Validation(error))
                .with_request_id(request_id.clone())
        })?;

    let result = state
        .record_payment
        .execute(&context, command)
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(submitted(result))
}

#[utoipa::path(
    post, path = "/api/v1/charges/{charge_id}/payment-declarations", tag = "pagamenti",
    params(
        ("charge_id" = Uuid, Path, description = "Identificatore dell'addebito"),
        ("Idempotency-Key" = String, Header, description = "Chiave fornita dal client, obbligatoria"),
    ),
    request_body = DeclarePaymentRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Dichiarazione registrata, in stato `pending`: **non muove il saldo** finché lo staff non la conferma", body = PaymentResponse),
        (status = 200, description = "Chiave di idempotenza già usata: la dichiarazione di prima", body = PaymentResponse),
        (status = 400, description = "Header `Idempotency-Key` assente o malformato", body = crate::error::ProblemDetails),
        (status = 404, description = "Addebito non trovato", body = crate::error::ProblemDetails),
        (status = 409, description = "Chiave già usata per un altro pagamento", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi", body = crate::error::ProblemDetails),
    )
)]
async fn declare_payment(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(charge_id): Path<Uuid>,
    IdempotencyKeyHeader(key): IdempotencyKeyHeader,
    Json(request): Json<DeclarePaymentRequest>,
) -> ApiResult<(StatusCode, Json<PaymentResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let command = request
        .into_command(ChargeId::from_uuid(charge_id), key)
        .map_err(|error| {
            ApiError::from(propflow_application::AppError::Validation(error))
                .with_request_id(request_id.clone())
        })?;

    let result = state
        .declare_payment
        .execute(&context, command)
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(submitted(result))
}

#[utoipa::path(
    get, path = "/api/v1/charges/{charge_id}/payments", tag = "pagamenti",
    params(
        ("charge_id" = Uuid, Path, description = "Identificatore dell'addebito"),
        ListPaymentsParams,
    ),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Pagina di pagamenti dell'addebito", body = PaymentPageResponse),
        (status = 404, description = "Addebito non trovato", body = crate::error::ProblemDetails),
    )
)]
async fn list_payments(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(charge_id): Path<Uuid>,
    Query(params): Query<ListPaymentsParams>,
) -> ApiResult<Json<PaymentPageResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let page = state
        .list_payments
        .execute(
            &context,
            ListPaymentsQuery {
                charge_id: ChargeId::from_uuid(charge_id),
                after: params.after.map(PaymentId::from_uuid),
                limit: params.limit,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(PaymentPageResponse {
        items: page
            .items
            .iter()
            .map(PaymentResponse::from_domain)
            .collect(),
        next_cursor: page.next_cursor.map(|id| id.as_uuid()),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/payments/{id}", tag = "pagamenti",
    params(("id" = Uuid, Path, description = "Identificatore del pagamento")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Pagamento", body = PaymentResponse),
        (status = 404, description = "Pagamento non trovato", body = crate::error::ProblemDetails),
    )
)]
async fn payment_detail(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<PaymentResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let payment = state
        .get_payment
        .execute(&context, PaymentId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(PaymentResponse::from_domain(&payment)))
}

#[utoipa::path(
    post, path = "/api/v1/payments/{id}/confirm", tag = "pagamenti",
    params(("id" = Uuid, Path, description = "Identificatore del pagamento")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Dichiarazione confermata: da qui il saldo si muove e lo stato dell'addebito si ricalcola", body = PaymentResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Pagamento o addebito non trovati", body = crate::error::ProblemDetails),
        (status = 409, description = "Il pagamento non è in `pending`, o l'addebito è annullato o stornato", body = crate::error::ProblemDetails),
    )
)]
async fn confirm_payment(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<PaymentResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let payment = state
        .confirm_payment
        .execute(&context, PaymentId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(PaymentResponse::from_domain(&payment)))
}

#[utoipa::path(
    post, path = "/api/v1/payments/{id}/reject", tag = "pagamenti",
    params(("id" = Uuid, Path, description = "Identificatore del pagamento")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Dichiarazione rifiutata: passa a `cancelled` e il saldo resta dov'era", body = PaymentResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Pagamento non trovato", body = crate::error::ProblemDetails),
        (status = 409, description = "Il pagamento non è in `pending`", body = crate::error::ProblemDetails),
    )
)]
async fn reject_payment(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<PaymentResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let payment = state
        .reject_payment
        .execute(&context, PaymentId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(PaymentResponse::from_domain(&payment)))
}
