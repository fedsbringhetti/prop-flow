use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// Parametri di connessione.
#[derive(Debug, Clone)]
pub struct PoolSettings {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
    /// Timeout applicato a ogni statement lato server.
    pub statement_timeout: Duration,
}

impl Default for PoolSettings {
    fn default() -> Self {
        Self {
            max_connections: 16,
            min_connections: 1,
            acquire_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(300),
            statement_timeout: Duration::from_secs(10),
        }
    }
}

/// I due pool dell'applicazione.
///
/// Sono separati perché usano ruoli di database diversi, e la separazione è il
/// meccanismo di sicurezza: `application` si connette come `propflow_app`, che è
/// soggetto alla Row Level Security e non ha il privilegio `DELETE`.
/// `privileged` usa `propflow_worker`, che ha `BYPASSRLS`, e serve solo dove un
/// contesto di sicurezza non può esistere — registrazione, login, rotazione dei
/// refresh token, dispatcher dell'outbox.
///
/// Tenerli distinti significa che un uso accidentale del pool privilegiato è
/// visibile nel codice come `begin_privileged()`, non nascosto in una connessione
/// che ha sempre tutti i permessi.
#[derive(Debug, Clone)]
pub struct DatabasePools {
    application: PgPool,
    privileged: PgPool,
}

impl DatabasePools {
    /// Apre i due pool. Il `statement_timeout` è impostato per connessione: una
    /// query che va in tavola non deve poter tenere occupata una connessione
    /// all'infinito.
    pub async fn connect(
        application_url: &str,
        privileged_url: &str,
        settings: &PoolSettings,
    ) -> Result<Self, sqlx::Error> {
        let application = Self::build(application_url, settings).await?;
        let privileged = Self::build(privileged_url, settings).await?;
        Ok(Self {
            application,
            privileged,
        })
    }

    async fn build(url: &str, settings: &PoolSettings) -> Result<PgPool, sqlx::Error> {
        let statement_timeout_ms = settings.statement_timeout.as_millis();
        PgPoolOptions::new()
            .max_connections(settings.max_connections)
            .min_connections(settings.min_connections)
            .acquire_timeout(settings.acquire_timeout)
            .idle_timeout(Some(settings.idle_timeout))
            .after_connect(move |connection, _meta| {
                Box::pin(async move {
                    // `set_config` e non `SET`: accetta un parametro, quindi il
                    // valore non viene interpolato in una stringa SQL.
                    sqlx::query("SELECT set_config('statement_timeout', $1, false)")
                        .bind(statement_timeout_ms.to_string())
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect(url)
            .await
    }

    #[must_use]
    pub const fn application(&self) -> &PgPool {
        &self.application
    }

    #[must_use]
    pub const fn privileged(&self) -> &PgPool {
        &self.privileged
    }

    /// Verifica di prontezza: usata dall'endpoint `/ready` e all'avvio.
    pub async fn ping(&self) -> Result<(), sqlx::Error> {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.application)
            .await
            .map(|_| ())
    }

    pub async fn close(&self) {
        self.application.close().await;
        self.privileged.close().await;
    }
}
