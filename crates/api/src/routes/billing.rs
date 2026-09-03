use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use propflow_application::use_cases::ai::BillOcrStatus;
use propflow_application::use_cases::billing::{
    IssueChargeCommand, ListChargesQuery, ListUtilityBillsQuery, Recharge,
    RechargeUtilityBillCommand, RegisterUtilityBillCommand, UpdateUtilityBillCommand,
};
use propflow_domain::DomainError;
use propflow_domain::ai::{AiJob, AiJobStatus, ExtractedField, ExtractedValue, OcrReading};
use propflow_domain::billing::{
    BillStatus, Charge, ChargeBalance, ChargeStatus, ChargeType, Consumption, ConsumptionUnit,
    InvoiceNumber, SupplierName, SupplierVat, UtilityBill, UtilityBillChanges, UtilityType,
};
use propflow_domain::property::Patch;
use propflow_domain::shared::{ChargeId, LeaseId, PropertyId, RoomId, UtilityBillId};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::Authenticated;
use crate::routes::{amount, explicit_null, format_amount};
use crate::state::AppState;

/// Le transizioni sono **sotto-risorse e non un `PATCH` sullo stato**, come per le
/// segnalazioni e per i contratti.
///
/// Il riaddebito in particolare: `POST /utility-bills/{id}/recharge` crea
/// l'addebito, lo collega e porta la bolletta ad `allocated` in una transazione
/// sola. L'alternativa — accettare `utility_bill_id` nel corpo di
/// `POST /leases/{id}/charges` — nasconderebbe in una creazione un effetto su un
/// altro aggregato, e infatti quel corpo **non lo accetta**: c'è una via sola
/// perché una bolletta e un addebito si leghino.
///
/// **Sugli addebiti non c'è alcun `PATCH`.** Non è una dimenticanza: un addebito
/// emesso è una pretesa già comunicata a una persona, e la migration `0023` toglie
/// al ruolo applicativo il privilegio di riscriverne qualunque colonna che non sia
/// `status`. Prorogare una scadenza si fa annullando e riemettendo.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/properties/{property_id}/utility-bills",
            post(register_bill),
        )
        .route("/api/v1/utility-bills", get(list_bills))
        .route(
            "/api/v1/utility-bills/{id}",
            get(bill_detail).patch(update_bill),
        )
        .route("/api/v1/utility-bills/{id}/ocr", get(bill_ocr_status))
        .route("/api/v1/utility-bills/{id}/validate", post(validate_bill))
        .route("/api/v1/utility-bills/{id}/void", post(void_bill))
        .route("/api/v1/utility-bills/{id}/recharge", post(recharge_bill))
        .route("/api/v1/leases/{lease_id}/charges", post(issue_charge))
        .route("/api/v1/charges", get(list_charges))
        .route("/api/v1/charges/{id}", get(charge_detail))
        .route("/api/v1/charges/{id}/void", post(void_charge))
        .route("/api/v1/charges/{id}/write-off", post(write_off_charge))
}

// ---------------------------------------------------------------------------
// Rappresentazioni di trasporto
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UtilityTypeDto {
    Electricity,
    Gas,
    Water,
    Heating,
    Waste,
    Internet,
    CondoFees,
    Other,
}

impl From<UtilityTypeDto> for UtilityType {
    fn from(value: UtilityTypeDto) -> Self {
        match value {
            UtilityTypeDto::Electricity => Self::Electricity,
            UtilityTypeDto::Gas => Self::Gas,
            UtilityTypeDto::Water => Self::Water,
            UtilityTypeDto::Heating => Self::Heating,
            UtilityTypeDto::Waste => Self::Waste,
            UtilityTypeDto::Internet => Self::Internet,
            UtilityTypeDto::CondoFees => Self::CondoFees,
            UtilityTypeDto::Other => Self::Other,
        }
    }
}

impl From<UtilityType> for UtilityTypeDto {
    fn from(value: UtilityType) -> Self {
        match value {
            UtilityType::Electricity => Self::Electricity,
            UtilityType::Gas => Self::Gas,
            UtilityType::Water => Self::Water,
            UtilityType::Heating => Self::Heating,
            UtilityType::Waste => Self::Waste,
            UtilityType::Internet => Self::Internet,
            UtilityType::CondoFees => Self::CondoFees,
            UtilityType::Other => Self::Other,
        }
    }
}

/// Stato di una bolletta sul filo.
///
/// Tre valori compaiono perché una riga letta può contenerli, ma **l'API non li
/// produce mai**: `pending_review` appartiene all'OCR, `paid` al pagamento della
/// bolletta al fornitore — che non è modellato affatto — e `disputed` alla
/// contestazione, che non ha una via di ritorno disegnata.
#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BillStatusDto {
    Draft,
    /// Mai scritto dall'API.
    PendingReview,
    Validated,
    Allocated,
    /// Mai scritto dall'API.
    Paid,
    /// Mai scritto dall'API.
    Disputed,
    Void,
}

impl From<BillStatusDto> for BillStatus {
    fn from(value: BillStatusDto) -> Self {
        match value {
            BillStatusDto::Draft => Self::Draft,
            BillStatusDto::PendingReview => Self::PendingReview,
            BillStatusDto::Validated => Self::Validated,
            BillStatusDto::Allocated => Self::Allocated,
            BillStatusDto::Paid => Self::Paid,
            BillStatusDto::Disputed => Self::Disputed,
            BillStatusDto::Void => Self::Void,
        }
    }
}

impl From<BillStatus> for BillStatusDto {
    fn from(value: BillStatus) -> Self {
        match value {
            BillStatus::Draft => Self::Draft,
            BillStatus::PendingReview => Self::PendingReview,
            BillStatus::Validated => Self::Validated,
            BillStatus::Allocated => Self::Allocated,
            BillStatus::Paid => Self::Paid,
            BillStatus::Disputed => Self::Disputed,
            BillStatus::Void => Self::Void,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChargeTypeDto {
    Rent,
    Deposit,
    UtilityRecharge,
    LateFee,
    Maintenance,
    Adjustment,
    Other,
}

impl From<ChargeTypeDto> for ChargeType {
    fn from(value: ChargeTypeDto) -> Self {
        match value {
            ChargeTypeDto::Rent => Self::Rent,
            ChargeTypeDto::Deposit => Self::Deposit,
            ChargeTypeDto::UtilityRecharge => Self::UtilityRecharge,
            ChargeTypeDto::LateFee => Self::LateFee,
            ChargeTypeDto::Maintenance => Self::Maintenance,
            ChargeTypeDto::Adjustment => Self::Adjustment,
            ChargeTypeDto::Other => Self::Other,
        }
    }
}

impl From<ChargeType> for ChargeTypeDto {
    fn from(value: ChargeType) -> Self {
        match value {
            ChargeType::Rent => Self::Rent,
            ChargeType::Deposit => Self::Deposit,
            ChargeType::UtilityRecharge => Self::UtilityRecharge,
            ChargeType::LateFee => Self::LateFee,
            ChargeType::Maintenance => Self::Maintenance,
            ChargeType::Adjustment => Self::Adjustment,
            ChargeType::Other => Self::Other,
        }
    }
}

/// Stato di un addebito sul filo.
///
/// **`overdue` non è mai memorizzato**: compare in `effective_status`, che lo
/// deriva dalla scadenza. Filtrare l'elenco con `?status=overdue` dà `422`
/// indicando `?outstanding=true`, perché quel filtro darebbe sempre zero righe.
///
/// `draft` non è mai scritto — un addebito nasce emesso — e `partially_paid` e
/// `paid` li produrrà il modulo dei pagamenti.
#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChargeStatusDto {
    /// Mai scritto dall'API: un addebito nasce emesso.
    Draft,
    Issued,
    /// Prodotto dal modulo dei pagamenti, non da questa fetta.
    PartiallyPaid,
    /// Prodotto dal modulo dei pagamenti, non da questa fetta.
    Paid,
    /// **Solo in `effective_status`**, mai memorizzato.
    Overdue,
    Void,
    WrittenOff,
}

impl From<ChargeStatusDto> for ChargeStatus {
    fn from(value: ChargeStatusDto) -> Self {
        match value {
            ChargeStatusDto::Draft => Self::Draft,
            ChargeStatusDto::Issued => Self::Issued,
            ChargeStatusDto::PartiallyPaid => Self::PartiallyPaid,
            ChargeStatusDto::Paid => Self::Paid,
            ChargeStatusDto::Overdue => Self::Overdue,
            ChargeStatusDto::Void => Self::Void,
            ChargeStatusDto::WrittenOff => Self::WrittenOff,
        }
    }
}

impl From<ChargeStatus> for ChargeStatusDto {
    fn from(value: ChargeStatus) -> Self {
        match value {
            ChargeStatus::Draft => Self::Draft,
            ChargeStatus::Issued => Self::Issued,
            ChargeStatus::PartiallyPaid => Self::PartiallyPaid,
            ChargeStatus::Paid => Self::Paid,
            ChargeStatus::Overdue => Self::Overdue,
            ChargeStatus::Void => Self::Void,
            ChargeStatus::WrittenOff => Self::WrittenOff,
        }
    }
}

/// Consumo rilevato, con la sua unità.
///
/// I due campi viaggiano **insieme** e non separatamente, benché a schema siano due
/// colonne indipendenti: `123.4567` senza unità non dice niente. Il valore ammette
/// **quattro** decimali, a differenza degli importi che ne ammettono due — la
/// colonna è `NUMERIC(14,4)`.
#[derive(Debug, Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct ConsumptionDto {
    /// Stringa decimale con al più quattro decimali (`"123.4567"`).
    #[schema(example = "123.4567")]
    pub value: String,
    #[schema(example = "Smc")]
    pub unit: String,
}

impl ConsumptionDto {
    fn parse(&self) -> Result<Consumption, DomainError> {
        Consumption::new(
            amount("consumption", &self.value)?,
            ConsumptionUnit::parse(&self.unit)?,
        )
    }

    fn from_domain(consumption: &Consumption) -> Self {
        Self {
            value: consumption.value().normalize().to_string(),
            unit: consumption.unit().as_str().to_owned(),
        }
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RegisterUtilityBillRequest {
    /// La stanza a cui l'utenza si riferisce. Deve appartenere all'immobile del
    /// percorso: lo schema non lo verifica, quindi lo verifica il service layer.
    #[serde(default)]
    pub room_id: Option<Uuid>,
    /// Il contratto a cui la bolletta è imputata.
    ///
    /// **Da valorizzare perché un inquilino possa vederla**: la policy mostra la
    /// bolletta allo staff oppure a chi è parte di questo contratto. Senza, la vede
    /// il solo staff.
    #[serde(default)]
    pub lease_id: Option<Uuid>,
    pub utility_type: UtilityTypeDto,
    #[serde(default)]
    pub supplier_name: Option<String>,
    /// Normalizzata in maiuscolo e senza separatori: è metà della chiave di
    /// deduplica, e `it 123` e `IT123` sarebbero due fatture diverse per l'indice.
    #[serde(default)]
    #[schema(example = "IT12345678901")]
    pub supplier_vat: Option<String>,
    #[serde(default)]
    #[schema(example = "2026/A/00417")]
    pub invoice_number: Option<String>,
    #[serde(default)]
    pub issue_date: Option<NaiveDate>,
    #[serde(default)]
    pub due_date: Option<NaiveDate>,
    #[serde(default)]
    pub period_start: Option<NaiveDate>,
    #[serde(default)]
    pub period_end: Option<NaiveDate>,
    /// Stringa decimale (`"184.20"`).
    #[schema(example = "184.20")]
    pub total_amount: String,
    /// Imponibile e IVA **non** devono sommare al totale, e nessuno lo verifica:
    /// una bolletta di utenza italiana porta abitualmente più aliquote, quote fisse
    /// e voci non imponibili.
    #[serde(default)]
    pub taxable_amount: Option<String>,
    #[serde(default)]
    pub vat_amount: Option<String>,
    #[serde(default)]
    pub consumption: Option<ConsumptionDto>,
    #[serde(default)]
    pub notes: Option<String>,
}

impl RegisterUtilityBillRequest {
    fn into_command(
        self,
        property_id: PropertyId,
    ) -> Result<RegisterUtilityBillCommand, DomainError> {
        Ok(RegisterUtilityBillCommand {
            property_id,
            room_id: self.room_id.map(RoomId::from_uuid),
            lease_id: self.lease_id.map(LeaseId::from_uuid),
            utility_type: self.utility_type.into(),
            supplier_name: self
                .supplier_name
                .as_deref()
                .map(SupplierName::parse)
                .transpose()?,
            supplier_vat: self
                .supplier_vat
                .as_deref()
                .map(SupplierVat::parse)
                .transpose()?,
            invoice_number: self
                .invoice_number
                .as_deref()
                .map(InvoiceNumber::parse)
                .transpose()?,
            issue_date: self.issue_date,
            due_date: self.due_date,
            period_start: self.period_start,
            period_end: self.period_end,
            total_amount: amount("total_amount", &self.total_amount)?,
            taxable_amount: self
                .taxable_amount
                .as_deref()
                .map(|raw| amount("taxable_amount", raw))
                .transpose()?,
            vat_amount: self
                .vat_amount
                .as_deref()
                .map(|raw| amount("vat_amount", raw))
                .transpose()?,
            consumption: self
                .consumption
                .as_ref()
                .map(ConsumptionDto::parse)
                .transpose()?,
            notes: self.notes,
        })
    }
}

/// Correzione parziale di una bolletta.
///
/// Ammessa da `draft` e da `validated`. **Su una bolletta validata la correzione
/// disfa la validazione**: lo stato torna a `draft` e chi la valida di nuovo deve
/// rifarlo apposta. Validare è l'affermazione «questi numeri sono quelli», e
/// cambiare i numeri la smentisce.
///
/// Da `allocated` in poi la bolletta è ferma e la richiesta dà `409`: quei numeri
/// sono la base di un addebito già emesso a qualcuno.
///
/// `property_id` non compare: è l'ancoraggio strutturale della riga, e una bolletta
/// inserita sull'immobile sbagliato si annulla e si reinserisce — che è possibile
/// proprio perché annullarla libera la chiave di deduplica.
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
pub struct UpdateUtilityBillRequest {
    #[serde(default)]
    pub utility_type: Option<UtilityTypeDto>,
    /// `null` azzera il valore.
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub room_id: Patch<Uuid>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub lease_id: Patch<Uuid>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub supplier_name: Patch<String>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub supplier_vat: Patch<String>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub invoice_number: Patch<String>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub issue_date: Patch<NaiveDate>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub due_date: Patch<NaiveDate>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub period_start: Patch<NaiveDate>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub period_end: Patch<NaiveDate>,
    /// Non azzerabile: a schema è `NOT NULL`.
    #[serde(default)]
    pub total_amount: Option<String>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub taxable_amount: Patch<String>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub vat_amount: Patch<String>,
    /// Azzerarlo azzera **entrambe** le colonne, valore e unità.
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<ConsumptionDto>, nullable)]
    pub consumption: Patch<ConsumptionDto>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub notes: Patch<String>,
}

impl UpdateUtilityBillRequest {
    fn into_command(self) -> Result<UpdateUtilityBillCommand, DomainError> {
        Ok(UtilityBillChanges {
            utility_type: self.utility_type.map(Into::into),
            room_id: self.room_id.map(|value| value.map(RoomId::from_uuid)),
            lease_id: self.lease_id.map(|value| value.map(LeaseId::from_uuid)),
            supplier_name: patch_parse(self.supplier_name.as_ref(), |raw| {
                SupplierName::parse(raw)
            })?,
            supplier_vat: patch_parse(self.supplier_vat.as_ref(), |raw| SupplierVat::parse(raw))?,
            invoice_number: patch_parse(self.invoice_number.as_ref(), |raw| {
                InvoiceNumber::parse(raw)
            })?,
            issue_date: self.issue_date,
            due_date: self.due_date,
            period_start: self.period_start,
            period_end: self.period_end,
            total_amount: self
                .total_amount
                .as_deref()
                .map(|raw| amount("total_amount", raw))
                .transpose()?,
            taxable_amount: patch_parse(self.taxable_amount.as_ref(), |raw| {
                amount("taxable_amount", raw)
            })?,
            vat_amount: patch_parse(self.vat_amount.as_ref(), |raw| amount("vat_amount", raw))?,
            consumption: match self.consumption.as_ref() {
                None => None,
                Some(None) => Some(None),
                Some(Some(dto)) => Some(Some(dto.parse()?)),
            },
            notes: self.notes,
        })
    }
}

/// Applica una conversione fallibile dentro un [`Patch`], preservando le tre
/// semantiche: non menzionato, azzerato, impostato.
fn patch_parse<T, U, F>(patch: Option<&Option<T>>, parse: F) -> Result<Patch<U>, DomainError>
where
    F: FnOnce(&T) -> Result<U, DomainError>,
{
    match patch {
        None => Ok(None),
        Some(None) => Ok(Some(None)),
        Some(Some(value)) => Ok(Some(Some(parse(value)?))),
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct IssueChargeRequest {
    pub charge_type: ChargeTypeDto,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub period_start: Option<NaiveDate>,
    #[serde(default)]
    pub period_end: Option<NaiveDate>,
    /// Stringa decimale (`"800.00"`).
    #[schema(example = "800.00")]
    pub amount: String,
    /// Obbligatoria: un addebito senza scadenza non è esigibile e non entrerebbe
    /// mai in nessuno scadenzario.
    pub due_date: NaiveDate,
    #[serde(default)]
    pub notes: Option<String>,
}

impl IssueChargeRequest {
    fn into_command(self, lease_id: LeaseId) -> Result<IssueChargeCommand, DomainError> {
        Ok(IssueChargeCommand {
            lease_id,
            charge_type: self.charge_type.into(),
            description: self.description,
            period_start: self.period_start,
            period_end: self.period_end,
            amount: amount("amount", &self.amount)?,
            due_date: self.due_date,
            notes: self.notes,
        })
    }
}

/// Corpo del riaddebito.
///
/// **Non c'è `charge_type`**: un addebito nato da una bolletta *è* un riaddebito di
/// utenza, e lasciarlo scegliere permetterebbe di emettere un canone che cita una
/// bolletta. Non c'è nemmeno il periodo: quello dell'addebito è quello della
/// bolletta, perché è ciò che l'inquilino sta pagando.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RechargeUtilityBillRequest {
    /// Il contratto a cui addebitare la quota.
    ///
    /// **Non deve coincidere con il contratto a cui la bolletta è imputata**:
    /// quello dice chi la vede, questo a chi si chiede il denaro. In un co-living
    /// con contratti per stanza sono legittimamente diversi.
    pub lease_id: Uuid,
    /// La quota, che **non** è per forza il totale della bolletta.
    ///
    /// Riaddebitare la stessa bolletta a più contratti è una ripetizione
    /// dell'operazione. **Nessuno verifica che le parti sommino al totale**: lo
    /// schema non ha un vincolo e un controllo applicativo sarebbe soggetto a race
    /// condition — la ripartizione con garanzia di somma è il modulo delle spese
    /// co-living.
    #[schema(example = "120.00")]
    pub amount: String,
    pub due_date: NaiveDate,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

impl RechargeUtilityBillRequest {
    fn into_command(self) -> Result<RechargeUtilityBillCommand, DomainError> {
        Ok(RechargeUtilityBillCommand {
            lease_id: LeaseId::from_uuid(self.lease_id),
            amount: amount("amount", &self.amount)?,
            due_date: self.due_date,
            description: self.description,
            notes: self.notes,
        })
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct UtilityBillResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub property_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease_id: Option<Uuid>,
    pub utility_type: UtilityTypeDto,
    pub status: BillStatusDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supplier_vat: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_number: Option<String>,
    pub issue_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    /// Stringhe decimali, come tutti gli importi dell'API.
    pub total_amount: String,
    pub taxable_amount: Option<String>,
    pub vat_amount: Option<String>,
    /// Sempre `EUR` in questa fetta: la valuta non è dichiarabile finché le viste
    /// dei saldi sommano importi senza confrontarla.
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumption: Option<ConsumptionDto>,
    /// Chi ha controllato i numeri, e quando. Una correzione successiva li azzera
    /// entrambi: validare è un'affermazione sui numeri, e cambiarli la smentisce.
    pub validated_by: Option<Uuid>,
    pub validated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl UtilityBillResponse {
    fn from_domain(bill: &UtilityBill) -> Self {
        Self {
            id: bill.id().as_uuid(),
            organization_id: bill.org_id().as_uuid(),
            property_id: bill.property_id().as_uuid(),
            room_id: bill.room_id().map(|id| id.as_uuid()),
            lease_id: bill.lease_id().map(|id| id.as_uuid()),
            utility_type: bill.utility_type().into(),
            status: bill.status().into(),
            supplier_name: bill.supplier_name().map(|name| name.as_str().to_owned()),
            supplier_vat: bill.supplier_vat().map(|vat| vat.as_str().to_owned()),
            invoice_number: bill
                .invoice_number()
                .map(|number| number.as_str().to_owned()),
            issue_date: bill.issue_date(),
            due_date: bill.due_date(),
            period_start: bill.period_start(),
            period_end: bill.period_end(),
            total_amount: format_amount(bill.total_amount().amount()),
            taxable_amount: bill
                .taxable_amount()
                .map(|money| format_amount(money.amount())),
            vat_amount: bill.vat_amount().map(|money| format_amount(money.amount())),
            currency: bill.currency().as_str().to_owned(),
            consumption: bill.consumption().map(ConsumptionDto::from_domain),
            validated_by: bill.validated_by().map(|id| id.as_uuid()),
            validated_at: bill.validated_at(),
            notes: bill.notes().map(str::to_owned),
            created_at: bill.created_at(),
            updated_at: bill.updated_at(),
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct UtilityBillPageResponse {
    pub items: Vec<UtilityBillResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Uuid>,
}

/// Addebito con il suo saldo.
///
/// **In questa fetta `paid_amount` vale sempre `"0.00"` e `outstanding_amount`
/// coincide sempre con `amount`.** Non è un difetto del calcolo: è l'assenza di
/// righe in `app.payments`, che è il modulo successivo. Il saldo si legge da
/// `app.v_charge_balances` e non si ricalcola qui.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ChargeResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub lease_id: Uuid,
    pub charge_type: ChargeTypeDto,
    /// Lo stato **memorizzato**, che cambia solo su azioni esplicite.
    pub status: ChargeStatusDto,
    /// Lo stato **derivato**: vale `overdue` quando la scadenza è passata e il
    /// residuo è positivo, mentre `status` resta `issued`. Nessuno scrive mai
    /// `overdue` in database — è la stessa regola di `expiring` sui contratti
    /// (ADR-019), e serve a non avere uno stato che invecchia da solo.
    pub effective_status: ChargeStatusDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub amount: String,
    /// Quanto è stato incassato. **Sempre `"0.00"` finché non esiste il modulo dei
    /// pagamenti.**
    pub paid_amount: String,
    /// Il residuo: `amount - paid_amount`, letto dalla vista.
    pub outstanding_amount: String,
    pub currency: String,
    pub due_date: NaiveDate,
    pub issued_at: Option<chrono::DateTime<chrono::Utc>>,
    /// La bolletta da cui l'addebito nasce, se è un riaddebito.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utility_bill_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ChargeResponse {
    /// Da un addebito **senza** saldo: è ciò che restituiscono l'emissione e le
    /// transizioni, che non pagano il join con la vista.
    ///
    /// `paid_amount` è zero e `outstanding_amount` è l'intero importo, il che in
    /// questa fetta coincide con ciò che la vista direbbe — non ci sono pagamenti.
    /// `effective_status` si calcola qui con la stessa formula della vista.
    fn from_charge(charge: &Charge, today: NaiveDate) -> Self {
        let mut response = Self::project(charge);
        response.paid_amount = format_amount(Decimal::ZERO);
        response.outstanding_amount = format_amount(charge.amount().amount());
        response.effective_status = charge.effective_status(today, Decimal::ZERO).into();
        response
    }

    /// Da un addebito **con** il saldo letto dalla vista.
    ///
    /// `effective_status` usa `is_past_due` della vista e non ricalcola la data: se
    /// le due definizioni divergessero per un fuso orario, vince quella che ha
    /// prodotto il numero mostrato accanto.
    fn from_balance(balance: &ChargeBalance) -> Self {
        let mut response = Self::project(&balance.charge);
        response.paid_amount = format_amount(balance.paid_amount.amount());
        response.outstanding_amount = format_amount(balance.outstanding_amount.amount());
        response.effective_status = balance.effective_status().into();
        response
    }

    fn project(charge: &Charge) -> Self {
        Self {
            id: charge.id().as_uuid(),
            organization_id: charge.org_id().as_uuid(),
            lease_id: charge.lease_id().as_uuid(),
            charge_type: charge.charge_type().into(),
            status: charge.status().into(),
            effective_status: charge.status().into(),
            description: charge.description().map(str::to_owned),
            period_start: charge.period_start(),
            period_end: charge.period_end(),
            amount: format_amount(charge.amount().amount()),
            paid_amount: format_amount(Decimal::ZERO),
            outstanding_amount: format_amount(charge.amount().amount()),
            currency: charge.currency().as_str().to_owned(),
            due_date: charge.due_date(),
            issued_at: charge.issued_at(),
            utility_bill_id: charge.utility_bill_id().map(|id| id.as_uuid()),
            notes: charge.notes().map(str::to_owned),
            created_at: charge.created_at(),
            updated_at: charge.updated_at(),
        }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ChargePageResponse {
    pub items: Vec<ChargeResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Uuid>,
}

/// Esito del riaddebito: entrambi gli aggregati che la transazione ha toccato.
///
/// Tornano tutti e due perché la stessa richiesta li cambia entrambi — la bolletta
/// passa ad `allocated` e l'addebito nasce — e un client che ricevesse solo
/// l'addebito dovrebbe rileggere la bolletta per sapere in che stato è finita.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct RechargeResponse {
    pub charge: ChargeResponse,
    pub utility_bill: UtilityBillResponse,
}

/// Stato di un job di elaborazione automatica.
///
/// **Cinque valori e l'applicazione ne scrive quattro.** `cancelled` esiste
/// nell'enum del database e nessuno lo produce: un job si chiude in millisecondi e
/// non c'è nulla da annullare finché non esiste un fornitore vero che impieghi
/// secondi. Resta nel DTO perché una riga scritta a mano potrebbe contenerlo.
#[derive(Debug, Clone, Copy, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiJobStatusDto {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl From<AiJobStatus> for AiJobStatusDto {
    fn from(value: AiJobStatus) -> Self {
        match value {
            AiJobStatus::Queued => Self::Queued,
            AiJobStatus::Running => Self::Running,
            AiJobStatus::Succeeded => Self::Succeeded,
            AiJobStatus::Failed => Self::Failed,
            AiJobStatus::Cancelled => Self::Cancelled,
        }
    }
}

/// Un campo che la macchina ha proposto.
///
/// `value` è **sempre una stringa**, qualunque sia il tipo del campo, per la
/// stessa ragione per cui gli importi lo sono ovunque in questa API: un decimale
/// che passa da un numero JSON passa da un float binario, e `123.4567` che torna
/// `123.45670000000001` è un valore che una persona dovrebbe correggere a mano
/// proprio nel campo che la macchina aveva letto giusto. `type` dice come
/// interpretarla; `unit` c'è solo per `quantity`.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ExtractedFieldResponse {
    /// Il nome della colonna della bolletta a cui la proposta si riferisce.
    pub field: String,
    /// `text`, `date`, `amount` o `quantity`.
    #[serde(rename = "type")]
    pub value_type: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Fra `"0"` e `"1"`, quattro decimali. **Bassa non vuol dire sbagliata**:
    /// vuol dire che la macchina non si fida, ed è il campo da guardare per primo.
    pub confidence: String,
}

impl ExtractedFieldResponse {
    fn from_domain(found: &ExtractedField) -> Self {
        let (value_type, value, unit) = match &found.value {
            ExtractedValue::Text(text) => ("text", text.clone(), None),
            ExtractedValue::Date(date) => ("date", date.to_string(), None),
            ExtractedValue::Amount(amount) => ("amount", amount.to_string(), None),
            ExtractedValue::Quantity { value, unit } => {
                ("quantity", value.to_string(), Some(unit.clone()))
            }
        };
        Self {
            field: found.field.as_str().to_owned(),
            value_type: value_type.to_owned(),
            value,
            unit,
            confidence: found.confidence.to_string(),
        }
    }
}

/// Il job di lettura automatica di una bolletta.
///
/// **Non porta `output`**, cioè la forma salvata sul job: sarebbe la stessa
/// informazione di `extracted` senza i valori, e due rappresentazioni della stessa
/// cosa nella stessa risposta si contraddicono al primo caso limite. La forma
/// serve a interrogare `ai_jobs` per misurare l'accuratezza, non a un client.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct AiJobResponse {
    pub id: Uuid,
    pub status: AiJobStatusDto,
    pub task: String,
    /// Il fornitore a cui il job è destinato, come **dato**: qui non c'è un ramo
    /// del codice per fornitore.
    pub provider_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Quante volte il job è stato preso in carico. Sale alla presa in carico e
    /// non alla chiusura, così un processo morto a metà lascia traccia di averci
    /// provato.
    pub attempts: u16,
    /// Perché è fallito, se è fallito. **Non è un codice di errore HTTP**: un job
    /// fallito è un esito riuscito della richiesta che lo ha accodato.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl AiJobResponse {
    fn from_domain(job: &AiJob) -> Self {
        Self {
            id: job.id().as_uuid(),
            status: job.status().into(),
            task: job.task().as_str().to_owned(),
            provider_name: job.provider_name().to_owned(),
            model: job.model().map(str::to_owned),
            attempts: job.attempts(),
            error_code: job
                .failure()
                .map(|failure| failure.code.as_str().to_owned()),
            error_message: job.failure().map(|failure| failure.message.clone()),
            started_at: job.started_at(),
            finished_at: job.finished_at(),
            created_at: job.created_at(),
        }
    }
}

/// A che punto è la lettura automatica di una bolletta, con la proposta.
///
/// **Tutti e tre i campi sono `null` finché nessuno ha caricato una scansione**, e
/// non è un `404`: la bolletta esiste, semplicemente nessuna macchina l'ha ancora
/// letta.
///
/// I valori qui dentro **non sono quelli della bolletta**: sono ciò che la
/// macchina propone, e la bolletta continua a portare quelli che ha digitato una
/// persona. È il confine per cui l'accuratezza dell'OCR si potrà misurare, e un
/// client che li scrivesse sopra i campi della bolletta senza chiedere lo
/// distruggerebbe.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct BillOcrStatusResponse {
    pub utility_bill_id: Uuid,
    /// Lo stato della bolletta, ripetuto qui perché è la domanda successiva di
    /// chi legge questa risposta: `pending_review` significa «c'è una proposta da
    /// rivedere».
    pub status: BillStatusDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job: Option<AiJobResponse>,
    /// La confidenza complessiva: il **minimo** fra quelle dei campi, non la
    /// media. Dieci campi letti bene non devono nascondere il totale letto male.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
    /// I campi proposti. `null` finché nessuna lettura è riuscita; un array
    /// **vuoto** significa che la macchina ha letto e non ha trovato niente di
    /// interpretabile, che è un esito diverso.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extracted: Option<Vec<ExtractedFieldResponse>>,
}

impl BillOcrStatusResponse {
    fn from_domain(status: &BillOcrStatus) -> Self {
        Self {
            utility_bill_id: status.bill.id().as_uuid(),
            status: status.bill.status().into(),
            job: status.job.as_ref().map(AiJobResponse::from_domain),
            confidence: status.bill.ocr_confidence().map(|value| value.to_string()),
            extracted: status.bill.ocr_extracted().map(|reading: &OcrReading| {
                reading
                    .fields()
                    .iter()
                    .map(ExtractedFieldResponse::from_domain)
                    .collect()
            }),
        }
    }
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListUtilityBillsParams {
    #[serde(default)]
    pub property_id: Option<Uuid>,
    #[serde(default)]
    pub lease_id: Option<Uuid>,
    #[serde(default)]
    pub status: Option<BillStatusDto>,
    #[serde(default)]
    pub utility_type: Option<UtilityTypeDto>,
    #[serde(default)]
    pub after: Option<Uuid>,
    #[serde(default)]
    pub limit: Option<u16>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListChargesParams {
    #[serde(default)]
    pub lease_id: Option<Uuid>,
    #[serde(default)]
    pub charge_type: Option<ChargeTypeDto>,
    /// **`overdue` dà `422`**: è derivato dalla scadenza e nessuna riga lo porta
    /// memorizzato, quindi il filtro darebbe sempre zero risultati. Usare
    /// `outstanding=true`.
    #[serde(default)]
    pub status: Option<ChargeStatusDto>,
    /// `true` = solo gli addebiti ancora da incassare, cioè lo scadenzario. È il
    /// filtro che usa l'indice parziale `charges_outstanding_ix`.
    #[serde(default)]
    pub outstanding: Option<bool>,
    #[serde(default)]
    pub after: Option<Uuid>,
    #[serde(default)]
    pub limit: Option<u16>,
}

// ---------------------------------------------------------------------------
// Handler: deserializzare, chiamare lo use case, serializzare. Nulla di più.
// ---------------------------------------------------------------------------

#[utoipa::path(
    post, path = "/api/v1/properties/{property_id}/utility-bills", tag = "bollette",
    params(("property_id" = Uuid, Path, description = "Identificatore dell'immobile")),
    request_body = RegisterUtilityBillRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Bolletta registrata, in stato `draft`", body = UtilityBillResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Immobile, stanza o contratto non trovati", body = crate::error::ProblemDetails),
        (status = 409, description = "Fattura già registrata per questo fornitore", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi, o riferimenti di un altro immobile", body = crate::error::ProblemDetails),
    )
)]
async fn register_bill(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(property_id): Path<Uuid>,
    Json(request): Json<RegisterUtilityBillRequest>,
) -> ApiResult<(StatusCode, Json<UtilityBillResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let command = request
        .into_command(PropertyId::from_uuid(property_id))
        .map_err(|error| {
            ApiError::from(propflow_application::AppError::Validation(error))
                .with_request_id(request_id.clone())
        })?;

    let bill = state
        .register_utility_bill
        .execute(&context, command)
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok((
        StatusCode::CREATED,
        Json(UtilityBillResponse::from_domain(&bill)),
    ))
}

#[utoipa::path(
    get, path = "/api/v1/utility-bills", tag = "bollette",
    params(ListUtilityBillsParams),
    security(("bearer" = [])),
    responses((status = 200, description = "Pagina di bollette", body = UtilityBillPageResponse))
)]
async fn list_bills(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Query(params): Query<ListUtilityBillsParams>,
) -> ApiResult<Json<UtilityBillPageResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let page = state
        .list_utility_bills
        .execute(
            &context,
            ListUtilityBillsQuery {
                property_id: params.property_id.map(PropertyId::from_uuid),
                lease_id: params.lease_id.map(LeaseId::from_uuid),
                status: params.status.map(Into::into),
                utility_type: params.utility_type.map(Into::into),
                after: params.after.map(UtilityBillId::from_uuid),
                limit: params.limit,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(UtilityBillPageResponse {
        items: page
            .items
            .iter()
            .map(UtilityBillResponse::from_domain)
            .collect(),
        next_cursor: page.next_cursor.map(|id| id.as_uuid()),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/utility-bills/{id}", tag = "bollette",
    params(("id" = Uuid, Path, description = "Identificatore della bolletta")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Bolletta", body = UtilityBillResponse),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
    )
)]
async fn bill_detail(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<UtilityBillResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let bill = state
        .get_utility_bill
        .execute(&context, UtilityBillId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(UtilityBillResponse::from_domain(&bill)))
}

#[utoipa::path(
    patch, path = "/api/v1/utility-bills/{id}", tag = "bollette",
    params(("id" = Uuid, Path, description = "Identificatore della bolletta")),
    request_body = UpdateUtilityBillRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Bolletta corretta. Se era validata torna in `draft`", body = UtilityBillResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Non trovata, o stanza/contratto inesistenti", body = crate::error::ProblemDetails),
        (status = 409, description = "Bolletta già riaddebitata o annullata", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi, o riferimenti di un altro immobile", body = crate::error::ProblemDetails),
    )
)]
async fn update_bill(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateUtilityBillRequest>,
) -> ApiResult<Json<UtilityBillResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let changes = request.into_command().map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;

    let bill = state
        .update_utility_bill
        .execute(&context, UtilityBillId::from_uuid(id), changes)
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(UtilityBillResponse::from_domain(&bill)))
}

#[utoipa::path(
    get, path = "/api/v1/utility-bills/{id}/ocr", tag = "bollette",
    params(("id" = Uuid, Path, description = "Identificatore della bolletta")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Stato della lettura automatica. `job` è `null` finché nessuno ha caricato una scansione", body = BillOcrStatusResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore: è la stessa condizione di `ai_jobs_select`", body = crate::error::ProblemDetails),
        (status = 404, description = "Bolletta non trovata o non visibile", body = crate::error::ProblemDetails),
    )
)]
async fn bill_ocr_status(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<BillOcrStatusResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let status = state
        .get_bill_ocr_status
        .execute(&context, UtilityBillId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(BillOcrStatusResponse::from_domain(&status)))
}

#[utoipa::path(
    post, path = "/api/v1/utility-bills/{id}/validate", tag = "bollette",
    params(("id" = Uuid, Path, description = "Identificatore della bolletta")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Bolletta validata", body = UtilityBillResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Solo una bozza si valida", body = crate::error::ProblemDetails),
    )
)]
async fn validate_bill(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<UtilityBillResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let bill = state
        .validate_utility_bill
        .execute(&context, UtilityBillId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(UtilityBillResponse::from_domain(&bill)))
}

#[utoipa::path(
    post, path = "/api/v1/utility-bills/{id}/void", tag = "bollette",
    params(("id" = Uuid, Path, description = "Identificatore della bolletta")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Bolletta annullata: **sparisce** dagli elenchi e il numero di fattura torna libero", body = UtilityBillResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Non trovata o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Già riaddebitata: annullare prima gli addebiti", body = crate::error::ProblemDetails),
    )
)]
async fn void_bill(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<UtilityBillResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let bill = state
        .void_utility_bill
        .execute(&context, UtilityBillId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(UtilityBillResponse::from_domain(&bill)))
}

#[utoipa::path(
    post, path = "/api/v1/utility-bills/{id}/recharge", tag = "bollette",
    params(("id" = Uuid, Path, description = "Identificatore della bolletta")),
    request_body = RechargeUtilityBillRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Addebito emesso e bolletta portata ad `allocated`", body = RechargeResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Bolletta o contratto non trovati", body = crate::error::ProblemDetails),
        (status = 409, description = "La bolletta non è validata", body = crate::error::ProblemDetails),
        (status = 422, description = "Importo non valido", body = crate::error::ProblemDetails),
    )
)]
async fn recharge_bill(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
    Json(request): Json<RechargeUtilityBillRequest>,
) -> ApiResult<(StatusCode, Json<RechargeResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let command = request.into_command().map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;

    let today = state.clock.now().date_naive();
    let Recharge { bill, charge } = state
        .recharge_utility_bill
        .execute(&context, UtilityBillId::from_uuid(id), command)
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok((
        StatusCode::CREATED,
        Json(RechargeResponse {
            charge: ChargeResponse::from_charge(&charge, today),
            utility_bill: UtilityBillResponse::from_domain(&bill),
        }),
    ))
}

#[utoipa::path(
    post, path = "/api/v1/leases/{lease_id}/charges", tag = "addebiti",
    params(("lease_id" = Uuid, Path, description = "Identificatore del contratto")),
    request_body = IssueChargeRequest,
    security(("bearer" = [])),
    responses(
        (status = 201, description = "Addebito emesso. Nasce `issued`: non esiste uno stato di bozza", body = ChargeResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Contratto non trovato o non visibile", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi", body = crate::error::ProblemDetails),
    )
)]
async fn issue_charge(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(lease_id): Path<Uuid>,
    Json(request): Json<IssueChargeRequest>,
) -> ApiResult<(StatusCode, Json<ChargeResponse>)> {
    let request_id = context.request_id().map(str::to_owned);
    let command = request
        .into_command(LeaseId::from_uuid(lease_id))
        .map_err(|error| {
            ApiError::from(propflow_application::AppError::Validation(error))
                .with_request_id(request_id.clone())
        })?;

    let today = state.clock.now().date_naive();
    let charge = state
        .issue_charge
        .execute(&context, command)
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok((
        StatusCode::CREATED,
        Json(ChargeResponse::from_charge(&charge, today)),
    ))
}

#[utoipa::path(
    get, path = "/api/v1/charges", tag = "addebiti",
    params(ListChargesParams),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Pagina di addebiti con il saldo", body = ChargePageResponse),
        (status = 422, description = "`status=overdue`: è derivato, usare `outstanding=true`", body = crate::error::ProblemDetails),
    )
)]
async fn list_charges(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Query(params): Query<ListChargesParams>,
) -> ApiResult<Json<ChargePageResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let page = state
        .list_charges
        .execute(
            &context,
            ListChargesQuery {
                lease_id: params.lease_id.map(LeaseId::from_uuid),
                charge_type: params.charge_type.map(Into::into),
                status: params.status.map(Into::into),
                outstanding: params.outstanding,
                after: params.after.map(ChargeId::from_uuid),
                limit: params.limit,
            },
        )
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(ChargePageResponse {
        items: page
            .items
            .iter()
            .map(ChargeResponse::from_balance)
            .collect(),
        next_cursor: page.next_cursor.map(|id| id.as_uuid()),
    }))
}

#[utoipa::path(
    get, path = "/api/v1/charges/{id}", tag = "addebiti",
    params(("id" = Uuid, Path, description = "Identificatore dell'addebito")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Addebito con il saldo", body = ChargeResponse),
        (status = 404, description = "Non trovato o non visibile", body = crate::error::ProblemDetails),
    )
)]
async fn charge_detail(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ChargeResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let balance = state
        .get_charge
        .execute(&context, ChargeId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(ChargeResponse::from_balance(&balance)))
}

#[utoipa::path(
    post, path = "/api/v1/charges/{id}/void", tag = "addebiti",
    params(("id" = Uuid, Path, description = "Identificatore dell'addebito")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Addebito annullato. **Resta visibile** a chi doveva pagarlo", body = ChargeResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Non trovato o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Solo un addebito emesso si annulla", body = crate::error::ProblemDetails),
    )
)]
async fn void_charge(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ChargeResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let today = state.clock.now().date_naive();
    let charge = state
        .void_charge
        .execute(&context, ChargeId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(ChargeResponse::from_charge(&charge, today)))
}

#[utoipa::path(
    post, path = "/api/v1/charges/{id}/write-off", tag = "addebiti",
    params(("id" = Uuid, Path, description = "Identificatore dell'addebito")),
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Incasso abbandonato. Diverso dall'annullamento: era dovuto, ed è una perdita", body = ChargeResponse),
        (status = 403, description = "Richiede ruolo proprietario o gestore", body = crate::error::ProblemDetails),
        (status = 404, description = "Non trovato o non visibile", body = crate::error::ProblemDetails),
        (status = 409, description = "Solo un addebito emesso si storna", body = crate::error::ProblemDetails),
    )
)]
async fn write_off_charge(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ChargeResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let today = state.clock.now().date_naive();
    let charge = state
        .write_off_charge
        .execute(&context, ChargeId::from_uuid(id))
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(ChargeResponse::from_charge(&charge, today)))
}
