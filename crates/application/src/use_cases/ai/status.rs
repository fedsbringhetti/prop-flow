use std::sync::Arc;

use propflow_domain::ai::AiJob;
use propflow_domain::billing::UtilityBill;
use propflow_domain::document::AttachmentOwner;
use propflow_domain::ports::UnitOfWorkFactory;
use propflow_domain::shared::UtilityBillId;
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

/// A che punto è la lettura automatica di una bolletta.
///
/// La bolletta c'è sempre — altrimenti la risposta è `404` — e il job può non
/// esserci: `None` significa «nessuno ha ancora caricato una scansione», che è la
/// risposta onesta e non un errore.
#[derive(Debug, Clone)]
pub struct BillOcrStatus {
    pub bill: UtilityBill,
    pub job: Option<AiJob>,
}

/// Legge lo stato del job di OCR di una bolletta, con la proposta che ha
/// prodotto.
///
/// **Sola lettura, e per staff.** `ensure_staff` e non `ensure_member` perché è la
/// stessa condizione di `ai_jobs_select`, che è `org_id = current_org_id() AND
/// is_org_staff(org_id)`: senza, un inquilino che vede la bolletta imputata al
/// proprio contratto otterrebbe zero righe sul job e leggerebbe «nessuna lettura
/// automatica», che è falso. Il criterio applicativo ripete quello della policy per
/// la ragione di sempre — la RLS da sola risponderebbe con un'assenza dove il
/// chiamante merita un `403`.
///
/// **Perché questa rotta esiste**, visto che non fa niente. Senza, il percorso
/// dell'OCR funziona e si osserva solo da `psql` e dai log: per un modulo il cui
/// valore *è* il percorso, quella è precisamente la parte da poter mostrare. Costa
/// poco — una lettura su una risorsa che esiste già — e rende la fetta
/// ispezionabile da fuori.
pub struct GetBillOcrStatus {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl GetBillOcrStatus {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        id: UtilityBillId,
    ) -> AppResult<BillOcrStatus> {
        authorization::ensure_staff(context)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        let bill = uow
            .utility_bills()
            .find_by_id(id)
            .await?
            .ok_or(AppError::not_found(Entity::UtilityBill))?;
        let job = uow
            .ai_jobs()
            .find_latest_for_subject(AttachmentOwner::UtilityBill(id))
            .await?;
        Ok(BillOcrStatus { bill, job })
    }
}
