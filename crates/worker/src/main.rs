//! Dispatcher dell'outbox: legge `app.domain_events` e pubblica sul bus.
//!
//! È il pezzo che rende l'Event Bus "predisposto" e non solo dichiarato. Il
//! broker non è nominato da nessuna parte: c'è un trait [`EventBus`] con una sola
//! implementazione che scrive nei log. Sostituirla con NATS, Kafka o RabbitMQ è
//! aggiungere un file, non riscrivere il ciclo.
//!
//! Gira come `propflow_worker` (BYPASSRLS): deve vedere gli eventi di tutte le
//! organizzazioni, quindi non può essere soggetto alle policy di riga.

use std::time::Duration;

use anyhow::{Context, Result};
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
