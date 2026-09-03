use async_trait::async_trait;

use propflow_domain::ai::{OcrReading, ProviderUsage};

/// Errori della lettura automatica di un documento.
///
/// Due varianti e non una, perché i due rimedi sono opposti: un documento
/// rifiutato si ricarica in un altro formato, un fornitore che non risponde si
/// aspetta. Diventano due `error_code` distinti sul job — `unsupported_document`
/// e `provider_unavailable` — e quella distinzione è ciò che permetterà di
/// decidere il ritentativo automatico, che è fuori perimetro proprio perché
/// richiede di sapere quali errori un fornitore vero produce.
///
/// **Nessuna variante per la confidenza bassa.** Una lettura di cui il fornitore
/// si fida poco è un esito riuscito: il job va a `succeeded` e la bolletta arriva
/// comunque a `pending_review`, dove una persona guarda. Trasformarla in errore
/// significherebbe scartare letture buone in base a una soglia che nessuno ha
/// ancora potuto misurare.
#[derive(Debug, Clone, thiserror::Error)]
pub enum OcrError {
    /// Il fornitore non sa leggere questo documento: formato che non gestisce,
    /// scansione illeggibile, pagine che non riconosce.
    #[error("documento non leggibile dal fornitore: {details}")]
    UnsupportedDocument { details: String },
    /// Il fornitore non ha risposto, o ha risposto un errore suo.
    #[error("fornitore OCR non disponibile: {details}")]
    Backend { details: String },
}

/// Ciò che un fornitore restituisce su un documento letto.
///
/// Due parti che vanno in due posti diversi, e tenerle in un tipo solo è ciò che
/// impedisce di confonderle: la [lettura](OcrReading) finisce sulla **bolletta**
/// (`ocr_extracted`, `ocr_confidence`), i [contatori](ProviderUsage) finiscono sul
/// **job** (`model`, `tokens_*`, `cost_amount`, `latency_ms`).
#[derive(Debug, Clone)]
pub struct OcrOutcome {
    pub reading: OcrReading,
    pub usage: ProviderUsage,
}

/// Lettura automatica di una bolletta.
///
/// **Dichiarata prima che il fornitore sia scelto**, come `ObjectStorage` e
/// `Notifier` prima di lei, e questa volta la scelta non è solo rimandata per
/// comodità: mandare l'immagine di una bolletta a un terzo — fornitore,
/// indirizzo, importi, spesso il codice cliente — è una decisione di trattamento
/// dei dati, e la prende chi risponde del trattamento, non chi scrive
/// l'adattatore. Il ragionamento completo, con le due strade scartate (un LLM con
/// visione, Tesseract in locale), sta in `docs/09-stato-e-roadmap.md`.
///
/// **Il rischio di disegnare una porta senza un fornitore davanti va nominato**,
/// perché è reale: si rischia di descrivere ciò che è comodo a noi invece di ciò
/// che i fornitori danno. Si mitiga così, e nessuna delle due cose lo elimina:
///
/// - **la porta chiede il minimo comune**. I byte con il loro MIME in ingresso;
///   in uscita i campi trovati, ciascuno con la propria confidenza, più i
///   contatori che `ai_jobs` ha già. Tutto ciò che un fornitore specifico offre
///   in più — bounding box, pagine, alternative — resta fuori: aggiungerlo dopo è
///   additivo, toglierlo no;
/// - **il primo adattatore vero è il momento di rileggerla**, ed è scritto qui
///   perché chi lo scriverà lo trovi. Se la porta si rivelasse sbagliata,
///   cambiarla allora costa un adattatore finto e nessun client.
///
/// Il nome del fornitore è un **dato** e non un ramo del codice: chi implementa
/// dichiara come si chiama con [`OcrProvider::name`], e quel valore finisce in
/// `ai_jobs.provider_name`.
#[async_trait]
pub trait OcrProvider: Send + Sync {
    /// Come il fornitore si chiama in `ai_jobs.provider_name`.
    ///
    /// Sta sulla porta e non nella configurazione perché è una proprietà
    /// dell'adattatore, non una scelta di chi lo installa: due deployment dello
    /// stesso fornitore devono produrre righe confrontabili.
    fn name(&self) -> &str;

    /// Legge un documento e propone i campi che ha trovato.
    ///
    /// `content` sono i byte già in memoria, come per `ObjectStorage::put`: una
    /// bolletta sta nel limite di corpo delle rotte di caricamento, e lo streaming
    /// verso un fornitore è una complicazione che si compra quando serve.
    async fn read(&self, content: &[u8], mime_type: &str) -> Result<OcrOutcome, OcrError>;
}
