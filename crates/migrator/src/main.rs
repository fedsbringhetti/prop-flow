//! Applica le migration dello schema.
//!
//! Binario separato dall'API per una ragione di privilegi: l'API si connette come
//! `propflow_app`, che è soggetto alla Row Level Security e non ha permessi DDL.
//! Le migration girano come `propflow_migrator`, che è owner delle tabelle. Farle
//! eseguire dall'API richiederebbe di dare all'API privilegi che non deve avere,
//! e in un deploy multi-replica farebbe partire N migrazioni concorrenti.
//!
//! In Kubernetes va come initContainer o come Job prima del rollout.
//!
//! Le migration si leggono dal filesystem e **non** con la macro
//! `sqlx::migrate!`: la macro le incorpora a compile time e non invalida la cache
//! quando si aggiunge un file nuovo, quindi eseguire il binario dopo aver scritto
//! una migration applicherebbe in silenzio il set precedente. Leggerle a runtime
//! rende impossibile quella classe di errore; l'immagine Docker copia la
//! cartella `migrations/` accanto al binario.

use std::path::PathBuf;

use anyhow::{Context, Result};
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use sqlx::{ConnectOptions, Executor};
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let url = std::env::var("PROPFLOW__DATABASE__MIGRATOR_URL")
        .context("PROPFLOW__DATABASE__MIGRATOR_URL non impostata")?;
    let directory = std::env::var("PROPFLOW__DATABASE__MIGRATIONS_DIR")
        .map_or_else(|_| PathBuf::from("migrations"), PathBuf::from);

    let migrator = Migrator::new(directory.as_path())
        .await
        .with_context(|| format!("migration non leggibili da {}", directory.display()))?;

    // `search_path` fissato a `public` sulla connessione, e non lasciato al
    // default del ruolo.
    //
    // Il ruolo `propflow_migrator` ha `search_path = app, public`. SQLx crea la
    // propria tabella di bookkeeping con `CREATE TABLE IF NOT EXISTS
    // _sqlx_migrations`, senza qualificare lo schema: alla prima esecuzione lo
    // schema `app` non esiste ancora, quindi la tabella nasce in `public`; alla
    // seconda `app` esiste e ne nascerebbe una *seconda*, vuota. Il migratore
    // concluderebbe che nessuna migration è stata applicata e le riapplicherebbe
    // tutte, fallendo su `CREATE DOMAIN` — cioè al primo redeploy.
    //
    // Tutte le migration qualificano gli oggetti con `app.`, quindi un
    // `search_path` a `public` non cambia nulla per il loro contenuto.
    let options = sqlx::postgres::PgConnectOptions::from_str(&url)
        .context("URL del database non valido")?
        .options([("search_path", "public")])
        .disable_statement_logging();

    // Una sola connessione: `sqlx migrate` prende un advisory lock, quindi più
    // repliche in avvio contemporaneo si serializzano invece di collidere.
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _meta| {
            Box::pin(async move {
                connection.execute("SET search_path = public").await?;
                Ok(())
            })
        })
        .connect_with(options)
        .await
        .context("connessione al database non riuscita")?;

    let applied_before = migrator.iter().count();
    migrator
        .run(&pool)
        .await
        .context("applicazione delle migration non riuscita")?;

    tracing::info!(
        migrations = applied_before,
        directory = %directory.display(),
        "migration applicate"
    );
    pool.close().await;
    Ok(())
}
