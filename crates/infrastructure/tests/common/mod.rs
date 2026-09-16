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

use chrono::Duration;
use propflow_application::ports::{
    OpaqueTokenFactory, PasswordHasher, RefreshTokenFactory, TokenIssuer,
};
use propflow_domain::ports::UnitOfWorkFactory;
use propflow_infrastructure::db::PoolSettings;
use propflow_infrastructure::security::{
    Argon2PasswordHasher, EdDsaTokenIssuer, JwtSettings, OpaqueRefreshTokenFactory,
    OpaqueTokenGenerator,
};
use propflow_infrastructure::{DatabasePools, PgUnitOfWorkFactory, SystemClock};

pub const APP_URL_VAR: &str = "PROPFLOW_TEST_APP_URL";
pub const PRIVILEGED_URL_VAR: &str = "PROPFLOW_TEST_PRIVILEGED_URL";
pub const JWT_PRIVATE_KEY_VAR: &str = "PROPFLOW_TEST_JWT_PRIVATE_KEY";
pub const JWT_PUBLIC_KEY_VAR: &str = "PROPFLOW_TEST_JWT_PUBLIC_KEY";

pub struct TestEnv {
    pub pools: DatabasePools,
    pub factory: Arc<dyn UnitOfWorkFactory>,
    pub hasher: Arc<dyn PasswordHasher>,
    pub clock: Arc<SystemClock>,
    /// Servono all'helper `register_owner` di `persistence.rs`, che passa da
    /// `AcceptOwnerInvitation` (ADR-053): quel caso d'uso emette sempre una
    /// sessione completa, come farebbe la rotta vera, e per farlo ha bisogno
    /// di firmare un access token e generare un refresh token.
    pub tokens: Arc<dyn TokenIssuer>,
    pub refresh_tokens: Arc<dyn RefreshTokenFactory>,
    pub invitation_tokens: Arc<dyn OpaqueTokenFactory>,
}

/// Prepara l'ambiente, oppure `None` se il database di test non è configurato.
pub async fn setup() -> Option<TestEnv> {
    let (Ok(app_url), Ok(privileged_url), Ok(private_key), Ok(public_key)) = (
        std::env::var(APP_URL_VAR),
        std::env::var(PRIVILEGED_URL_VAR),
        std::env::var(JWT_PRIVATE_KEY_VAR),
        std::env::var(JWT_PUBLIC_KEY_VAR),
    ) else {
        eprintln!(
            "SALTATO: impostare {APP_URL_VAR}, {PRIVILEGED_URL_VAR}, {JWT_PRIVATE_KEY_VAR} e \
             {JWT_PUBLIC_KEY_VAR} (vedi ops/ci/test-integration.sh)"
        );
        return None;
    };

    let pools = DatabasePools::connect(&app_url, &privileged_url, &PoolSettings::default())
        .await
        .expect("connessione al database di test");

    let tokens: Arc<dyn TokenIssuer> = Arc::new(
        EdDsaTokenIssuer::new(&JwtSettings {
            private_key_pem: std::fs::read_to_string(private_key).expect("chiave privata"),
            public_key_pem: std::fs::read_to_string(public_key).expect("chiave pubblica"),
            issuer: "propflow".to_owned(),
            audience: "propflow-api".to_owned(),
            access_token_lifetime: Duration::minutes(15),
        })
        .expect("chiavi valide"),
    );

    Some(TestEnv {
        factory: Arc::new(PgUnitOfWorkFactory::new(pools.clone())),
        hasher: Arc::new(Argon2PasswordHasher::new().expect("Argon2 inizializzato")),
        clock: Arc::new(SystemClock),
        tokens,
        refresh_tokens: Arc::new(OpaqueRefreshTokenFactory::new(Duration::days(30))),
        invitation_tokens: Arc::new(OpaqueTokenGenerator),
        pools,
    })
}

/// Scrive un invito da proprietario **come farà lo script**
/// (`ops/dev/issue-owner-invitation.sh`), con il ruolo privilegiato.
///
/// Copiata da `crates/api/tests/common::issue_owner_invitation`: i due
/// `common` sono binari di test di crate diverse e non condividono un crate di
/// supporto, quindi la forma è duplicata invece che astratta in una
/// dipendenza nuova.
pub async fn issue_owner_invitation(pools: &DatabasePools, email: &str) -> String {
    let token = OpaqueTokenGenerator.generate().expect("token generato");
    let now = chrono::Utc::now();
    let expires_at = now + Duration::days(7);

    sqlx::query(
        "INSERT INTO app.owner_invitations (email, token_hash, expires_at, created_at)
         VALUES ($1::app.email_address, $2, $3, $4)",
    )
    .bind(email)
    .bind(token.hash.as_bytes().as_slice())
    .bind(expires_at)
    .bind(now)
    .execute(pools.privileged())
    .await
    .expect("invito da proprietario inserito");

    token.plaintext
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

/// Chiave dell'advisory lock che serializza i test dell'OCR.
///
/// Arbitraria e condivisa fra i due binari di test: deve solo non collidere con
/// altri lock nello stesso database.
const OCR_QUEUE_LOCK: i64 = 918_273_645;

/// Prende in esclusiva la coda dei job AI per la durata del test.
///
/// **Serve perché la coda è globale, e lo è di proposito.** `claim_next` non
/// filtra per organizzazione — il worker serve tutte, e deve — mentre ogni test
/// ha il **proprio** doppio dello storage oggetti. Due test dell'OCR in parallelo
/// sullo stesso database producono quindi il difetto peggiore possibile: il worker
/// dell'uno prende il job dell'altro, non trova l'oggetto nel proprio bucket, e lo
/// chiude `failed` con `document_unavailable`. Non è un difetto del codice — in
/// produzione c'è un solo storage — ma della suite, che simula una configurazione
/// che non può esistere.
///
/// L'invariante da rispettare è: **accodare e smaltire dentro la stessa sezione
/// critica**, così quando il lock si libera non restano job in coda per il worker
/// del test successivo.
///
/// `pg_advisory_xact_lock` e non quello di sessione: si rilascia con la
/// transazione, quindi anche se il test va in panico. Un lock di sessione su una
/// connessione che torna al pool resterebbe preso, e la suite si bloccherebbe.
pub struct OcrQueueGate(#[allow(dead_code)] sqlx::Transaction<'static, sqlx::Postgres>);

pub async fn lock_ocr_queue(pools: &DatabasePools) -> OcrQueueGate {
    let mut tx = pools
        .privileged()
        .begin()
        .await
        .expect("transazione per il lock");
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(OCR_QUEUE_LOCK)
        .execute(&mut *tx)
        .await
        .expect("lock sulla coda dei job");
    OcrQueueGate(tx)
}
