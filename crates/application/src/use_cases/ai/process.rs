use std::sync::Arc;

use propflow_domain::ai::{AiJob, AiJobErrorCode, AiJobFailure, AiTask, OcrReading, ProviderUsage};
use propflow_domain::document::{AttachmentOwner, Document};
use propflow_domain::ports::{Clock, WorkerUnitOfWork, WorkerUnitOfWorkFactory};

use crate::error::{AppError, AppResult};
use crate::ports::{ObjectStorage, OcrError, OcrProvider};

/// Che cosa un giro del ciclo ha fatto. Serve al worker a decidere se dormire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Processed {
    /// La coda era vuota.
    Nothing,
    /// Un job è stato portato a `succeeded`.
    Succeeded,
    /// Un job è stato portato a `failed`. **Non è un errore del ciclo**: è un
    /// esito registrato, e il worker continua.
    Failed,
}

/// Il corpo del ciclo dell'OCR: prende un job dalla coda, lo lavora, lo chiude.
///
/// # Le tre fasi, e perché sono tre
///
/// ```text
/// ┌ transazione 1 (breve) ┐   ┌ nessuna transazione ┐   ┌ transazione 2 (breve) ┐
/// │ claim_next            │   │ storage.get         │   │ job.succeed | fail    │
/// │ job.start → running   │   │ provider.read       │   │ bill.attach_ocr       │
/// │ leggi il documento    │   │                     │   │ commit                │
/// └───────────────────────┘   └─────────────────────┘   └───────────────────────┘
/// ```
///
/// **La divisione è la trappola centrale del modulo**, e le due vie sbagliate si
/// somigliano entrambe a questa:
///
/// - prendere il job in una transazione, **chiuderla** e lavorare fuori senza aver
///   marcato la riga: il lock cade con la transazione, il job torna `queued` agli
///   occhi di chiunque, e **due worker leggono la stessa bolletta**;
/// - lavorare **dentro** la transazione del claim: corretto, e tiene una
///   connessione del pool occupata per tutti i secondi di una chiamata a un
///   fornitore esterno. Con quattro connessioni e quattro job lenti, il worker si
///   ferma del tutto.
///
/// La via è la terza: una transazione breve che porta il job a `running` — che è
/// ciò che lo **toglie dalla coda**, perché `claim_next` filtra su `queued` — e
/// una seconda transazione che lo chiude.
///
/// # Cosa succede se il processo muore a metà
///
/// Il job resta `running` per sempre. Non c'è scadenza, e `attempts` da sola non
/// lo rimette in coda: il ritentativo è **fuori perimetro**, perché deciderlo
/// richiede di sapere quali errori un fornitore vero produce. Va saputo perché il
/// sintomo è muto — una bolletta che non arriva mai a `pending_review` senza che
/// nulla sia in errore — e il rimedio, finché il ritentativo non esiste, è
/// guardare `app.ai_jobs` per gli `running` vecchi.
pub struct ProcessAiJob {
    unit_of_work: Arc<dyn WorkerUnitOfWorkFactory>,
    storage: Arc<dyn ObjectStorage>,
    provider: Arc<dyn OcrProvider>,
    clock: Arc<dyn Clock>,
}

impl ProcessAiJob {
    #[must_use]
    pub const fn new(
        unit_of_work: Arc<dyn WorkerUnitOfWorkFactory>,
        storage: Arc<dyn ObjectStorage>,
        provider: Arc<dyn OcrProvider>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            unit_of_work,
            storage,
            provider,
            clock,
        }
    }

    /// Un giro del ciclo.
    ///
    /// **Non prende un [`RequestContext`](propflow_domain::RequestContext), ed è
    /// l'unico use case del progetto che non lo fa.** Non è una dimenticanza: non
    /// c'è nessun chiamante da autorizzare. Il worker non serve una richiesta, e
    /// l'isolamento fra organizzazioni qui non è dato dalla Row Level Security ma
    /// dal fatto che ogni job porta la **propria** `org_id` e tocca il **proprio**
    /// soggetto.
    pub async fn execute(&self, task: AiTask) -> AppResult<Processed> {
        let Some((mut job, document)) = self.claim(task).await? else {
            return Ok(Processed::Nothing);
        };

        // Fuori da ogni transazione: qui si sta chiamando la rete, e una
        // connessione del database tenuta aperta nel frattempo è una connessione
        // sottratta a tutti gli altri.
        let outcome = match document {
            Ok(document) => self.read(&document).await,
            Err(failure) => Err(failure),
        };

        match outcome {
            Ok((reading, usage)) => self.finish_with_reading(job, reading, usage).await,
            Err(failure) => {
                let now = self.clock.now();
                job.fail(failure, now)?;
                let mut uow = self.unit_of_work.begin().await?;
                uow.ai_jobs().update(&job).await?;
                uow.commit().await?;
                // La bolletta **resta dov'era**: in bozza, con il suo allegato,
                // correggibile a mano come se l'OCR non esistesse.
                tracing::warn!(
                    job_id = %job.id(),
                    error_code = job.failure().map(|failure| failure.code.as_str()),
                    attempts = job.attempts(),
                    "lettura automatica non riuscita"
                );
                Ok(Processed::Failed)
            }
        }
    }

    /// Transazione 1: prende il job, lo marca `running`, legge il documento.
    ///
    /// Il documento si legge **qui dentro** e non dopo per una ragione precisa: è
    /// l'unico momento in cui si sta già pagando una transazione, e leggerlo dopo
    /// ne costerebbe una terza. Il `Result` interno porta il fallimento invece di
    /// interromperlo: un documento archiviato è un job da chiudere `failed`, non
    /// un errore del ciclo — e per chiuderlo bisogna prima averlo marcato
    /// `running`, che è quello che questa transazione ha appena fatto.
    async fn claim(
        &self,
        task: AiTask,
    ) -> AppResult<Option<(AiJob, Result<Document, AiJobFailure>)>> {
        let now = self.clock.now();
        let mut uow = self.unit_of_work.begin().await?;

        let Some(mut job) = uow.ai_jobs().claim_next(task).await? else {
            // Nessun `commit`: il `Drop` fa rollback, e non c'è niente da salvare.
            return Ok(None);
        };
        job.start(now)?;
        uow.ai_jobs().update(&job).await?;

        let document = self.load_document(uow.as_mut(), &job).await?;

        uow.commit().await?;
        tracing::debug!(job_id = %job.id(), attempts = job.attempts(), "job preso in carico");
        Ok(Some((job, document)))
    }

    /// Il documento da leggere, oppure il motivo per cui non c'è.
    async fn load_document(
        &self,
        uow: &mut dyn WorkerUnitOfWork,
        job: &AiJob,
    ) -> AppResult<Result<Document, AiJobFailure>> {
        let Some(document_id) = job.input_document_id() else {
            // `ai_jobs.input_document_id` è `ON DELETE SET NULL`: la colonna può
            // diventare nulla da sé, senza che nessuno se ne accorga.
            return Ok(Err(AiJobFailure::new(
                AiJobErrorCode::DocumentArchived,
                "il documento in ingresso non è più referenziato",
            )));
        };

        // `find_by_id` filtra `deleted_at IS NULL`, quindi un documento archiviato
        // torna `None` **anche al worker**, che pure ha `BYPASSRLS` e la riga la
        // vedrebbe. È la decisione che il piano lasciava aperta, e la risposta è
        // sì: un job su un documento archiviato **fallisce**. Altrimenti la
        // macchina legge ciò che una persona ha deciso di togliere — e lo
        // ripropone alla stessa persona come se fosse ancora lì.
        match uow.documents().find_by_id(document_id).await? {
            Some(document) => Ok(Ok(document)),
            None => Ok(Err(AiJobFailure::new(
                AiJobErrorCode::DocumentArchived,
                "il documento in ingresso è stato archiviato dopo l'accodamento",
            ))),
        }
    }

    /// Fuori transazione: i byte dallo storage, poi il fornitore.
    async fn read(&self, document: &Document) -> Result<(OcrReading, ProviderUsage), AiJobFailure> {
        let bytes = self
            .storage
            .get(document.storage().object_key())
            .await
            .map_err(|error| {
                // Un oggetto che manca per un documento che c'è è
                // un'**incoerenza fra database e bucket**, non un errore del
                // fornitore: merita un codice suo perché il rimedio è guardare lo
                // storage, non il fornitore.
                AiJobFailure::new(AiJobErrorCode::DocumentUnavailable, error.to_string())
            })?;

        match self
            .provider
            .read(&bytes, document.mime_type().as_str())
            .await
        {
            Ok(outcome) => Ok((outcome.reading, outcome.usage)),
            Err(error @ OcrError::UnsupportedDocument { .. }) => Err(AiJobFailure::new(
                AiJobErrorCode::UnsupportedDocument,
                error.to_string(),
            )),
            Err(error @ OcrError::Backend { .. }) => Err(AiJobFailure::new(
                AiJobErrorCode::ProviderUnavailable,
                error.to_string(),
            )),
        }
    }

    /// Transazione 2: chiude il job e porta la bolletta in revisione.
    ///
    /// Le due scritture stanno **nella stessa transazione**, e non è pignoleria:
    /// separarle produrrebbe o un job `succeeded` su una bolletta rimasta in
    /// bozza — cioè una lettura che nessuno vedrà mai — o una bolletta in
    /// revisione il cui job risulta ancora `running`. Sono le due incoerenze che
    /// nessuno saprebbe diagnosticare guardando una riga sola.
    async fn finish_with_reading(
        &self,
        mut job: AiJob,
        reading: OcrReading,
        usage: ProviderUsage,
    ) -> AppResult<Processed> {
        let now = self.clock.now();
        let AttachmentOwner::UtilityBill(bill_id) = job.subject() else {
            // `ai_jobs.subject_type` è `app.attachment_owner_type`, che ha undici
            // valori: un job `bill_ocr` su un soggetto che non è una bolletta è
            // una riga scritta a mano, e va chiusa come tale invece di essere
            // interpretata.
            return self
                .finish_with_failure(
                    job,
                    AiJobFailure::new(
                        AiJobErrorCode::SubjectNotProcessable,
                        "il soggetto del job non è una bolletta",
                    ),
                )
                .await;
        };

        let mut uow = self.unit_of_work.begin().await?;
        let Some(mut bill) = uow.utility_bills().find_by_id(bill_id).await? else {
            drop(uow);
            return self
                .finish_with_failure(
                    job,
                    AiJobFailure::new(
                        AiJobErrorCode::SubjectNotProcessable,
                        "la bolletta è stata annullata mentre il job era in coda",
                    ),
                )
                .await;
        };

        // La bolletta si rilegge **adesso**, non si porta dietro dal claim: fra
        // le due transazioni una persona può averla validata, riaddebitata o
        // annullata, e `attach_ocr` rifiuta tutti e tre i casi. È la stessa
        // ragione per cui `RecordPayment` rilegge l'addebito.
        if bill.attach_ocr(job.id(), reading, now).is_err() {
            drop(uow);
            return self
                .finish_with_failure(
                    job,
                    AiJobFailure::new(
                        AiJobErrorCode::SubjectNotProcessable,
                        "la bolletta non è più una bozza",
                    ),
                )
                .await;
        }

        job.succeed(
            bill.ocr_extracted().ok_or_else(|| {
                AppError::internal("la bolletta non ha registrato la lettura appena allegata")
            })?,
            usage,
            now,
        )?;
        uow.utility_bills().update(&bill).await?;
        uow.ai_jobs().update(&job).await?;
        uow.commit().await?;

        tracing::info!(
            job_id = %job.id(),
            utility_bill_id = %bill_id,
            fields = job.output().len(),
            // La confidenza è un numero, non un dato personale: dice quanto la
            // macchina si fida, non che cosa ha letto.
            confidence = job.confidence().map(|value| value.to_string()),
            "bolletta portata in revisione"
        );
        Ok(Processed::Succeeded)
    }

    /// Chiude un job che era già `running` quando la lavorazione si è fermata.
    async fn finish_with_failure(
        &self,
        mut job: AiJob,
        failure: AiJobFailure,
    ) -> AppResult<Processed> {
        let now = self.clock.now();
        let code = failure.code;
        job.fail(failure, now)?;
        let mut uow = self.unit_of_work.begin().await?;
        uow.ai_jobs().update(&job).await?;
        uow.commit().await?;
        tracing::warn!(
            job_id = %job.id(),
            error_code = code.as_str(),
            "job chiuso senza proposta"
        );
        Ok(Processed::Failed)
    }
}
