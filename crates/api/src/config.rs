use std::path::PathBuf;
use std::time::Duration;

use figment::Figment;
use figment::providers::{Env, Format, Toml};

/// Configurazione dell'applicazione.
///
/// Si legge da `config/default.toml` (opzionale) e dalle variabili d'ambiente con
/// prefisso `PROPFLOW__`, dove il doppio underscore separa i livelli:
/// `PROPFLOW__DATABASE__APPLICATION_URL`. L'ambiente vince sul file, così un
/// deploy Kubernetes non ha bisogno di riscrivere il file.
///
/// Il percorso del file è **relativo alla directory di lavoro**: l'API va avviata
/// dalla radice del repository, o dall'immagine, dove sta in `/app/config`.
/// Altrove figment lo salta in silenzio — è dichiarato opzionale — e valgono i
/// default qui sotto. Non c'è un avviso perché la configurazione si carica prima
/// della telemetria: vedi `docs/08-decisioni.md#adr-023`.
///
/// `.env` non lo legge **questo processo**: lo carica chi lo avvia — `just`, che ha
/// `set dotenv-load`, o la shell con `set -a; source .env; set +a` per chi lancia il
/// binario a mano. Stesso ADR: resta un file per l'ambiente, non una sorgente di
/// configurazione.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub storage: StorageConfig,
    pub mail: MailConfig,
    #[serde(default)]
    pub telemetry: TelemetryConfig,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind")]
    pub bind: String,
    /// Timeout complessivo di una richiesta.
    #[serde(default = "default_request_timeout_seconds")]
    pub request_timeout_seconds: u64,
    /// Dimensione massima del corpo di una richiesta, in byte.
    #[serde(default = "default_body_limit_bytes")]
    pub body_limit_bytes: usize,
    /// Dimensione massima del corpo sulle rotte di caricamento dei documenti.
    ///
    /// Esiste separata perché il limite generale è montato sull'**intero** router
    /// e non ammette deroghe per rotta: il layer esterno rifiuta per primo, e un
    /// limite per rotta più largo non verrebbe mai raggiunto. Alzare quello
    /// generale funzionerebbe, ed è la via sbagliata — indebolirebbe ogni endpoint
    /// per servirne uno. Vedi `crates/api/src/lib.rs`.
    #[serde(default = "default_upload_body_limit_bytes")]
    pub upload_body_limit_bytes: usize,
    /// `Secure` sul cookie del refresh token.
    ///
    /// **L'unica impostazione che la fetta del client introduce**, e il default è
    /// `true` di proposito: un ambiente che dimentica di dichiararla ottiene il
    /// comportamento sicuro, non quello comodo. Va portata a `false` soltanto per
    /// lo sviluppo su `http://localhost`, dove il browser non manda un cookie che
    /// dichiara `Secure` — il refresh fallirebbe sempre e la sessione durerebbe i
    /// quindici minuti dell'access token, con un sintomo che non nomina la causa.
    #[serde(default = "default_secure_cookies")]
    pub secure_cookies: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DatabaseConfig {
    /// Connessione del servizio HTTP: ruolo soggetto alla Row Level Security.
    pub application_url: String,
    /// Connessione dei flussi pre-autenticazione e del worker: ruolo con BYPASSRLS.
    pub privileged_url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_statement_timeout_seconds")]
    pub statement_timeout_seconds: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AuthConfig {
    /// Percorso della chiave privata Ed25519 (PEM PKCS#8).
    ///
    /// Un percorso e non il contenuto: in Kubernetes le chiavi arrivano come
    /// Secret montati su file, e una chiave privata in una variabile d'ambiente
    /// finisce nei dump di `/proc`, nei log di crash e in `docker inspect`.
    pub private_key_path: PathBuf,
    pub public_key_path: PathBuf,
    #[serde(default = "default_issuer")]
    pub issuer: String,
    #[serde(default = "default_audience")]
    pub audience: String,
    /// Vita dell'access token: breve, perché non è revocabile.
    #[serde(default = "default_access_ttl_seconds")]
    pub access_token_ttl_seconds: i64,
    /// Vita del refresh token: lunga, ma revocabile e a rotazione singola.
    #[serde(default = "default_refresh_ttl_days")]
    pub refresh_token_ttl_days: i64,
}

/// Storage oggetti per i documenti.
///
/// Un solo adattatore, S3, che parla anche con MinIO: il fornitore è un **dato**
/// di configurazione e non un ramo del codice (`docs/06-sviluppo.md` §Aggiungere
/// un provider esterno). In sviluppo l'endpoint è il MinIO del compose, in
/// produzione quello di S3.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct StorageConfig {
    pub endpoint: String,
    #[serde(default = "default_storage_region")]
    pub region: String,
    #[serde(default = "default_storage_bucket")]
    pub bucket: String,
    /// Credenziali: **obbligatorie e dall'ambiente**, mai dal file. Il file è
    /// versionato, e una chiave nel repository è una chiave compromessa.
    pub access_key_id: String,
    pub secret_access_key: String,
    /// `true` mette il bucket nel percorso invece che nel sottodominio. Serve a
    /// MinIO e a ogni endpoint personalizzato, dove un DNS
    /// `<bucket>.<host>` non esiste.
    #[serde(default = "default_storage_path_style")]
    pub path_style: bool,
    /// Vita dell'URL pre-firmato di download.
    ///
    /// Corta: l'URL firmato è un permesso **trasferibile**, e chi lo intercetta lo
    /// può usare fino alla scadenza senza presentare un token.
    #[serde(default = "default_presign_ttl_seconds")]
    pub presign_ttl_seconds: u32,
    /// Timeout di una singola richiesta verso lo storage.
    #[serde(default = "default_storage_timeout_seconds")]
    pub request_timeout_seconds: u64,
}

/// Posta in uscita.
///
/// **Obbligatoria**, come le credenziali dello storage e per una ragione più
/// stringente. Da quando esiste il recupero della password la posta è l'**unico
/// canale** verso una persona chiusa fuori dal proprio account: un `Notifier` che
/// non facesse niente quando la configurazione manca produrrebbe il difetto
/// peggiore di tutti — una richiesta di recupero che risponde `202` e non manda
/// niente, su chi non ha nessun altro modo di rientrare.
///
/// Un'impostazione mancante ferma quindi **l'avvio**, che è la lezione già pagata
/// tre volte con le quattro `PROPFLOW__STORAGE__*`: il fallimento al primo uso
/// somiglia a un problema di rete invece che a una configurazione incompleta.
///
/// La configurazione ferma un relay **mal configurato**, non un relay *caduto*:
/// quello lo dice il controllo di prontezza, che interroga SMTP apposta.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MailConfig {
    /// Host del relay. In sviluppo è il raccoglitore del compose.
    pub host: String,
    #[serde(default = "default_mail_port")]
    pub port: u16,
    /// Utenza e password insieme, o nessuna delle due: il raccoglitore locale non
    /// autentica nessuno, un relay vero le vuole entrambe. Una sola delle due è
    /// quasi sempre una variabile dimenticata, e l'adattatore la rifiuta all'avvio.
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    /// Il mittente. Su un dominio senza SPF, DKIM e DMARC la posta parte e finisce
    /// nello spam, con un sintomo **muto**: nessun errore, il codice non compare.
    pub from_address: String,
    #[serde(default = "default_mail_from_name")]
    pub from_name: String,
    /// `starttls` (587), `tls` (465) o `none`. L'ultimo **solo** per il
    /// raccoglitore locale: un relay vero raggiunto in chiaro manda credenziali e
    /// destinatari sulla rete.
    #[serde(default = "default_mail_encryption")]
    pub encryption: MailEncryptionConfig,
    #[serde(default = "default_mail_timeout_seconds")]
    pub timeout_seconds: u64,
}

/// Il valore dell'impostazione, distinto dal tipo dell'infrastruttura: `config.rs`
/// non conosce `propflow-infrastructure`, e `main.rs` è il solo punto che traduce.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MailEncryptionConfig {
    StartTls,
    Tls,
    None,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TelemetryConfig {
    #[serde(default = "default_service_name")]
    pub service_name: String,
    #[serde(default)]
    pub otlp_endpoint: Option<String>,
    #[serde(default = "default_log_filter")]
    pub filter: String,
    #[serde(default)]
    pub json_logs: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: default_service_name(),
            otlp_endpoint: None,
            filter: default_log_filter(),
            json_logs: false,
        }
    }
}

/// Il rifiuto della configurazione, con **dove trovare ciò che manca**.
///
/// Esiste per un difetto che ha morso tre volte, sempre allo stesso punto:
/// avviare l'API per un giro a mano con un `.env` copiato prima della fetta dei
/// documenti. Mancano le quattro `PROPFLOW__STORAGE__*`, e figment risponde
/// `missing field `access_key_id` for key "default.storage"` — che è la
/// validazione che fa il suo mestiere, e non dice **da dove** debba arrivare
/// quella variabile né che `.env.example` la porti.
///
/// **Il rimedio non è un default**: uno storage configurato a caso è peggio di un
/// avvio fallito, perché il fallimento si sposterebbe al primo caricamento e
/// somiglierebbe a un problema di rete. Il rimedio è dire dove si copia.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Un'impostazione senza default e senza valore.
    ///
    /// Il messaggio di figment — quale campo e sotto quale chiave — arriva dalla
    /// catena delle cause e **non si ripete qui**: anyhow le stampa entrambe, e
    /// dirla due volte allontana il rimedio dalla riga che lo dà.
    #[error(
        "manca un'impostazione obbligatoria.\n         Le variabili hanno prefisso PROPFLOW__ e il doppio underscore separa i \
         livelli (PROPFLOW__STORAGE__ACCESS_KEY_ID).\n         L'elenco completo, con valori buoni per lo sviluppo locale, è in \
         .env.example — che il processo non carica da solo: lo fa `just` (che ha \
         set dotenv-load), oppure la shell con `set -a; source .env; set +a` se stai \
         lanciando il binario a mano.\n         Un .env copiato prima di una fetta che ha aggiunto impostazioni non le \
         ha: confrontalo con .env.example."
    )]
    Incomplete {
        #[source]
        source: Box<figment::Error>,
    },
    /// Un valore c'è ed è sbagliato: un numero non numerico, un booleano scritto
    /// male. Qui il messaggio di figment basta — dice quale chiave e perché — e
    /// aggiungere la prosa qui sopra sarebbe rumore su un errore già chiaro.
    #[error("configurazione non valida")]
    Invalid {
        #[source]
        source: Box<figment::Error>,
    },
}

impl AppConfig {
    /// L'errore è incapsulato: `figment::Error` accumula il contesto di tutte le
    /// sorgenti e supera i 200 byte, che non vale la pena propagare per valore.
    pub fn load() -> Result<Self, ConfigError> {
        Figment::new()
            .merge(Toml::file("config/default.toml"))
            .merge(Env::prefixed("PROPFLOW__").split("__"))
            .extract()
            .map_err(|error| {
                // `MissingField` è il caso del `.env` vecchio; `UnknownField` no,
                // ma condivide il rimedio — si guarda `.env.example` e si vede
                // come si chiama davvero.
                let incomplete = matches!(
                    error.kind,
                    figment::error::Kind::MissingField(_) | figment::error::Kind::UnknownField(..)
                );
                let source = Box::new(error);
                if incomplete {
                    ConfigError::Incomplete { source }
                } else {
                    ConfigError::Invalid { source }
                }
            })
    }

    #[must_use]
    pub const fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.server.request_timeout_seconds)
    }
}

const fn default_mail_port() -> u16 {
    587
}

fn default_mail_from_name() -> String {
    "PropFlow".to_owned()
}

const fn default_mail_encryption() -> MailEncryptionConfig {
    MailEncryptionConfig::StartTls
}

const fn default_mail_timeout_seconds() -> u64 {
    10
}

fn default_bind() -> String {
    "0.0.0.0:8080".to_owned()
}
const fn default_request_timeout_seconds() -> u64 {
    30
}
const fn default_body_limit_bytes() -> usize {
    1024 * 1024
}
const fn default_upload_body_limit_bytes() -> usize {
    25 * 1024 * 1024
}
const fn default_secure_cookies() -> bool {
    true
}
fn default_storage_region() -> String {
    "us-east-1".to_owned()
}
fn default_storage_bucket() -> String {
    "propflow-documents".to_owned()
}
const fn default_storage_path_style() -> bool {
    true
}
const fn default_presign_ttl_seconds() -> u32 {
    300
}
const fn default_storage_timeout_seconds() -> u64 {
    30
}
const fn default_max_connections() -> u32 {
    16
}
const fn default_statement_timeout_seconds() -> u64 {
    10
}
fn default_issuer() -> String {
    "propflow".to_owned()
}
fn default_audience() -> String {
    "propflow-api".to_owned()
}
const fn default_access_ttl_seconds() -> i64 {
    900
}
const fn default_refresh_ttl_days() -> i64 {
    30
}
fn default_service_name() -> String {
    "propflow-api".to_owned()
}
fn default_log_filter() -> String {
    "info,propflow_api=debug".to_owned()
}
