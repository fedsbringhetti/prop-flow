//! Supporto ai test di integrazione.
//!
//! I test richiedono un PostgreSQL reale con lo schema applicato e i tre ruoli
//! creati: verificare la Row Level Security contro un finto database non
//! verificherebbe nulla, perché la RLS *è* il database.
//!
//! Senza le variabili d'ambiente i test si dichiarano saltati invece di fallire,
//! così `cargo test` resta eseguibile su una macchina senza Docker. Il runner
//! `ops/ci/test-integration.sh` le imposta.

use std::sync::Arc;

use propflow_application::ports::PasswordHasher;
use propflow_domain::ports::UnitOfWorkFactory;
use propflow_infrastructure::db::PoolSettings;
use propflow_infrastructure::security::Argon2PasswordHasher;
use propflow_infrastructure::{DatabasePools, PgUnitOfWorkFactory, SystemClock};

pub const APP_URL_VAR: &str = "PROPFLOW_TEST_APP_URL";
pub const PRIVILEGED_URL_VAR: &str = "PROPFLOW_TEST_PRIVILEGED_URL";

pub struct TestEnv {
    pub pools: DatabasePools,
    pub factory: Arc<dyn UnitOfWorkFactory>,
    pub hasher: Arc<dyn PasswordHasher>,
    pub clock: Arc<SystemClock>,
}

/// Prepara l'ambiente, oppure `None` se il database di test non è configurato.
pub async fn setup() -> Option<TestEnv> {
    let (Ok(app_url), Ok(privileged_url)) = (
        std::env::var(APP_URL_VAR),
        std::env::var(PRIVILEGED_URL_VAR),
    ) else {
        eprintln!(
            "SALTATO: impostare {APP_URL_VAR} e {PRIVILEGED_URL_VAR} \
             (vedi ops/ci/test-integration.sh)"
        );
        return None;
    };

    let pools = DatabasePools::connect(&app_url, &privileged_url, &PoolSettings::default())
        .await
        .expect("connessione al database di test");

    Some(TestEnv {
        factory: Arc::new(PgUnitOfWorkFactory::new(pools.clone())),
        hasher: Arc::new(Argon2PasswordHasher::new().expect("Argon2 inizializzato")),
        clock: Arc::new(SystemClock),
        pools,
    })
}

/// Email irripetibile: i test girano sullo stesso database, e un vincolo unique
/// non deve far fallire il secondo test solo perché il primo è passato.
pub fn unique_email(prefix: &str) -> String {
    format!("{prefix}-{}@example.test", uuid::Uuid::now_v7().simple())
}

/// Macro di uscita anticipata quando il database di test non è configurato.
#[macro_export]
macro_rules! require_db {
    () => {
        match common::setup().await {
            Some(env) => env,
            None => return,
        }
    };
}
