use chrono::{DateTime, Utc};

use super::extraction::OcrReading;
use super::value_objects::{Confidence, ProviderUsage};
use crate::document::AttachmentOwner;
use crate::error::{DomainError, DomainResult, Invariant};
use crate::shared::{AiJobId, DocumentId, OrgId};

/// Capacità richiesta a un fornitore.
///
/// Rispecchia `app.ai_capability` per intero. **L'applicazione scrive solo
/// `Ocr`**: `Vision`, `Llm` ed `Embedding` restano varianti non scritte, come
/// `percentage` fra i metodi di ripartizione. Non sono una dimenticanza — sono i
/// moduli che verranno, e ciascuno porta con sé una decisione di prodotto sua
/// (con quale soglia una macchina può assegnare una categoria a una
/// segnalazione, per esempio) che non si prende scrivendo un adattatore.
///
/// Restano fra le varianti perché una riga scritta a mano potrebbe contenerle, e
/// un valore sconosciuto in lettura deve dare errore invece di ricadere su un
/// default plausibile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiCapability {
    Ocr,
    /// Mai scritta dall'applicazione: vedi la nota sul tipo.
    Vision,
    /// Mai scritta dall'applicazione: vedi la nota sul tipo.
    Llm,
    /// Mai scritta dall'applicazione: vedi la nota sul tipo.
    Embedding,
}

impl AiCapability {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ocr => "ocr",
            Self::Vision => "vision",
            Self::Llm => "llm",
            Self::Embedding => "embedding",
        }
    }
}

/// Compito applicativo del job.
///
/// A schema è `TEXT` e non un enum — il commento della `0010` elenca
/// `'bill_ocr'`, `'ticket_triage'`, `'ticket_reply_draft'` — ma qui è un enum
/// chiuso lo stesso: `task` è metà di `ai_jobs_org_task_ix` e il criterio con cui
/// il worker sceglie cosa fare, quindi un refuso in un letterale sarebbe un job
/// che nessun ciclo prende in carico e che resta `queued` per sempre senza che
/// nulla protesti.
///
/// **L'applicazione scrive solo `BillOcr`.** Il triage delle segnalazioni è fuori
/// perimetro: `tickets.ai_job_id` e `ticket_messages.ai_job_id` restano nulli.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiTask {
    BillOcr,
    /// Mai scritto dall'applicazione: il triage è il modulo successivo.
    TicketTriage,
    /// Mai scritto dall'applicazione: vedi sopra.
    TicketReplyDraft,
}

impl AiTask {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BillOcr => "bill_ocr",
            Self::TicketTriage => "ticket_triage",
            Self::TicketReplyDraft => "ticket_reply_draft",
        }
    }

    /// `None` per un compito sconosciuto: un job scritto a mano con un `task` che
    /// il codice non conosce deve dare errore in lettura, non essere trattato
    /// come un OCR.
    #[must_use]
    pub fn from_db(value: &str) -> Option<Self> {
        [Self::BillOcr, Self::TicketTriage, Self::TicketReplyDraft]
            .into_iter()
            .find(|task| task.as_str() == value)
    }
}

/// Stato di un job.
///
/// Il ciclo che l'applicazione percorre è `queued → running → succeeded | failed`.
///
/// **`Cancelled` esiste nell'enum del database e l'applicazione non lo scrive
/// mai**, ed è una decisione presa e non una dimenticanza. Un job si chiude in
/// millisecondi e non c'è nulla da annullare finché non esiste un fornitore vero
/// che impieghi secondi; una rotta di annullamento sarebbe codice per un caso che
/// non si può nemmeno provare. C'è anche una domanda che scriverlo aprirebbe:
/// `ai_jobs_finished_ck` esige `finished_at` sui soli `succeeded` e `failed`,
/// quindi un job annullato non avrebbe una data di fine — e cosa significhi un
/// terminale senza data di fine è una decisione a sé, da prendere insieme al
/// ritentativo, che è l'altra cosa fuori perimetro.
///
/// Resta fra le varianti per la regola di sempre: un valore sconosciuto in
/// lettura deve dare errore.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiJobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    /// Mai scritto dall'applicazione: vedi la nota sul tipo.
    Cancelled,
}

impl AiJobStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// `true` per gli stati da cui il job non si muove più.
    ///
    /// `Cancelled` è fra loro anche se nessuno lo scrive: un job letto da una
    /// riga scritta a mano deve risultare concluso, non riprendibile.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }

    /// `true` per i due stati che `ai_jobs_finished_ck` obbliga ad avere
    /// `finished_at`.
    ///
    /// `Cancelled` **non** è fra loro, ed è esattamente ciò che rende quel valore
    /// una decisione rimandata invece di una riga in più.
    #[must_use]
    pub const fn requires_finished_at(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed)
    }
}

/// Perché un job è fallito.
///
/// Un enum e non testo libero per la ragione di sempre in questo progetto: da qui
/// nasce `ai_jobs.error_code`, che è ciò su cui si conta quante letture falliscono
/// e per cosa. Il messaggio che lo accompagna è invece libero, ed è dove il
/// fornitore può dire la sua.
///
/// **Nessuno di questi codici arriva a un client.** `error_code` si legge dalla
/// rotta di stato del job, che è riservata allo staff, e non entra mai nel `code`
/// di una risposta di errore: un job fallito è un esito riuscito della richiesta
/// che lo ha accodato.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiJobErrorCode {
    /// Il documento in ingresso è stato archiviato dopo l'accodamento.
    ///
    /// Il worker gira con `BYPASSRLS` e la riga la vedrebbe comunque: fallire è
    /// la scelta, perché altrimenti la macchina legge ciò che una persona ha
    /// deciso di togliere.
    DocumentArchived,
    /// Il fornitore ha rifiutato il documento: formato che non sa leggere,
    /// scansione illeggibile.
    UnsupportedDocument,
    /// Il fornitore non ha risposto, o ha risposto un errore suo.
    ProviderUnavailable,
    /// Il soggetto del job non è più in uno stato in cui una proposta abbia
    /// senso: la bolletta è stata validata, riaddebitata o annullata mentre il
    /// job era in coda.
    SubjectNotProcessable,
    /// L'oggetto del documento non è nello storage.
    ///
    /// Un'incoerenza fra database e bucket, non un errore del fornitore: merita
    /// un codice suo perché il rimedio è guardare lo storage, non il fornitore.
    DocumentUnavailable,
}

impl AiJobErrorCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DocumentArchived => "document_archived",
            Self::UnsupportedDocument => "unsupported_document",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::SubjectNotProcessable => "subject_not_processable",
            Self::DocumentUnavailable => "document_unavailable",
        }
    }
}

/// Il fallimento completo: il codice e ciò che il fornitore ha detto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiJobFailure {
    pub code: AiJobErrorCode,
    pub message: String,
}

impl AiJobFailure {
    /// Lo schema tiene `error_message` a `TEXT` senza vincoli: il tetto è nostro,
    /// e serve perché un fornitore che risponde con una pagina HTML di errore non
    /// riempia la colonna con un documento.
    pub const MAX_MESSAGE_LENGTH: usize = 2000;

    #[must_use]
    pub fn new(code: AiJobErrorCode, message: impl Into<String>) -> Self {
        let mut message: String = message.into();
        message.truncate(Self::MAX_MESSAGE_LENGTH);
        Self { code, message }
    }
}

/// Dati necessari ad accodare un job.
#[derive(Debug, Clone)]
pub struct NewAiJob {
    pub id: AiJobId,
    pub org_id: OrgId,
    pub capability: AiCapability,
    pub task: AiTask,
    /// Il nome del fornitore come **dato**, non come ramo del codice: è la regola
    /// di `docs/06-sviluppo.md` §Aggiungere un provider esterno, e qui vale
    /// doppio perché il fornitore vero non è ancora scelto.
    pub provider_name: String,
    /// L'oggetto su cui il job lavora.
    ///
    /// `ai_jobs.subject_type` è `app.attachment_owner_type` e non un enum suo, il
    /// che significa **undici** valori a schema contro i sei che
    /// [`AttachmentOwner`] modella: leggere un job il cui soggetto è fuori
    /// perimetro deve dare errore, come già fa `attachment_owner_type_from_db`.
    pub subject: AttachmentOwner,
    /// Il documento da leggere. `ai_jobs.input_document_id` è
    /// `ON DELETE SET NULL`, quindi può diventare nullo senza che nessuno se ne
    /// accorga; e i documenti hanno il **soft delete**, quindi può restare
    /// valorizzato puntando a una riga che l'API non mostra più.
    pub input_document_id: Option<DocumentId>,
}

/// Un lavoro affidato a una macchina.
///
/// È l'unica scrittura di questo progetto che **non** accoda in outbox, e va
/// motivato perché rompe una regolarità. Un job dell'OCR non è un fatto di
/// dominio: è un dettaglio di come una bolletta arriva a `pending_review`. Il
/// fatto che chi consuma vuole conoscere è la bolletta che ha cambiato stato, e
/// quello lo annuncia già `billing.utility_bill.status_changed`. Accodare
/// `ai.job.queued` e `ai.job.succeeded` darebbe a un consumatore tre eventi per
/// un fatto solo, due dei quali su un aggregato che nessuno fuori da qui conosce.
///
/// **Non è nemmeno tracciato in `audit_log`**, e neanche questo è una
/// dimenticanza: la `0013` esclude `ai_jobs` dall'audit per volume, insieme a
/// `ticket_messages`, `user_sessions`, `domain_events` e
/// `ticket_status_history`. Ciò che una macchina ha fatto **alla bolletta** resta
/// invece tracciato, perché `app.utility_bills` ha il trigger di audit — con
/// `ocr_extracted` fra le colonne redatte.
///
/// **`request_params` resta al suo default `{}`.** Il commento della `0010`
/// chiede che in `ai_jobs` vadano «SOLO parametri e riferimenti, mai il prompt con
/// PII, mai l'immagine»: finché la porta non ha parametri da passare, la colonna
/// non ha niente di onesto da contenere. `trace_id` resta nullo per la stessa
/// ragione — è la correlazione con OpenTelemetry, e scriverci il nostro
/// `request_id` significherebbe chiamare traccia una cosa che non lo è.
#[derive(Debug, Clone)]
pub struct AiJob {
    id: AiJobId,
    org_id: OrgId,
    capability: AiCapability,
    task: AiTask,
    status: AiJobStatus,
    provider_name: String,
    model: Option<String>,
    provider_version: Option<String>,
    subject: AttachmentOwner,
    input_document_id: Option<DocumentId>,
    output: Vec<(super::BillField, Confidence)>,
    confidence: Option<Confidence>,
    usage: ProviderUsage,
    attempts: u16,
    failure: Option<AiJobFailure>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl AiJob {
    /// Accoda un job. Nasce sempre `queued`, e non è solo una convenzione: la
    /// policy `ai_jobs_enqueue` della migration `0031` ha `status = 'queued'`
    /// nella `WITH CHECK`, quindi un job che nascesse altrove verrebbe rifiutato
    /// dalla Row Level Security.
    #[must_use]
    pub fn queue(parts: NewAiJob, now: DateTime<Utc>) -> Self {
        let NewAiJob {
            id,
            org_id,
            capability,
            task,
            provider_name,
            subject,
            input_document_id,
        } = parts;
        Self {
            id,
            org_id,
            capability,
            task,
            status: AiJobStatus::Queued,
            provider_name,
            model: None,
            provider_version: None,
            subject,
            input_document_id,
            output: Vec::new(),
            confidence: None,
            usage: ProviderUsage::default(),
            attempts: 0,
            failure: None,
            started_at: None,
            finished_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Prende in carico il job: `queued → running`, e incrementa `attempts`.
    ///
    /// Il passaggio a `running` è ciò che **toglie il job dalla coda**, perché
    /// `ai_jobs_queue_ix` è parziale su `('queued','running')` e chi accoda
    /// filtra sul solo `queued`. È la ragione per cui questa transizione va
    /// scritta e committata **prima** di chiamare il fornitore: fra due worker che
    /// competono, la finestra fra il `SELECT … FOR UPDATE SKIP LOCKED` e questa
    /// scrittura è l'unica in cui il job è preso e non ancora marcato, e va tenuta
    /// corta quanto una transazione senza chiamate di rete.
    ///
    /// `attempts` sale qui e non alla chiusura: un processo che muore a metà
    /// lavoro deve lasciare traccia di averci provato. Il ritentativo resta fuori
    /// perimetro — nessuno riporta a `queued` un job abbandonato — ma il contatore
    /// è già quello giusto per quando esisterà.
    pub fn start(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if !matches!(self.status, AiJobStatus::Queued) {
            return Err(DomainError::Invariant(Invariant::AiJobNotStartable));
        }
        self.status = AiJobStatus::Running;
        self.started_at = Some(now);
        self.attempts = self.attempts.saturating_add(1);
        self.updated_at = now;
        Ok(())
    }

    /// Chiude il job con successo.
    ///
    /// `finished_at` lo valorizza **questa funzione**, e nessuno lo fa al posto
    /// nostro: `ai_jobs_finished_ck` è
    /// `status <> ANY('succeeded','failed') OR finished_at IS NOT NULL`, cioè
    /// costringe una sola direzione — obbliga ad avere la data quando si è
    /// conclusi, e non impedisce di lasciarla su un job che torna indietro. È la
    /// **quinta** occorrenza di questa forma dopo `tickets_closed_ck`,
    /// `utility_bills_validated_ck`, `payments_succeeded_ck` e
    /// `settlements_confirmed_ck`: a questo punto è una convenzione dello schema,
    /// e l'azzeramento tocca sempre al dominio.
    ///
    /// **Una confidenza bassa non è un fallimento**: si passa comunque da qui.
    pub fn succeed(
        &mut self,
        reading: &OcrReading,
        usage: ProviderUsage,
        now: DateTime<Utc>,
    ) -> DomainResult<()> {
        if !matches!(self.status, AiJobStatus::Running) {
            return Err(DomainError::Invariant(Invariant::AiJobNotFinishable));
        }
        usage.ensure_storable()?;

        self.status = AiJobStatus::Succeeded;
        // Solo la **forma**: quali campi con quale confidenza, senza i valori.
        // Quelli stanno sulla bolletta, che è dove il contenuto di una bolletta
        // deve stare.
        self.output = reading.shape();
        self.confidence = reading.overall_confidence();
        self.model = usage.model.clone();
        self.provider_version = usage.provider_version.clone();
        self.usage = usage;
        self.failure = None;
        self.finished_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    /// Chiude il job con un errore. La bolletta **resta dov'era**.
    ///
    /// Nessun `output` e nessuna confidenza: un fallimento non è una lettura a
    /// confidenza zero, ed è la distinzione che rende misurabile l'accuratezza —
    /// una media che comprendesse i fallimenti misurerebbe la disponibilità del
    /// fornitore invece della sua precisione.
    pub fn fail(&mut self, failure: AiJobFailure, now: DateTime<Utc>) -> DomainResult<()> {
        if !matches!(self.status, AiJobStatus::Running) {
            return Err(DomainError::Invariant(Invariant::AiJobNotFinishable));
        }
        self.status = AiJobStatus::Failed;
        self.failure = Some(failure);
        self.finished_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    #[must_use]
    pub const fn id(&self) -> AiJobId {
        self.id
    }
    #[must_use]
    pub const fn org_id(&self) -> OrgId {
        self.org_id
    }
    #[must_use]
    pub const fn capability(&self) -> AiCapability {
        self.capability
    }
    #[must_use]
    pub const fn task(&self) -> AiTask {
        self.task
    }
    #[must_use]
    pub const fn status(&self) -> AiJobStatus {
        self.status
    }
    #[must_use]
    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }
    #[must_use]
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }
    #[must_use]
    pub fn provider_version(&self) -> Option<&str> {
        self.provider_version.as_deref()
    }
    #[must_use]
    pub const fn subject(&self) -> AttachmentOwner {
        self.subject
    }
    #[must_use]
    pub const fn input_document_id(&self) -> Option<DocumentId> {
        self.input_document_id
    }
    #[must_use]
    pub fn output(&self) -> &[(super::BillField, Confidence)] {
        &self.output
    }
    #[must_use]
    pub const fn confidence(&self) -> Option<Confidence> {
        self.confidence
    }
    #[must_use]
    pub const fn usage(&self) -> &ProviderUsage {
        &self.usage
    }
    #[must_use]
    pub const fn attempts(&self) -> u16 {
        self.attempts
    }
    #[must_use]
    pub const fn failure(&self) -> Option<&AiJobFailure> {
        self.failure.as_ref()
    }
    #[must_use]
    pub const fn started_at(&self) -> Option<DateTime<Utc>> {
        self.started_at
    }
    #[must_use]
    pub const fn finished_at(&self) -> Option<DateTime<Utc>> {
        self.finished_at
    }
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Ricostruzione da persistenza: non riesegue le invarianti, perché un job già
    /// salvato deve poter essere riletto anche se le regole sono cambiate da
    /// allora.
    #[must_use]
    pub fn from_persistence(parts: AiJobParts) -> Self {
        let AiJobParts {
            id,
            org_id,
            capability,
            task,
            status,
            provider_name,
            model,
            provider_version,
            subject,
            input_document_id,
            output,
            confidence,
            usage,
            attempts,
            failure,
            started_at,
            finished_at,
            created_at,
            updated_at,
        } = parts;
        Self {
            id,
            org_id,
            capability,
            task,
            status,
            provider_name,
            model,
            provider_version,
            subject,
            input_document_id,
            output,
            confidence,
            usage,
            attempts,
            failure,
            started_at,
            finished_at,
            created_at,
            updated_at,
        }
    }
}

/// Dati grezzi di un job letto dalla persistenza.
#[derive(Debug, Clone)]
pub struct AiJobParts {
    pub id: AiJobId,
    pub org_id: OrgId,
    pub capability: AiCapability,
    pub task: AiTask,
    pub status: AiJobStatus,
    pub provider_name: String,
    pub model: Option<String>,
    pub provider_version: Option<String>,
    pub subject: AttachmentOwner,
    pub input_document_id: Option<DocumentId>,
    pub output: Vec<(super::BillField, Confidence)>,
    pub confidence: Option<Confidence>,
    pub usage: ProviderUsage,
    pub attempts: u16,
    pub failure: Option<AiJobFailure>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{BillField, ExtractedField, ExtractedValue};
    use crate::shared::UtilityBillId;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn confidence(raw: &str) -> Confidence {
        Confidence::new(Decimal::from_str(raw).expect("decimale valido")).expect("confidenza")
    }

    fn job() -> AiJob {
        AiJob::queue(
            NewAiJob {
                id: AiJobId::generate(),
                org_id: OrgId::generate(),
                capability: AiCapability::Ocr,
                task: AiTask::BillOcr,
                provider_name: "deterministic".to_owned(),
                subject: AttachmentOwner::UtilityBill(UtilityBillId::generate()),
                input_document_id: Some(DocumentId::generate()),
            },
            Utc::now(),
        )
    }

    fn reading() -> OcrReading {
        OcrReading::new(vec![ExtractedField {
            field: BillField::TotalAmount,
            value: ExtractedValue::Amount(Decimal::from_str("128.40").expect("importo")),
            confidence: confidence("0.91"),
        }])
    }

    /// Porta il job allo stato richiesto percorrendo il flusso nominale, così
    /// che una transizione mancante nel percorso si veda subito.
    fn at(status: AiJobStatus) -> AiJob {
        let now = Utc::now();
        let mut job = job();
        match status {
            AiJobStatus::Queued => {}
            AiJobStatus::Running => job.start(now).expect("preso in carico"),
            AiJobStatus::Succeeded => {
                job.start(now).expect("preso in carico");
                job.succeed(&reading(), ProviderUsage::default(), now)
                    .expect("concluso");
            }
            AiJobStatus::Failed => {
                job.start(now).expect("preso in carico");
                job.fail(
                    AiJobFailure::new(AiJobErrorCode::ProviderUnavailable, "niente"),
                    now,
                )
                .expect("fallito");
            }
            // Mai prodotto dall'applicazione: vedi la nota su `AiJobStatus`.
            AiJobStatus::Cancelled => {}
        }
        job
    }

    #[test]
    fn a_queued_job_has_not_been_attempted_yet() {
        let job = job();
        assert_eq!(job.status(), AiJobStatus::Queued);
        assert_eq!(job.attempts(), 0);
        assert_eq!(job.started_at(), None);
        assert_eq!(job.finished_at(), None);
        assert_eq!(job.confidence(), None);
        assert!(job.output().is_empty());
    }

    /// La tabella delle transizioni, ammesse **e rifiutate**.
    #[test]
    fn the_transition_table_is_complete() {
        use AiJobStatus::{Failed, Queued, Running, Succeeded};

        for from in [Queued, Running, Succeeded, Failed] {
            assert_eq!(
                at(from).start(Utc::now()).is_ok(),
                matches!(from, Queued),
                "start da {from:?}"
            );
        }
        for from in [Queued, Running, Succeeded, Failed] {
            assert_eq!(
                at(from)
                    .succeed(&reading(), ProviderUsage::default(), Utc::now())
                    .is_ok(),
                matches!(from, Running),
                "succeed da {from:?}"
            );
        }
        for from in [Queued, Running, Succeeded, Failed] {
            assert_eq!(
                at(from)
                    .fail(
                        AiJobFailure::new(AiJobErrorCode::ProviderUnavailable, "niente"),
                        Utc::now()
                    )
                    .is_ok(),
                matches!(from, Running),
                "fail da {from:?}"
            );
        }
    }

    /// La quinta occorrenza del `CHECK` a una direzione sola: se non lo scrive
    /// il dominio, `ai_jobs_finished_ck` rifiuta la riga.
    #[test]
    fn both_terminal_states_carry_the_moment_they_ended() {
        for status in [AiJobStatus::Succeeded, AiJobStatus::Failed] {
            let job = at(status);
            assert_eq!(job.status(), status);
            assert!(
                job.finished_at().is_some(),
                "{status:?} senza finished_at: ai_jobs_finished_ck rifiuterebbe la riga"
            );
            assert!(status.requires_finished_at());
        }
        // `cancelled` è il valore per cui il vincolo **non** chiede la data, ed è
        // metà della ragione per cui l'applicazione non lo scrive.
        assert!(!AiJobStatus::Cancelled.requires_finished_at());
        assert!(AiJobStatus::Cancelled.is_terminal());
    }

    #[test]
    fn taking_a_job_counts_the_attempt_and_takes_it_out_of_the_queue() {
        let mut job = job();
        job.start(Utc::now()).expect("preso in carico");
        assert_eq!(job.status(), AiJobStatus::Running);
        assert_eq!(job.attempts(), 1);
        assert!(job.started_at().is_some());
        assert!(job.finished_at().is_none());
    }

    /// Il confine fra le due colonne JSONB, verificato sull'aggregato: il job
    /// conserva la forma e **non** i valori.
    #[test]
    fn a_succeeded_job_keeps_the_shape_and_not_the_values() {
        let job = at(AiJobStatus::Succeeded);
        assert_eq!(
            job.output(),
            &[(BillField::TotalAmount, confidence("0.91"))]
        );
        assert_eq!(job.confidence(), Some(confidence("0.91")));
        assert!(job.failure().is_none());
    }

    #[test]
    fn a_failed_job_carries_its_code_and_no_reading() {
        let job = at(AiJobStatus::Failed);
        assert_eq!(
            job.failure().map(|failure| failure.code),
            Some(AiJobErrorCode::ProviderUnavailable)
        );
        assert!(job.output().is_empty());
        assert_eq!(
            job.confidence(),
            None,
            "un fallimento non è confidenza zero"
        );
    }

    #[test]
    fn a_provider_message_cannot_fill_the_column_with_a_document() {
        let failure = AiJobFailure::new(AiJobErrorCode::ProviderUnavailable, "x".repeat(9000));
        assert_eq!(failure.message.len(), AiJobFailure::MAX_MESSAGE_LENGTH);
    }

    #[test]
    fn a_counter_that_does_not_fit_an_integer_column_stops_the_job() {
        let mut job = at(AiJobStatus::Running);
        let usage = ProviderUsage {
            tokens_input: Some(ProviderUsage::MAX_COUNTER + 1),
            ..ProviderUsage::default()
        };
        assert!(job.succeed(&reading(), usage, Utc::now()).is_err());
        assert_eq!(
            job.status(),
            AiJobStatus::Running,
            "lo stato non è cambiato"
        );
    }

    #[test]
    fn every_task_and_status_has_a_distinct_stable_name() {
        for task in [
            AiTask::BillOcr,
            AiTask::TicketTriage,
            AiTask::TicketReplyDraft,
        ] {
            assert_eq!(AiTask::from_db(task.as_str()), Some(task));
        }
        assert_eq!(AiTask::from_db("qualcosa_di_nuovo"), None);
    }
}
