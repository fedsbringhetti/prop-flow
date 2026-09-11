use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use propflow_application::use_cases::account::UpdateMyProfileCommand;
use propflow_domain::DomainError;
use propflow_domain::identity::{User, UserProfileChanges};
use propflow_domain::property::Patch;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::Authenticated;
use crate::routes::explicit_null;
use crate::state::AppState;

/// Il proprio account, e **mai un identificatore nel percorso**.
///
/// È la forma che tiene chiusa la domanda di
/// [ADR-036](../../../../../docs/08-decisioni.md): nessuna rotta proietta un utente
/// per identificatore. Un `PATCH /users/{id}` che accettasse solo il proprio
/// sarebbe la stessa cosa con una guardia applicativa davanti — cioè la porta
/// aperta e un cartello — e la prima persona a rimuovere quella guardia
/// «uniformando» le rotte non troverebbe niente che glielo impedisca.
///
/// **Non prende `X-Org-Id` e risponde senza**, come `/organizations`: i propri dati
/// non appartengono a nessuna organizzazione, e pretenderne una attiva romperebbe
/// il caso di chi è appena uscito dall'unica in cui abitava.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/me", get(read).patch(update))
}

/// I propri dati.
///
/// **Non porta `platform_role`, né il contatore dei tentativi falliti, né il
/// blocco.** Non sono affare di chi guarda: il ruolo di piattaforma è una
/// proprietà del sistema e non dell'account, e i due contatori sono una difesa —
/// mostrarli direbbe a chi ha rubato una scheda quanto gli manca per bloccare
/// fuori il proprietario.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct MeResponse {
    pub id: Uuid,
    /// **Si legge e non si modifica**, ed è deliberato: non è un dato anagrafico,
    /// è la credenziale d'accesso. `PATCH /api/v1/me` non la accetta, e cambiarla
    /// richiede prima la verifica del nuovo indirizzo, che è fuori perimetro.
    #[schema(example = "mario.rossi@example.it")]
    pub email: String,
    /// Se l'indirizzo risulti verificato. Oggi è sempre assente — la verifica
    /// dell'email non è implementata — e il campo c'è perché il client possa
    /// mostrarlo senza cambiare forma il giorno in cui lo sarà.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_verified_at: Option<chrono::DateTime<chrono::Utc>>,
    #[schema(example = "Mario Rossi")]
    pub full_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "+39 333 1234567")]
    pub phone: Option<String>,
    /// Etichetta BCP 47. La usa il client per formattare date e importi con `Intl`.
    #[schema(example = "it-IT")]
    pub locale: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl MeResponse {
    fn from_domain(user: &User) -> Self {
        Self {
            id: user.id().as_uuid(),
            email: user.email().as_str().to_owned(),
            email_verified_at: user.email_verified_at(),
            full_name: user.full_name().to_owned(),
            phone: user.phone().map(str::to_owned),
            locale: user.locale().to_owned(),
            created_at: user.created_at(),
        }
    }
}

/// Modifica parziale. Un campo assente resta invariato; `phone` presente con
/// valore `null` viene azzerato.
///
/// **`email` si riconosce e si rifiuta**, e non si ignora. Un campo sconosciuto nel
/// corpo viene ignorato come ovunque nell'API — è la tolleranza che permette a un
/// client vecchio di parlare con un'API nuova — ma l'email non è un campo
/// sconosciuto: è quello che chiunque proverà a mandare per primo. Ignorarla
/// darebbe `200` con l'indirizzo di prima, cioè il sintomo muto che questa fetta
/// esiste per non produrre. Il `422` nomina il campo e dice perché.
///
/// **Non c'è `password`**: ha un percorso suo, che pretende quella vecchia. Un
/// `PATCH` che la accettasse la cambierebbe con la sola prova di avere una scheda
/// aperta.
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
pub struct UpdateMeRequest {
    /// Presente in qualunque forma, anche `null`, risponde `422`: l'email è la
    /// credenziale d'accesso e la chiave con cui inviti pendenti, policy e parti
    /// contrattuali si ritrovano. Si cambierà con la verifica del nuovo indirizzo.
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable)]
    pub email: Patch<String>,
    #[serde(default)]
    #[schema(example = "Mario Rossi")]
    pub full_name: Option<String>,
    #[serde(default, deserialize_with = "explicit_null")]
    #[schema(value_type = Option<String>, nullable, example = "+39 333 1234567")]
    pub phone: Patch<String>,
    #[serde(default)]
    #[schema(example = "it-IT")]
    pub locale: Option<String>,
}

impl UpdateMeRequest {
    fn into_changes(self) -> Result<UserProfileChanges, DomainError> {
        if self.email.is_some() {
            return Err(DomainError::validation(
                "email",
                "cannot be changed here: it is the sign-in credential, and pending invitations are keyed on it",
            ));
        }
        Ok(UserProfileChanges {
            full_name: self.full_name,
            phone: self.phone,
            locale: self.locale,
        })
    }
}

#[utoipa::path(
    get, path = "/api/v1/me", tag = "account",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "I propri dati. Risponde anche senza `X-Org-Id`", body = MeResponse),
        (status = 401, description = "Autenticazione richiesta", body = crate::error::ProblemDetails),
    )
)]
async fn read(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
) -> ApiResult<Json<MeResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let user = state
        .get_my_account
        .execute(&context)
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(MeResponse::from_domain(&user)))
}

#[utoipa::path(
    patch, path = "/api/v1/me", tag = "account",
    request_body = UpdateMeRequest,
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Dati aggiornati. Un corpo vuoto non è un errore: risponde con i dati invariati", body = MeResponse),
        (status = 401, description = "Autenticazione richiesta", body = crate::error::ProblemDetails),
        (status = 422, description = "Dati non validi: nome vuoto, telefono malformato, lingua che non è un'etichetta BCP 47 — oppure `email` nel corpo, che si riconosce e si rifiuta con `errors[].field = \"email\"`", body = crate::error::ProblemDetails),
    )
)]
async fn update(
    State(state): State<AppState>,
    Authenticated(context): Authenticated,
    Json(request): Json<UpdateMeRequest>,
) -> ApiResult<Json<MeResponse>> {
    let request_id = context.request_id().map(str::to_owned);
    let changes = request.into_changes().map_err(|error| {
        ApiError::from(propflow_application::AppError::Validation(error))
            .with_request_id(request_id.clone())
    })?;
    let user = state
        .update_my_profile
        .execute(&context, UpdateMyProfileCommand { changes })
        .await
        .map_err(|error| ApiError::from(error).with_request_id(request_id))?;

    Ok(Json(MeResponse::from_domain(&user)))
}
