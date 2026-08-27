use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::Duration;
use propflow_api::config::AppConfig;
use propflow_api::state::{AppState, Dependencies};
use propflow_infrastructure::db::PoolSettings;
use propflow_infrastructure::security::{
    Argon2PasswordHasher, EdDsaTokenIssuer, JwtSettings, OpaqueRefreshTokenFactory,
    OpaqueTokenGenerator,
};
use propflow_infrastructure::storage::{S3ObjectStorage, S3Settings};
use propflow_infrastructure::telemetry::{self, TelemetrySettings};
use propflow_infrastructure::{DatabasePools, PgUnitOfWorkFactory, SystemClock};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load().context("configurazione non valida")?;

    let _telemetry = telemetry::init(&TelemetrySettings {
        service_name: config.telemetry.service_name.clone(),
        otlp_endpoint: config.telemetry.otlp_endpoint.clone(),
        filter: config.telemetry.filter.clone(),
        json: config.telemetry.json_logs,
    })
    .context("inizializzazione della telemetria non riuscita")?;

    // Le chiavi si leggono da file: in Kubernetes arrivano come Secret montati, e
    // una chiave privata in una variabile d'ambiente è leggibile da `/proc`, dai
    // dump di crash e da `docker inspect`.
    let private_key_pem =
        std::fs::read_to_string(&config.auth.private_key_path).with_context(|| {
            format!(
                "chiave privata non leggibile: {:?}",
                config.auth.private_key_path
            )
        })?;
    let public_key_pem =
        std::fs::read_to_string(&config.auth.public_key_path).with_context(|| {
            format!(
                "chiave pubblica non leggibile: {:?}",
                config.auth.public_key_path
            )
        })?;

    let pool_settings = PoolSettings {
        max_connections: config.database.max_connections,
        statement_timeout: std::time::Duration::from_secs(
            config.database.statement_timeout_seconds,
        ),
        ..PoolSettings::default()
    };
    let pools = DatabasePools::connect(
        &config.database.application_url,
        &config.database.privileged_url,
        &pool_settings,
    )
    .await
    .context("connessione al database non riuscita")?;

    // Fail fast: se il database non risponde all'avvio è meglio non entrare mai in
    // servizio, invece di accettare traffico e restituire 500.
    pools.ping().await.context("database non raggiungibile")?;

    // Le migration NON girano qui: l'API si connette come `propflow_app`, che non
    // ha privilegi DDL. Le applica `propflow-migrator` come `propflow_migrator`,
    // da un job separato.
    let dependencies = Dependencies {
        unit_of_work: Arc::new(PgUnitOfWorkFactory::new(pools.clone())),
        hasher: Arc::new(
            Argon2PasswordHasher::new().context("inizializzazione di Argon2 non riuscita")?,
        ),
        tokens: Arc::new(
            EdDsaTokenIssuer::new(&JwtSettings {
                private_key_pem,
                public_key_pem,
                issuer: config.auth.issuer.clone(),
                audience: config.auth.audience.clone(),
                access_token_lifetime: Duration::seconds(config.auth.access_token_ttl_seconds),
            })
            .context("chiavi JWT non valide")?,
        ),
        refresh_tokens: Arc::new(OpaqueRefreshTokenFactory::new(Duration::days(
            config.auth.refresh_token_ttl_days,
        ))),
        invitation_tokens: Arc::new(OpaqueTokenGenerator),
        clock: Arc::new(SystemClock),
        // Costruito qui e non alla prima richiesta: un endpoint sbagliato o un
        // bucket inesistente devono impedire l'avvio, non far fallire il primo
        // caricamento di un utente.
        object_storage: Arc::new(
            S3ObjectStorage::new(&S3Settings {
                endpoint: config.storage.endpoint.clone(),
                region: config.storage.region.clone(),
                bucket: config.storage.bucket.clone(),
                access_key_id: config.storage.access_key_id.clone(),
                secret_access_key: config.storage.secret_access_key.clone(),
                path_style: config.storage.path_style,
                request_timeout: std::time::Duration::from_secs(
                    config.storage.request_timeout_seconds,
                ),
            })
            .context("configurazione dello storage oggetti non valida")?,
        ),
        presign_ttl_seconds: config.storage.presign_ttl_seconds,
        pools: pools.clone(),
    };

    let state = AppState::new(dependencies);
    let app = propflow_api::router(
        state,
        config.server.body_limit_bytes,
        config.server.upload_body_limit_bytes,
        config.request_timeout(),
    );

    let listener = tokio::net::TcpListener::bind(&config.server.bind)
        .await
        .with_context(|| format!("bind su {} non riuscito", config.server.bind))?;
    let local = listener
        .local_addr()
        .context("indirizzo locale non disponibile")?;
    tracing::info!(
        address = %local,
        version = env!("CARGO_PKG_VERSION"),
        "API in ascolto"
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("il server si è interrotto")?;

    // Chiusura ordinata: le connessioni tornano al database invece di restare
    // appese fino al timeout lato server.
    pools.close().await;
    tracing::info!("arresto completato");
    Ok(())
}

/// Attende SIGINT o SIGTERM. SIGTERM è quello che manda Kubernetes: ignorarlo
/// significa farsi uccidere a forza dopo il grace period, interrompendo le
/// richieste in volo.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(error) => tracing::error!(error = %error, "impossibile registrare SIGTERM"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => tracing::info!("ricevuto SIGINT"),
        () = terminate => tracing::info!("ricevuto SIGTERM"),
    }
}
