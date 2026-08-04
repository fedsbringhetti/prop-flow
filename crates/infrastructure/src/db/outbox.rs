use async_trait::async_trait;

use propflow_domain::events::DomainEvent;
use propflow_domain::ports::{EventSink, RepositoryError, RepositoryResult};

use super::error::map_sqlx_error;
use super::uow::PgUnitOfWork;

/// Outbox transazionale.
///
/// L'evento entra nella stessa transazione del comando che lo ha prodotto: se il
/// commit fallisce non resta né la modifica né l'annuncio, e se riesce esistono
/// entrambi. È la proprietà che rende superfluo un two-phase commit verso il broker.
#[async_trait]
impl EventSink for PgUnitOfWork {
    async fn append(&mut self, event: &DomainEvent) -> RepositoryResult<()> {
        let payload =
            serde_json::to_value(event.payload()).map_err(|error| RepositoryError::Backend {
                details: format!("serializzazione del payload evento fallita: {error}"),
            })?;
        let metadata =
            serde_json::to_value(event.metadata()).map_err(|error| RepositoryError::Backend {
                details: format!("serializzazione dei metadati evento fallita: {error}"),
            })?;

        sqlx::query(
            "INSERT INTO app.domain_events (
                 id, org_id, aggregate_type, aggregate_id, event_type, event_version,
                 payload, metadata, occurred_at
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(event.id().as_uuid())
        .bind(event.org_id().map(|id| id.as_uuid()))
        .bind(event.aggregate_type())
        .bind(event.aggregate_id())
        .bind(event.event_type())
        .bind(1_i16)
        .bind(payload)
        .bind(metadata)
        .bind(event.occurred_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }
}
