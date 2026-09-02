use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, NaiveDate, Utc};
use propflow_application::use_cases::expense::{
    AddGroupMemberCommand, CreateExpenseGroupCommand, DeclareSettlementCommand,
    ListExpenseGroupsQuery, ListExpensesQuery, ListSettlementsQuery, RecordExpenseCommand,
};
use propflow_domain::DomainError;
use propflow_domain::expense::{
    Expense, ExpenseCategory, ExpenseGroup, ExpenseGroupKind, ExpenseGroupMember, ExpenseShare,
    GroupBalance, Settlement, SettlementStatus, SplitMethod, Weight,
};
use propflow_domain::shared::{ExpenseGroupId, ExpenseId, SettlementId, UserId};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::Authenticated;
use crate::routes::payments::PaymentMethodDto;
use crate::routes::{amount, format_amount};
use crate::state::AppState;

/// Il denaro che i coinquilini si devono **fra loro**.
///
/// # Perché quasi tutto è `ensure_member` e non `ensure_staff`
///
/// È l'opposto della fetta contabile, e non è una concessione: le policy della
/// `0014` erano già scritte così. `expense_groups_insert` chiede `is_org_member`,
/// tutto il resto chiede `can_access_expense_group`. Il proprietario può guardare
/// — è staff dell'organizzazione — ma un co-living che non può organizzarsi da sé
/// non è il prodotto descritto in `docs/01-prodotto.md`.
///
/// # Le sotto-risorse, e perché non un `PATCH` sullo stato
///
/// Confermare, rifiutare e ritirare un regolamento sono tre sotto-risorse e non
/// tre valori di un campo: valgono le ragioni delle transizioni degli altri
/// aggregati, e in più qui **le tre autorizzano in modo diverso** — le prime due
/// spettano a chi riceve, la terza a chi ha dichiarato. Un `PATCH` obbligherebbe
/// l'handler a scegliere il controllo guardando il corpo, cioè a fare
/// autorizzazione.
///
/// # `DELETE /groups/{id}/members/{user_id}` esce, non cancella
///
/// La riga resta, con `left_at` valorizzato. Deve restare: le quote intestate a
/// quella persona restano, e dalla migration `0027` chi esce con un conto aperto
/// continua a comparire nei saldi finché il suo netto non è zero. Il verbo è
/// `DELETE` perché è l'operazione che il client vuole esprimere — «non fa più
/// parte del gruppo» — e la risposta `200` con la membership aggiornata dice che
/// cosa è davvero successo.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/expense-groups",
            get(list_groups).post(create_group),
        )
        .route("/api/v1/expense-groups/{id}", get(group_detail))
        .route("/api/v1/expense-groups/{id}/archive", post(archive_group))
        .route("/api/v1/expense-groups/{id}/members", post(add_member))
        .route(
            "/api/v1/expense-groups/{id}/members/{user_id}",
            axum::routing::delete(leave_group),
        )
        .route("/api/v1/expense-groups/{id}/balances", get(group_balances))
        .route(
            "/api/v1/expense-groups/{id}/expenses",
            get(list_expenses).post(record_expense),
        )
        .route("/api/v1/expenses/{id}", get(expense_detail))
        .route(
            "/api/v1/expense-groups/{id}/settlements",
            get(list_settlements).post(declare_settlement),
        )
        .route("/api/v1/settlements/{id}/confirm", post(confirm_settlement))
        .route("/api/v1/settlements/{id}/reject", post(reject_settlement))
        .route("/api/v1/settlements/{id}/cancel", post(cancel_settlement))
}

// ---------------------------------------------------------------------------
// Rappresentazioni di trasporto
// ---------------------------------------------------------------------------

/// Che cosa il gruppo tiene insieme.
#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExpenseGroupKindDto {
    Coliving,
    Household,
    Property,
}

impl From<ExpenseGroupKindDto> for ExpenseGroupKind {
    fn from(value: ExpenseGroupKindDto) -> Self {
        match value {
            ExpenseGroupKindDto::Coliving => Self::Coliving,
            ExpenseGroupKindDto::Household => Self::Household,
            ExpenseGroupKindDto::Property => Self::Property,
        }
    }
}

impl From<ExpenseGroupKind> for ExpenseGroupKindDto {
    fn from(value: ExpenseGroupKind) -> Self {
        match value {
            ExpenseGroupKind::Coliving => Self::Coliving,
            ExpenseGroupKind::Household => Self::Household,
            ExpenseGroupKind::Property => Self::Property,
        }
    }
}

/// Come una spesa si divide.
///
/// **`percentage`, `exact` e `by_usage` compaiono e l'API non li accetta**: una
/// richiesta che li nomina riceve `422`, non una ripartizione approssimata. I primi
/// due chiedono un input per membro che questa fetta non ha; il terzo chiede
/// letture di contatori che non esistono.
#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SplitMethodDto {
    /// In parti uguali fra i membri attivi.
    Equal,
    /// In proporzione al peso di ciascuno: chi ha la doppia paga di più.
    Weighted,
    /// **Rifiutato con `422`.**
    Percentage,
    /// **Rifiutato con `422`.**
    Exact,
    /// **Rifiutato con `422`.**
    ByUsage,
}

impl From<SplitMethodDto> for SplitMethod {
    fn from(value: SplitMethodDto) -> Self {
        match value {
            SplitMethodDto::Equal => Self::Equal,
            SplitMethodDto::Weighted => Self::Weighted,
            SplitMethodDto::Percentage => Self::Percentage,
            SplitMethodDto::Exact => Self::Exact,
            SplitMethodDto::ByUsage => Self::ByUsage,
        }
    }
}

impl From<SplitMethod> for SplitMethodDto {
    fn from(value: SplitMethod) -> Self {
        match value {
            SplitMethod::Equal => Self::Equal,
            SplitMethod::Weighted => Self::Weighted,
            SplitMethod::Percentage => Self::Percentage,
            SplitMethod::Exact => Self::Exact,
            SplitMethod::ByUsage => Self::ByUsage,
        }
    }
}

/// Che cosa si è comprato.
#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExpenseCategoryDto {
    Utilities,
    Groceries,
    Cleaning,
    Connectivity,
    Maintenance,
    Rent,
    Furniture,
    Other,
}

impl From<ExpenseCategoryDto> for ExpenseCategory {
    fn from(value: ExpenseCategoryDto) -> Self {
        match value {
            ExpenseCategoryDto::Utilities => Self::Utilities,
            ExpenseCategoryDto::Groceries => Self::Groceries,
            ExpenseCategoryDto::Cleaning => Self::Cleaning,
            ExpenseCategoryDto::Connectivity => Self::Connectivity,
            ExpenseCategoryDto::Maintenance => Self::Maintenance,
            ExpenseCategoryDto::Rent => Self::Rent,
            ExpenseCategoryDto::Furniture => Self::Furniture,
            ExpenseCategoryDto::Other => Self::Other,
        }
    }
}

impl From<ExpenseCategory> for ExpenseCategoryDto {
    fn from(value: ExpenseCategory) -> Self {
        match value {
            ExpenseCategory::Utilities => Self::Utilities,
            ExpenseCategory::Groceries => Self::Groceries,
            ExpenseCategory::Cleaning => Self::Cleaning,
            ExpenseCategory::Connectivity => Self::Connectivity,
            ExpenseCategory::Maintenance => Self::Maintenance,
            ExpenseCategory::Rent => Self::Rent,
            ExpenseCategory::Furniture => Self::Furniture,
            ExpenseCategory::Other => Self::Other,
        }
    }
}

/// Stato di un regolamento.
///
/// **Tutti e quattro l'API li produce**, a differenza degli stati di un pagamento
/// dove quattro su sette non si scrivono mai. La ragione non è di prodotto: la
/// `USING` di `settlements_update` nomina anche `from_user_id`, quindi chi dichiara
/// può ritirare — e `cancelled` è la sua uscita.
#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SettlementStatusDto {
    /// Dichiarato e non ancora confermato. **Non muove i saldi.**
    Pending,
    /// Chi riceve ha confermato. Da qui i saldi si muovono.
    Confirmed,
    /// Chi riceve dice che il denaro non è arrivato.
    Rejected,
    /// Chi ha dichiarato ha ritirato.
    Cancelled,
}

impl From<SettlementStatusDto> for SettlementStatus {
    fn from(value: SettlementStatusDto) -> Self {
        match value {
            SettlementStatusDto::Pending => Self::Pending,
            SettlementStatusDto::Confirmed => Self::Confirmed,
            SettlementStatusDto::Rejected => Self::Rejected,
            SettlementStatusDto::Cancelled => Self::Cancelled,
        }
    }
}

impl From<SettlementStatus> for SettlementStatusDto {
    fn from(value: SettlementStatus) -> Self {
        match value {
            SettlementStatus::Pending => Self::Pending,
            SettlementStatus::Confirmed => Self::Confirmed,
            SettlementStatus::Rejected => Self::Rejected,
            SettlementStatus::Cancelled => Self::Cancelled,
        }
    }
}

/// Converte un peso dal filo.
///
/// **Come gli importi è una stringa decimale, e non condivide la funzione**:
/// `app.expense_group_members.weight` è `NUMERIC(6,3)` mentre gli importi sono
/// `NUMERIC(14,2)`, quindi passare da `amount` accetterebbe due decimali dove ne
/// servono tre. È la stessa distinzione che il value object `Weight` esiste per
/// fare.
fn weight(field: &'static str, raw: Option<&str>) -> Result<Weight, DomainError> {
    let Some(raw) = raw else {
        return Ok(Weight::one());
    };
    let value = raw
        .trim()
        .parse::<Decimal>()
        .map_err(|_| DomainError::validation(field, "must be a decimal weight"))?;
    Weight::parse(value)
}

/// Formatta un peso per il filo: sempre tre decimali.
///
/// La scala di un `Decimal` dipende da come è stato costruito, quindi senza
/// normalizzare lo stesso peso uscirebbe `"1"` alla creazione e `"1.000"` alla
/// lettura successiva. È lo stesso argomento di `format_amount`, con tre decimali
/// invece di due perché il tipo a schema ne ha tre.
fn format_weight(value: Weight) -> String {
    format!("{:.3}", value.value())
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateExpenseGroupRequest {
    pub name: String,
    #[serde(default)]
    pub kind: Option<ExpenseGroupKindDto>,
    /// L'immobile a cui il gruppo si riferisce, se ce n'è uno. È **opzionale**, ed
    /// è la ragione per cui la ripartizione automatica di una bolletta resta fuori
    /// da questa fetta: senza immobile non c'è modo di dire quale bolletta
    /// riguardi quale gruppo.
    #[serde(default)]
    pub property_id: Option<Uuid>,
    /// La consuetudine del gruppo, non una regola: ogni spesa può dichiararne una
    /// diversa.
    #[serde(default)]
    pub default_split_method: Option<SplitMethodDto>,
    /// Il peso di chi crea, che è anche il primo membro. Come stringa decimale,
    /// fino a tre decimali. Assente vale `"1"`.
    #[serde(default)]
    pub weight: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AddGroupMemberRequest {
    pub user_id: Uuid,
    /// La stanza occupata, quando il gruppo è legato a un immobile in co-living.
    #[serde(default)]
    pub room_id: Option<Uuid>,
    #[serde(default)]
    pub display_name: Option<String>,
    /// Il peso per la ripartizione `weighted`: `"2"` per una doppia. Assente vale
    /// `"1"`.
    #[serde(default)]
    pub weight: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RecordExpenseRequest {
    pub description: String,
    /// Importo, come stringa decimale: un float binario non rappresenta
    /// esattamente 0.10, e in contabilità la differenza si accumula.
    pub amount: String,
    #[serde(default)]
    pub category: Option<ExpenseCategoryDto>,
    /// Chi ha anticipato. Assente significa «io».
    ///
    /// Deve essere un **membro attivo** del gruppo: lo schema non lo impone, e
    /// senza il controllo si scriverebbe un credito che nessun saldo mostrerebbe.
    #[serde(default)]
    pub paid_by_user_id: Option<Uuid>,
    /// Quando si è comprato. Assente significa «oggi». È una data e non un
    /// istante, quindi il fuso di chi guarda non entra in gioco.
    #[serde(default)]
    pub expense_date: Option<NaiveDate>,
    /// Assente usa il metodo predefinito del gruppo.
    #[serde(default)]
    pub split_method: Option<SplitMethodDto>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// **Non c'è `from_user_id`**, e non è una dimenticanza: chi dichiara è sempre chi
/// chiama, perché `settlements_insert` chiede `from_user_id = current_user_id()` a
/// chi non è staff. Accettarlo qui produrrebbe zero righe inserite, cioè
/// `auth.denied_by_policy` — il codice riservato ai nostri difetti.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct DeclareSettlementRequest {
    /// A chi si è pagato.
    pub to_user_id: Uuid,
    pub amount: String,
    #[serde(default)]
    pub method: Option<PaymentMethodDto>,
    /// Quando il denaro si è mosso. Assente significa «adesso», e la conferma lo
    /// risolve.
    #[serde(default)]
    pub settled_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ExpenseGroupMemberResponse {
    pub user_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Il peso per la ripartizione `weighted`, con tre decimali.
    pub weight: String,
    pub joined_at: DateTime<Utc>,
    /// Quando è uscito. **La riga resta**: le quote a suo nome restano, e finché
    /// il suo saldo non è zero compare ancora fra i saldi del gruppo.
    pub left_at: Option<DateTime<Utc>>,
}

impl ExpenseGroupMemberResponse {
    fn from_domain(member: &ExpenseGroupMember) -> Self {
        Self {
            user_id: member.user_id().as_uuid(),
            room_id: member.room_id().map(|id| id.as_uuid()),
            display_name: member.display_name().map(str::to_owned),
            weight: format_weight(member.weight()),
            joined_at: member.joined_at(),
            left_at: member.left_at(),
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ExpenseGroupResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_id: Option<Uuid>,
    pub kind: ExpenseGroupKindDto,
    pub name: String,
    pub currency: String,
    pub default_split_method: SplitMethodDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<Uuid>,
    /// Il conto è chiuso: non accetta più spese né regolamenti. **Non è una
    /// cancellazione**: i saldi e le spese restano leggibili.
    pub is_archived: bool,
    /// La composizione, **chi è uscito compreso**: senza di lui un client non
    /// saprebbe spiegare le quote di una spesa vecchia.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<ExpenseGroupMemberResponse>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ExpenseGroupResponse {
    fn from_domain(group: &ExpenseGroup, members: Option<&[ExpenseGroupMember]>) -> Self {
        Self {
            id: group.id().as_uuid(),
            organization_id: group.org_id().as_uuid(),
            property_id: group.property_id().map(|id| id.as_uuid()),
            kind: group.kind().into(),
            name: group.name().to_owned(),
            currency: group.currency().as_str().to_owned(),
            default_split_method: group.default_split_method().into(),
            created_by: group.created_by().map(|id| id.as_uuid()),
            is_archived: group.is_archived(),
            members: members.map(|members| {
                members
                    .iter()
                    .map(ExpenseGroupMemberResponse::from_domain)
                    .collect()
            }),
            created_at: group.created_at(),
            updated_at: group.updated_at(),
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ExpenseGroupPageResponse {
    pub items: Vec<ExpenseGroupResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ExpenseShareResponse {
    pub user_id: Uuid,
    pub share_amount: String,
    /// Il peso con cui la quota è stata calcolata. **Nullo su una divisione in
    /// parti uguali**, dove il peso non ha partecipato al calcolo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share_weight: Option<String>,
}

impl ExpenseShareResponse {
    fn from_domain(share: &ExpenseShare) -> Self {
        Self {
            user_id: share.user_id().as_uuid(),
            share_amount: format_amount(share.share_amount().amount()),
            share_weight: share.share_weight().map(format_weight),
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ExpenseResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub group_id: Uuid,
    /// Chi ha anticipato: è il verso del credito.
    pub paid_by_user_id: Uuid,
    pub category: ExpenseCategoryDto,
    pub description: String,
    pub amount: String,
    pub currency: String,
    pub expense_date: NaiveDate,
    pub split_method: SplitMethodDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Le quote. **La loro somma è esattamente `amount`**, sempre: il trigger
    /// `expense_shares_balanced` non ammette altro.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shares: Option<Vec<ExpenseShareResponse>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ExpenseResponse {
    fn from_domain(expense: &Expense, shares: Option<&[ExpenseShare]>) -> Self {
        Self {
            id: expense.id().as_uuid(),
            organization_id: expense.org_id().as_uuid(),
            group_id: expense.group_id().as_uuid(),
            paid_by_user_id: expense.paid_by_user_id().as_uuid(),
            category: expense.category().into(),
            description: expense.description().to_owned(),
            amount: format_amount(expense.amount().amount()),
            currency: expense.currency().as_str().to_owned(),
            expense_date: expense.expense_date(),
            split_method: expense.split_method().into(),
            notes: expense.notes().map(str::to_owned),
            shares: shares.map(|shares| {
                shares
                    .iter()
                    .map(ExpenseShareResponse::from_domain)
                    .collect()
            }),
            created_at: expense.created_at(),
            updated_at: expense.updated_at(),
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ExpensePageResponse {
    pub items: Vec<ExpenseResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Uuid>,
}

/// Il saldo di una persona nel gruppo.
///
/// `net_balance` positivo = **in credito**, il gruppo gli deve. Negativo = in
/// debito. **La somma dei `net_balance` di un gruppo fa zero**, ed è l'invariante
/// che rende questi numeri leggibili.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct GroupBalanceResponse {
    pub user_id: Uuid,
    pub total_paid: String,
    pub total_owed: String,
    pub total_settled_out: String,
    pub total_settled_in: String,
    /// Può essere **negativo**: è metà dell'informazione per cui il campo esiste.
    pub net_balance: String,
}

impl GroupBalanceResponse {
    fn from_domain(balance: &GroupBalance) -> Self {
        Self {
            user_id: balance.user_id.as_uuid(),
            total_paid: format_amount(balance.total_paid.amount()),
            total_owed: format_amount(balance.total_owed.amount()),
            total_settled_out: format_amount(balance.total_settled_out.amount()),
            total_settled_in: format_amount(balance.total_settled_in.amount()),
            net_balance: format_amount(balance.net_balance.amount()),
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct GroupBalancesResponse {
    /// I creditori per primi, e a parità l'ordine per `user_id` della
    /// ripartizione.
    ///
    /// **Comprende chi è uscito dal gruppo con un conto aperto**, dalla migration
    /// `0027`: prima quel debito spariva con lui, e la somma dei saldi smetteva di
    /// fare zero. Chi esce in pari non compare.
    pub items: Vec<GroupBalanceResponse>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct SettlementResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub group_id: Uuid,
    /// Chi paga, cioè chi ha dichiarato.
    pub from_user_id: Uuid,
    /// Chi riceve, cioè chi deve confermare.
    pub to_user_id: Uuid,
    pub amount: String,
    pub currency: String,
    pub status: SettlementStatusDto,
    pub method: PaymentMethodDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Quando il denaro si è mosso. **Nullo su un regolamento rifiutato o
    /// ritirato**: un trasferimento che non è avvenuto non ha una data in cui è
    /// avvenuto.
    pub settled_at: Option<DateTime<Utc>>,
    pub confirmed_at: Option<DateTime<Utc>>,
    /// Chi ha confermato. È il creditore, o uno staff che ha sistemato il conto.
    pub confirmed_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SettlementResponse {
    fn from_domain(settlement: &Settlement) -> Self {
        Self {
            id: settlement.id().as_uuid(),
            organization_id: settlement.org_id().as_uuid(),
            group_id: settlement.group_id().as_uuid(),
            from_user_id: settlement.from_user_id().as_uuid(),
            to_user_id: settlement.to_user_id().as_uuid(),
            amount: format_amount(settlement.amount().amount()),
            currency: settlement.currency().as_str().to_owned(),
            status: settlement.status().into(),
            method: settlement.method().into(),
            note: settlement.note().map(str::to_owned),
            settled_at: settlement.settled_at(),
            confirmed_at: settlement.confirmed_at(),
            confirmed_by: settlement.confirmed_by().map(|id| id.as_uuid()),
            created_at: settlement.created_at(),
            updated_at: settlement.updated_at(),
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct SettlementPageResponse {
    pub items: Vec<SettlementResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListGroupsParams {
    #[serde(default)]
    pub property_id: Option<Uuid>,
    #[serde(default)]
    pub is_archived: Option<bool>,
    #[serde(default)]
    pub after: Option<Uuid>,
    #[serde(default)]
    pub limit: Option<u16>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListExpensesParams {
    #[serde(default)]
    pub category: Option<ExpenseCategoryDto>,
    #[serde(default)]
    pub after: Option<Uuid>,
    #[serde(default)]
    pub limit: Option<u16>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListSettlementsParams {
    #[serde(default)]
    pub status: Option<SettlementStatusDto>,
    #[serde(default)]
    pub after: Option<Uuid>,
    #[serde(default)]
    pub limit: Option<u16>,
}

// ---------------------------------------------------------------------------
// Handler: deserializzare, chiamare lo use case, serializzare. Nulla di più.
// ---------------------------------------------------------------------------

#[utoipa::path(
    post, path = "/api/v1/expense-groups", tag = "spese co-living",
    request_body = CreateExpenseGroupRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Gruppo creato, con **chi lo ha creato già dentro**: le due scritture sono nella stessa transazione (migration `0026`)", body = ExpenseGroupResponse),
        (status = 403, description = "Richiede l'appartenenza all'organizzazione", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi, o un metodo di ripartizione fuori perimetro", body = crate::error::ProblemDetails),
    )
)]
async fn create_group(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Json(request): Json<CreateExpenseGroupRequest>,
) -> ApiResult<(StatusCode, Json<ExpenseGroupResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let weight = weight("weight", request.weight.as_deref()).map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;

    let (group, member) = state
        .create_expense_group
        .execute(
            &context,
            CreateExpenseGroupCommand {
                property_id: request
                    .property_id
                    .map(propflow_domain::shared::PropertyId::from_uuid),
                kind: request.kind.unwrap_or(ExpenseGroupKindDto::Coliving).into(),
                name: request.name,
                default_split_method: request
                    .default_split_method
                    .unwrap_or(SplitMethodDto::Equal)
                    .into(),
                weight,
                display_name: request.display_name,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok((
        StatusCode::CREATED,
        Json(ExpenseGroupResponse::from_domain(
            &group,
            Some(std::slice::from_ref(&member)),
        )),
    ))
}

#[utoipa::path(
    get, path = "/api/v1/expense-groups", tag = "spese co-living",
    params(ListGroupsParams),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Pagina di gruppi. La composizione non c'è: si legge dal dettaglio", body = ExpenseGroupPageResponse),
    )
)]
async fn list_groups(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Query(params): Query<ListGroupsParams>,
) -> ApiResult<Json<ExpenseGroupPageResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let page = state
        .list_expense_groups
        .execute(
            &context,
            ListExpenseGroupsQuery {
                property_id: params
                    .property_id
                    .map(propflow_domain::shared::PropertyId::from_uuid),
                is_archived: params.is_archived,
                after: params.after.map(ExpenseGroupId::from_uuid),
                limit: params.limit,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(ExpenseGroupPageResponse {
        items: page
            .items
            .iter()
            .map(|group| ExpenseGroupResponse::from_domain(group, None))
            .collect(),
        next_cursor: page.next_cursor.map(|id| id.as_uuid()),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/expense-groups/{id}", tag = "spese co-living",
    params(("id" = Uuid, Path, description = "Identificatore del gruppo")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Gruppo con la sua composizione, chi è uscito compreso", body = ExpenseGroupResponse),
        (status = 404, description = "Gruppo non trovato", body = crate::error::ProblemDetails),
    )
)]
async fn group_detail(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ExpenseGroupResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let (group, members) = state
        .get_expense_group
        .execute(&context, ExpenseGroupId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(ExpenseGroupResponse::from_domain(
        &group,
        Some(&members),
    )))
}

#[utoipa::path(
    post, path = "/api/v1/expense-groups/{id}/archive", tag = "spese co-living",
    params(("id" = Uuid, Path, description = "Identificatore del gruppo")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Conto chiuso. **Non è una cancellazione**: spese e saldi restano leggibili, e si chiude anche con dei conti aperti", body = ExpenseGroupResponse),
        (status = 404, description = "Gruppo non trovato", body = crate::error::ProblemDetails),
        (status = 409, description = "Il gruppo è già archiviato", body = crate::error::ProblemDetails),
    )
)]
async fn archive_group(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ExpenseGroupResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let group = state
        .archive_expense_group
        .execute(&context, ExpenseGroupId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(ExpenseGroupResponse::from_domain(&group, None)))
}

#[utoipa::path(
    post, path = "/api/v1/expense-groups/{id}/members", tag = "spese co-living",
    params(("id" = Uuid, Path, description = "Identificatore del gruppo")),
    request_body = AddGroupMemberRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Iscritto, **o rientrato**: chi è già stato nel gruppo non ha una seconda riga, e il rientro è un aggiornamento", body = ExpenseGroupMemberResponse),
        (status = 404, description = "Gruppo non trovato", body = crate::error::ProblemDetails),
        (status = 409, description = "La persona è già un membro attivo, o il gruppo è archiviato", body = crate::error::ProblemDetails),
    )
)]
async fn add_member(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<AddGroupMemberRequest>,
) -> ApiResult<(StatusCode, Json<ExpenseGroupMemberResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let weight = weight("weight", request.weight.as_deref()).map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;

    let member = state
        .add_group_member
        .execute(
            &context,
            AddGroupMemberCommand {
                group_id: ExpenseGroupId::from_uuid(id),
                user_id: UserId::from_uuid(request.user_id),
                room_id: request
                    .room_id
                    .map(propflow_domain::shared::RoomId::from_uuid),
                display_name: request.display_name,
                weight,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok((
        StatusCode::CREATED,
        Json(ExpenseGroupMemberResponse::from_domain(&member)),
    ))
}

#[utoipa::path(
    delete, path = "/api/v1/expense-groups/{id}/members/{user_id}", tag = "spese co-living",
    params(
        ("id" = Uuid, Path, description = "Identificatore del gruppo"),
        ("user_id" = Uuid, Path, description = "Chi esce"),
    ),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Uscito. **La riga resta** con `left_at` valorizzato: le quote a suo nome restano, e finché il suo saldo non è zero compare ancora fra i saldi del gruppo", body = ExpenseGroupMemberResponse),
        (status = 403, description = "Si esce per sé stessi: togliere qualcun altro richiede il ruolo di staff", body = crate::error::ProblemDetails),
        (status = 404, description = "Gruppo o membership non trovati", body = crate::error::ProblemDetails),
        (status = 409, description = "La persona era già uscita", body = crate::error::ProblemDetails),
    )
)]
async fn leave_group(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<ExpenseGroupMemberResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let member = state
        .leave_group
        .execute(
            &context,
            ExpenseGroupId::from_uuid(id),
            UserId::from_uuid(user_id),
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(ExpenseGroupMemberResponse::from_domain(&member)))
}

#[utoipa::path(
    get, path = "/api/v1/expense-groups/{id}/balances", tag = "spese co-living",
    params(("id" = Uuid, Path, description = "Identificatore del gruppo")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Chi è in credito e chi in debito. La somma dei `net_balance` fa **zero**", body = GroupBalancesResponse),
        (status = 404, description = "Gruppo non trovato", body = crate::error::ProblemDetails),
    )
)]
async fn group_balances(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<GroupBalancesResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let balances = state
        .get_group_balances
        .execute(&context, ExpenseGroupId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(GroupBalancesResponse {
        items: balances
            .iter()
            .map(GroupBalanceResponse::from_domain)
            .collect(),
    }))
}

#[utoipa::path(
    post, path = "/api/v1/expense-groups/{id}/expenses", tag = "spese co-living",
    params(("id" = Uuid, Path, description = "Identificatore del gruppo")),
    request_body = RecordExpenseRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Spesa registrata **con le sue quote**, la cui somma è esattamente l'importo", body = ExpenseResponse),
        (status = 404, description = "Gruppo non trovato", body = crate::error::ProblemDetails),
        (status = 409, description = "Il gruppo è archiviato, o chi ha anticipato non è un membro attivo", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi, o un metodo di ripartizione fuori perimetro", body = crate::error::ProblemDetails),
    )
)]
async fn record_expense(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<RecordExpenseRequest>,
) -> ApiResult<(StatusCode, Json<ExpenseResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let amount = amount("amount", &request.amount).map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;

    let recorded = state
        .record_expense
        .execute(
            &context,
            RecordExpenseCommand {
                group_id: ExpenseGroupId::from_uuid(id),
                paid_by_user_id: request.paid_by_user_id.map(UserId::from_uuid),
                category: request.category.unwrap_or(ExpenseCategoryDto::Other).into(),
                description: request.description,
                amount,
                expense_date: request.expense_date,
                split_method: request.split_method.map(Into::into),
                notes: request.notes,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok((
        StatusCode::CREATED,
        Json(ExpenseResponse::from_domain(
            &recorded.expense,
            Some(&recorded.shares),
        )),
    ))
}

#[utoipa::path(
    get, path = "/api/v1/expense-groups/{id}/expenses", tag = "spese co-living",
    params(
        ("id" = Uuid, Path, description = "Identificatore del gruppo"),
        ListExpensesParams,
    ),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Pagina di spese. Le quote non ci sono: si leggono dal dettaglio", body = ExpensePageResponse),
        (status = 404, description = "Gruppo non trovato", body = crate::error::ProblemDetails),
    )
)]
async fn list_expenses(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Query(params): Query<ListExpensesParams>,
) -> ApiResult<Json<ExpensePageResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let page = state
        .list_expenses
        .execute(
            &context,
            ListExpensesQuery {
                group_id: ExpenseGroupId::from_uuid(id),
                category: params.category.map(Into::into),
                after: params.after.map(ExpenseId::from_uuid),
                limit: params.limit,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(ExpensePageResponse {
        items: page
            .items
            .iter()
            .map(|expense| ExpenseResponse::from_domain(expense, None))
            .collect(),
        next_cursor: page.next_cursor.map(|id| id.as_uuid()),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/expenses/{id}", tag = "spese co-living",
    params(("id" = Uuid, Path, description = "Identificatore della spesa")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Spesa con le sue quote", body = ExpenseResponse),
        (status = 404, description = "Spesa non trovata", body = crate::error::ProblemDetails),
    )
)]
async fn expense_detail(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ExpenseResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let (expense, shares) = state
        .get_expense
        .execute(&context, ExpenseId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(ExpenseResponse::from_domain(&expense, Some(&shares))))
}

#[utoipa::path(
    post, path = "/api/v1/expense-groups/{id}/settlements", tag = "spese co-living",
    params(("id" = Uuid, Path, description = "Identificatore del gruppo")),
    request_body = DeclareSettlementRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Dichiarato, in stato `pending`: **non muove i saldi** finché chi riceve non conferma", body = SettlementResponse),
        (status = 404, description = "Gruppo non trovato", body = crate::error::ProblemDetails),
        (status = 409, description = "Il gruppo è archiviato", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi, o destinatario uguale a chi dichiara", body = crate::error::ProblemDetails),
    )
)]
async fn declare_settlement(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<DeclareSettlementRequest>,
) -> ApiResult<(StatusCode, Json<SettlementResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let amount = amount("amount", &request.amount).map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;

    let settlement = state
        .declare_settlement
        .execute(
            &context,
            DeclareSettlementCommand {
                group_id: ExpenseGroupId::from_uuid(id),
                to_user_id: UserId::from_uuid(request.to_user_id),
                amount,
                method: request
                    .method
                    .unwrap_or(PaymentMethodDto::BankTransfer)
                    .into(),
                settled_at: request.settled_at,
                note: request.note,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok((
        StatusCode::CREATED,
        Json(SettlementResponse::from_domain(&settlement)),
    ))
}

#[utoipa::path(
    get, path = "/api/v1/expense-groups/{id}/settlements", tag = "spese co-living",
    params(
        ("id" = Uuid, Path, description = "Identificatore del gruppo"),
        ListSettlementsParams,
    ),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Pagina di regolamenti. `?status=pending` è come chi riceve trova che cosa confermare", body = SettlementPageResponse),
        (status = 404, description = "Gruppo non trovato", body = crate::error::ProblemDetails),
    )
)]
async fn list_settlements(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Query(params): Query<ListSettlementsParams>,
) -> ApiResult<Json<SettlementPageResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let page = state
        .list_settlements
        .execute(
            &context,
            ListSettlementsQuery {
                group_id: ExpenseGroupId::from_uuid(id),
                status: params.status.map(Into::into),
                after: params.after.map(SettlementId::from_uuid),
                limit: params.limit,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(SettlementPageResponse {
        items: page
            .items
            .iter()
            .map(SettlementResponse::from_domain)
            .collect(),
        next_cursor: page.next_cursor.map(|id| id.as_uuid()),
    }))
}

#[utoipa::path(
    post, path = "/api/v1/settlements/{id}/confirm", tag = "spese co-living",
    params(("id" = Uuid, Path, description = "Identificatore del regolamento")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Confermato: da qui i saldi si muovono. **Conferma chi riceve**, non lo staff", body = SettlementResponse),
        (status = 403, description = "Il regolamento è indirizzato a qualcun altro", body = crate::error::ProblemDetails),
        (status = 404, description = "Regolamento non trovato", body = crate::error::ProblemDetails),
        (status = 409, description = "Il regolamento non è in `pending`", body = crate::error::ProblemDetails),
    )
)]
async fn confirm_settlement(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SettlementResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let settlement = state
        .confirm_settlement
        .execute(&context, SettlementId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(SettlementResponse::from_domain(&settlement)))
}

#[utoipa::path(
    post, path = "/api/v1/settlements/{id}/reject", tag = "spese co-living",
    params(("id" = Uuid, Path, description = "Identificatore del regolamento")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Rifiutato: il denaro non è arrivato. I saldi restano dov'erano e `settled_at` viene azzerato", body = SettlementResponse),
        (status = 403, description = "Il regolamento è indirizzato a qualcun altro", body = crate::error::ProblemDetails),
        (status = 404, description = "Regolamento non trovato", body = crate::error::ProblemDetails),
        (status = 409, description = "Il regolamento non è in `pending`", body = crate::error::ProblemDetails),
    )
)]
async fn reject_settlement(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SettlementResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let settlement = state
        .reject_settlement
        .execute(&context, SettlementId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(SettlementResponse::from_domain(&settlement)))
}

#[utoipa::path(
    post, path = "/api/v1/settlements/{id}/cancel", tag = "spese co-living",
    params(("id" = Uuid, Path, description = "Identificatore del regolamento")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Ritirato da chi lo aveva dichiarato. **Fra pari il ritiro è lecito**, a differenza dei pagamenti verso il proprietario", body = SettlementResponse),
        (status = 403, description = "Il regolamento è stato dichiarato da qualcun altro: a lui il ritiro, a chi riceve il rifiuto", body = crate::error::ProblemDetails),
        (status = 404, description = "Regolamento non trovato", body = crate::error::ProblemDetails),
        (status = 409, description = "Il regolamento non è in `pending`", body = crate::error::ProblemDetails),
    )
)]
async fn cancel_settlement(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SettlementResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let settlement = state
        .cancel_settlement
        .execute(&context, SettlementId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(SettlementResponse::from_domain(&settlement)))
}
