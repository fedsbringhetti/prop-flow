use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use propflow_domain::reporting::OwnerDashboard;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::Authenticated;
use crate::routes::format_amount;
use crate::state::AppState;

/// La sola rotta del modulo, e la sola che attraversi tutti gli altri.
///
/// **Non prende parametri.** L'organizzazione è quella attiva — quella del token,
/// o quella dichiarata con `X-Org-Id` — perché una dashboard con l'organizzazione
/// nel percorso inviterebbe a chiederne una di cui non si è membri, e la risposta
/// a quella domanda sarebbe comunque `403`.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/dashboard", get(dashboard))
}

/// I nove numeri, così come la vista li produce.
///
/// I tre importi sono **stringhe decimali** come ogni altro importo dell'API: un
/// numero JSON passa da un float binario, e `184.20` che torna
/// `184.19999999999999` è un errore che si accumula in contabilità. I sei conteggi
/// sono interi, perché contare non ha decimali.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct DashboardResponse {
    pub organization_id: Uuid,
    /// Immobili non archiviati, bozze comprese.
    pub properties_count: i64,
    /// Contratti con lo stato memorizzato `active`.
    pub active_leases: i64,
    /// Somma dei canoni dei contratti attivi.
    ///
    /// **È un'attesa dichiarata, non un incasso previsto**: lo stato memorizzato
    /// resta `active` anche dopo la scadenza, perché nessuno scheduler lo riscrive
    /// (ADR-019). `leases_expiring_90d` sta accanto per questo.
    #[schema(example = "2400.00")]
    pub monthly_rent_expected: String,
    /// Residuo dovuto, sommato dai soli addebiti con residuo positivo.
    ///
    /// Un credito da sovrapagamento — `outstanding_amount` negativo — **non
    /// compensa** e non compare: la vista filtra `> 0`.
    #[schema(example = "1284.20")]
    pub total_outstanding: String,
    /// Addebiti scaduti con residuo, secondo il `CURRENT_DATE` del database.
    pub overdue_charges: i64,
    /// Segnalazioni nei sei stati non terminali.
    pub open_tickets: i64,
    /// Contratti in scadenza entro novanta giorni.
    pub leases_expiring_90d: i64,
    /// Bollette in `draft` **oppure** in `pending_review`: una registrata e senza
    /// scansione, e una che la macchina ha letto e che aspetta una persona.
    pub bills_pending_review: i64,
    /// Quanto si è rinunciato a incassare: la somma dei **residui** degli addebiti
    /// `written_off`. Dalla migration `0032`.
    ///
    /// **È il residuo, non l'importo.** Un addebito da `"800.00"` con `"300.00"`
    /// incassati e poi stornato contribuisce `"500.00"`: gli altri trecento sono
    /// entrati e non si sono persi.
    ///
    /// Sta accanto a `total_outstanding` e **non lo compensa**: quello dice quanto
    /// è ancora dovuto, questo quanto non lo sarà mai. Un addebito `void` non
    /// compare qui — non doveva essere chiesto, quindi non è una perdita.
    #[schema(example = "500.00")]
    pub written_off_total: String,
}

impl DashboardResponse {
    fn from_domain(dashboard: &OwnerDashboard) -> Self {
        Self {
            organization_id: dashboard.org_id.as_uuid(),
            properties_count: dashboard.properties_count,
            active_leases: dashboard.active_leases,
            monthly_rent_expected: format_amount(dashboard.monthly_rent_expected),
            total_outstanding: format_amount(dashboard.total_outstanding),
            overdue_charges: dashboard.overdue_charges,
            open_tickets: dashboard.open_tickets,
            leases_expiring_90d: dashboard.leases_expiring_90d,
            bills_pending_review: dashboard.bills_pending_review,
            written_off_total: format_amount(dashboard.written_off_total),
        }
    }
}

#[utoipa::path(
    get, path = "/api/v1/dashboard", tag = "dashboard",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "I nove numeri dell'organizzazione attiva", body = DashboardResponse),
        (status = 401, description = "Autenticazione richiesta", body = crate::error::ProblemDetails),
        (status = 403, description = "Richiede ruolo proprietario o gestore, oppure l'organizzazione attiva non è del chiamante", body = crate::error::ProblemDetails),
    )
)]
async fn dashboard(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
) -> ApiResult<Json<DashboardResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let dashboard = state
        .owner_dashboard
        .execute(&context)
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(DashboardResponse::from_domain(&dashboard)))
}
