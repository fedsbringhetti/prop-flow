//! Il worker di PropFlow: **due cicli in un processo solo**.
//!
//! | Ciclo | Legge | Fa |
//! |---|---|---|
//! | dispatcher dell'outbox | `app.domain_events` | pubblica sul bus |
//! | lettura automatica | `app.ai_jobs` | legge la scansione e porta la bolletta in revisione |
//!
//! Entrambi girano come `propflow_worker` (BYPASSRLS): devono vedere le righe di
//! tutte le organizzazioni, quindi non possono essere soggetti alle policy di riga.
//!
//! # Perché un processo e non due binari
//!
//! La ragione per due binari era buona: un job di OCR lungo affamerebbe il
//! dispatcher dell'outbox. **Quel carico però non esiste in questa fetta** —
//! l'adattatore è deterministico e la scelta del fornitore vero è rimandata — e
//! tararsi su un'ipotesi costa un'immagine Docker, un servizio nello stack compose,
//! una voce in `docs/07-operazioni.md` e una modifica a `release.yml`.
//!
//! I due cicli girano quindi in due `tokio::spawn` dello stesso processo. La scelta
//! è **reversibile a poco prezzo**: il ciclo dell'OCR sta dietro `ProcessAiJob`,
//! che dipende solo da porte, quindi spostarlo in un binario suo non tocca né il
//! dominio né gli use case. **Il momento di rifarsi la domanda è il primo
//! adattatore vero**, che è anche il momento in cui rileggere la porta `OcrProvider`:
//! sono la stessa conversazione.
//!
//! Se uno dei due cicli termina — cosa che oggi succede solo se il suo `Result`
//! esce dal loop infinito — il processo esce. Un worker che resta su a metà
//! sarebbe peggio: l'orchestratore non lo riavvierebbe, e metà del lavoro
//! smetterebbe di essere fatto in silenzio.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use propflow_application::use_cases::ai::{ProcessAiJob, Processed};
use propflow_domain::ai::AiTask;
use propflow_infrastructure::db::PgWorkerUnitOfWorkFactory;
use propflow_infrastructure::storage::{S3ObjectStorage, S3Settings};
use propflow_infrastructure::{DeterministicOcrProvider, SystemClock};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Evento pronto per la pubblicazione.
#[derive(Debug)]
struct OutboxEvent {
    id: Uuid,
    sequence: i64,
    event_type: String,
    aggregate_type: String,
    aggregate_id: Uuid,
    payload: serde_json::Value,
}

/// Destinazione degli eventi di dominio.
trait EventBus: Send + Sync {
    fn publish(&self, event: &OutboxEvent) -> Result<()>;
}

/// Implementazione di partenza: registra l'evento nei log strutturati.
///
/// Non è un segnaposto inutile — rende osservabile il flusso degli eventi da
/// subito, e permette di verificare che l'outbox funzioni prima di introdurre un
/// broker con la sua infrastruttura da gestire.
struct LoggingEventBus;

impl EventBus for LoggingEventBus {
    fn publish(&self, event: &OutboxEvent) -> Result<()> {
        tracing::info!(
            event_id = %event.id,
            sequence = event.sequence,
            event_type = %event.event_type,
            aggregate = %format_args!("{}:{}", event.aggregate_type, event.aggregate_id),
            "evento di dominio pubblicato"
        );
        // Il payload solo a livello debug: gli eventi non trasportano dati
        // personali per costruzione, ma restano rumore nei log di produzione.
        tracing::debug!(event_id = %event.id, payload = %event.payload, "payload dell'evento");
        Ok(())
    }
}

/// Quanti eventi prendere per ciclo.
const BATCH_SIZE: i64 = 64;
/// Attesa quando la coda è vuota.
const IDLE_POLL: Duration = Duration::from_secs(2);
/// Tentativi prima di dichiarare l'evento non recapitabile.
const MAX_ATTEMPTS: i16 = 10;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let url = std::env::var("PROPFLOW__DATABASE__PRIVILEGED_URL")
        .context("PROPFLOW__DATABASE__PRIVILEGED_URL non impostata")?;
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .context("connessione al database non riuscita")?;

    // Lo storage si costruisce **all'avvio** e non alla prima lettura, come
    // nell'API: un endpoint sbagliato o un bucket inesistente devono impedire al
    // worker di partire, non far fallire il primo job e lasciarne una scia di
    // `failed` che sembrano colpa del fornitore di OCR.
    let ocr = ProcessAiJob::new(
        Arc::new(PgWorkerUnitOfWorkFactory::new(pool.clone())),
        Arc::new(
            S3ObjectStorage::new(&storage_settings()?)
                .context("configurazione dello storage oggetti non valida")?,
        ),
        Arc::new(DeterministicOcrProvider),
        Arc::new(SystemClock),
    );

    let outbox_pool = pool.clone();
    let outbox = tokio::spawn(async move { dispatch_forever(outbox_pool).await });
    let reader = tokio::spawn(async move { read_bills_forever(ocr).await });

    // `try_join!` e non due `await` in fila: se uno dei due cicli esce, il
    // processo deve uscire subito. Aspettare l'altro significherebbe un worker
    // vivo che fa metà del lavoro, e nessun orchestratore lo riavvierebbe perché
    // da fuori è ancora su.
    tokio::try_join!(flatten(outbox), flatten(reader))?;
    Ok(())
}

/// Un task che esce è un errore, qualunque cosa abbia restituito.
///
/// I due cicli sono infiniti per costruzione: se uno ritorna, o il suo `Result` è
/// un errore, o è successo qualcosa che il codice non prevede. In entrambi i casi
/// il processo deve terminare.
async fn flatten(handle: tokio::task::JoinHandle<Result<()>>) -> Result<()> {
    match handle.await {
        Ok(result) => result,
        Err(error) => Err(anyhow::anyhow!("un ciclo del worker è terminato: {error}")),
    }
}

/// Le impostazioni dello storage, dalle stesse variabili che legge l'API.
///
/// **Le stesse e non altre**: il worker legge gli oggetti che l'API ha scritto, e
/// due configurazioni separate significherebbero due bucket che divergono al primo
/// deploy — con il sintomo peggiore possibile, cioè job che falliscono
/// `document_unavailable` su documenti che l'API mostra benissimo.
///
/// Lette con `std::env::var` e non da `AppConfig`, che vive nel crate `api` e
/// porterebbe con sé server, JWT e telemetria: il worker non ha un bind né chiavi
/// di firma, e dipendere da una struct che li pretende significherebbe dargli una
/// configurazione che non usa.
fn storage_settings() -> Result<S3Settings> {
    let required =
        |name: &str| std::env::var(name).with_context(|| format!("{name} non impostata"));
    Ok(S3Settings {
        endpoint: required("PROPFLOW__STORAGE__ENDPOINT")?,
        region: std::env::var("PROPFLOW__STORAGE__REGION")
            .unwrap_or_else(|_| "us-east-1".to_owned()),
        bucket: std::env::var("PROPFLOW__STORAGE__BUCKET")
            .unwrap_or_else(|_| "propflow-documents".to_owned()),
        access_key_id: required("PROPFLOW__STORAGE__ACCESS_KEY_ID")?,
        secret_access_key: required("PROPFLOW__STORAGE__SECRET_ACCESS_KEY")?,
        path_style: std::env::var("PROPFLOW__STORAGE__PATH_STYLE")
            .map(|value| value != "false")
            .unwrap_or(true),
        request_timeout: Duration::from_secs(
            std::env::var("PROPFLOW__STORAGE__REQUEST_TIMEOUT_SECONDS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(30),
        ),
    })
}

/// Il ciclo del dispatcher dell'outbox.
async fn dispatch_forever(pool: PgPool) -> Result<()> {
    let bus = LoggingEventBus;
    tracing::info!("dispatcher dell'outbox avviato");

    loop {
        match dispatch_batch(&pool, &bus).await {
            Ok(0) => tokio::time::sleep(IDLE_POLL).await,
            Ok(count) => tracing::debug!(count, "batch di eventi pubblicato"),
            Err(error) => {
                // Un errore del ciclo non deve fermare il worker: si aspetta e si
                // riprova, altrimenti un'indisponibilità momentanea del database
                // richiede un riavvio manuale.
                tracing::error!(error = %error, "ciclo del dispatcher fallito, nuovo tentativo");
                tokio::time::sleep(IDLE_POLL).await;
            }
        }
    }
}

/// Il ciclo della lettura automatica delle bollette.
///
/// **Un job per giro e non un lotto**, al contrario del dispatcher: un evento si
/// pubblica in microsecondi e prenderne sessantaquattro alla volta risparmia
/// sessantatré transazioni, mentre una lettura chiama un fornitore esterno — e un
/// lotto significherebbe tenere il lock su tutti i job del lotto per la somma delle
/// loro durate, cioè esattamente ciò che `SKIP LOCKED` esiste per evitare.
async fn read_bills_forever(ocr: ProcessAiJob) -> Result<()> {
    tracing::info!("lettura automatica delle bollette avviata");

    loop {
        match ocr.execute(AiTask::BillOcr).await {
            // Coda vuota: si dorme. Con lavoro in coda si riparte subito, così una
            // fila di scansioni non si smaltisce a due secondi l'una.
            Ok(Processed::Nothing) => tokio::time::sleep(IDLE_POLL).await,
            // Un job `failed` **non è un errore del ciclo**: è un esito registrato
            // sul job, e il worker continua. Il log lo scrive già `ProcessAiJob`.
            Ok(Processed::Succeeded | Processed::Failed) => {}
            Err(error) => {
                // Qui invece il ciclo non ha potuto nemmeno registrare l'esito: il
                // database non risponde, o una nostra invariante è saltata. Stessa
                // scelta del dispatcher — si aspetta e si riprova, invece di
                // richiedere un riavvio manuale.
                tracing::error!(error = %error, "ciclo della lettura automatica fallito, nuovo tentativo");
                tokio::time::sleep(IDLE_POLL).await;
            }
        }
    }
}

/// Preleva un lotto di eventi e li pubblica.
///
/// `FOR UPDATE SKIP LOCKED` permette a più repliche del worker di consumare la
/// stessa coda senza coordinamento esterno: chi arriva secondo salta le righe già
/// bloccate invece di aspettarle.
async fn dispatch_batch(pool: &PgPool, bus: &dyn EventBus) -> Result<usize> {
    let mut tx = pool.begin().await?;

    let rows = sqlx::query(
        "SELECT id, sequence, event_type, aggregate_type, aggregate_id, payload, attempts
           FROM app.domain_events
          WHERE status = ANY (ARRAY['pending','failed']::app.event_status[])
            AND available_at <= now()
          ORDER BY available_at, sequence
          LIMIT $1
          FOR UPDATE SKIP LOCKED",
    )
    .bind(BATCH_SIZE)
    .fetch_all(&mut *tx)
    .await?;

    if rows.is_empty() {
        tx.rollback().await?;
        return Ok(0);
    }

    let mut published = 0_usize;
    for row in &rows {
        let event = OutboxEvent {
            id: row.try_get("id")?,
            sequence: row.try_get("sequence")?,
            event_type: row.try_get("event_type")?,
            aggregate_type: row.try_get("aggregate_type")?,
            aggregate_id: row.try_get("aggregate_id")?,
            payload: row.try_get("payload")?,
        };
        let attempts: i16 = row.try_get("attempts")?;

        match bus.publish(&event) {
            Ok(()) => {
                sqlx::query(
                    "UPDATE app.domain_events
                        SET status = 'dispatched'::app.event_status,
                            published_at = now(),
                            attempts = attempts + 1,
                            last_error = NULL
                      WHERE id = $1",
                )
                .bind(event.id)
                .execute(&mut *tx)
                .await?;
                published += 1;
            }
            Err(error) => {
                let next_attempt = attempts.saturating_add(1);
                // Backoff esponenziale limitato: un evento che fallisce sempre non
                // deve monopolizzare la coda, e dopo MAX_ATTEMPTS viene messo da
                // parte per un'analisi manuale invece di girare per sempre.
                let status = if next_attempt >= MAX_ATTEMPTS {
                    "discarded"
                } else {
                    "failed"
                };
                let backoff_seconds = i64::from(next_attempt).saturating_mul(30).min(3600);
                sqlx::query(
                    "UPDATE app.domain_events
                        SET status = $2::app.event_status,
                            attempts = attempts + 1,
                            last_error = $3,
                            available_at = now() + make_interval(secs => $4::double precision)
                      WHERE id = $1",
                )
                .bind(event.id)
                .bind(status)
                .bind(error.to_string())
                .bind(backoff_seconds as f64)
                .execute(&mut *tx)
                .await?;
                tracing::warn!(
                    event_id = %event.id,
                    attempts = next_attempt,
                    status,
                    error = %error,
                    "pubblicazione dell'evento non riuscita"
                );
            }
        }
    }

    tx.commit().await?;
    Ok(published)
}
