use chrono::{DateTime, Utc};

use propflow_domain::ai::{AiCapability, AiJob, AiTask, NewAiJob};
use propflow_domain::billing::BillStatus;
use propflow_domain::document::{AttachmentOwner, Document};
use propflow_domain::ports::UnitOfWork;
use propflow_domain::shared::AiJobId;

use crate::error::AppResult;

/// Accoda la lettura automatica di una bolletta, se il documento appena caricato
/// la merita.
///
/// **Non è uno use case con un `execute` suo, ed è deliberato.** Prende in
/// prestito una [`UnitOfWork`] già aperta invece di aprirne una, perché il job va
/// scritto **nella stessa transazione che registra il documento**. Un job accodato
/// a parte punterebbe a una riga che potrebbe non essere mai stata committata: il
/// worker lo prenderebbe, non troverebbe il documento, e fallirebbe con un errore
/// che a quel punto non ha più niente a che vedere con la sua causa — un
/// caricamento andato storto mezzo secondo prima, altrove.
///
/// È la stessa forma di `ensure_owner_is_visible`, e per la stessa ragione: ciò
/// che deve stare in una transazione si scrive come una funzione che la riceve,
/// non come uno use case che ne apre un'altra.
///
/// # Quando accoda, e quando no
///
/// Tre condizioni, tutte necessarie, e ognuna evita un caso concreto:
///
/// | Condizione | Cosa evita |
/// |---|---|
/// | il documento è allegato a una **bolletta** | leggere la foto di un guasto come se fosse una fattura |
/// | la bolletta è in **bozza** | riportare a «da rivedere» una bolletta che una persona ha già validato, riaddebitato o annullato |
/// | la bolletta è **visibile** a chi carica | accodare lavoro su una riga che il chiamante non potrebbe nemmeno leggere |
///
/// **Il `kind` del documento non è fra le condizioni**, ed è una scelta: un
/// allegato a una bolletta è la scansione di quella bolletta, comunque lo abbia
/// classificato chi lo ha caricato. Vincolarlo a `DocumentKind::UtilityBill`
/// significherebbe che l'OCR non parte quando qualcuno sceglie `invoice` — che è
/// altrettanto ragionevole — e il sintomo sarebbe una funzione che «a volte non
/// funziona» senza che nulla lo dica.
///
/// # Perché non fallisce mai la richiesta
///
/// Restituisce `Ok(None)` quando non c'è niente da accodare, e gli errori che
/// restituisce sono solo quelli della scrittura. Un caricamento **deve riuscire
/// anche quando l'OCR non è disponibile**: la bolletta resta in bozza con il suo
/// allegato, che è esattamente ciò che si aveva prima che questo modulo
/// esistesse.
pub(crate) async fn enqueue_bill_ocr(
    uow: &mut dyn UnitOfWork,
    document: &Document,
    provider_name: &str,
    now: DateTime<Utc>,
) -> AppResult<Option<AiJobId>> {
    let AttachmentOwner::UtilityBill(bill_id) = document.owner() else {
        return Ok(None);
    };

    // Riletta dal repository, quindi **sotto le policy del chiamante**: se la
    // bolletta non è sua, non c'è job da accodare. Il criterio non è riscritto a
    // mano qui, come in `ensure_owner_is_visible`.
    let Some(bill) = uow.utility_bills().find_by_id(bill_id).await? else {
        return Ok(None);
    };
    if !matches!(bill.status(), BillStatus::Draft) {
        return Ok(None);
    }

    let job = AiJob::queue(
        NewAiJob {
            id: AiJobId::generate(),
            org_id: bill.org_id(),
            capability: AiCapability::Ocr,
            task: AiTask::BillOcr,
            provider_name: provider_name.to_owned(),
            subject: AttachmentOwner::UtilityBill(bill_id),
            input_document_id: Some(document.id()),
        },
        now,
    );
    uow.ai_jobs().enqueue(&job).await?;
    Ok(Some(job.id()))
}
