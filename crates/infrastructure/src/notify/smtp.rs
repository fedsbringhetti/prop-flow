use std::time::Duration;

use async_trait::async_trait;
use lettre::message::{Mailbox, MessageBuilder};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use propflow_application::ports::{Notification, NotificationContent, Notifier, NotifyError};

/// Come si protegge la connessione verso il relay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailEncryption {
    /// Connessione in chiaro, poi `STARTTLS`. È la porta 587 e il caso normale.
    StartTls,
    /// TLS dal primo byte. È la porta 465, che diversi fornitori offrono ancora.
    Tls,
    /// Nessuna cifratura. **Solo per il raccoglitore locale**: un relay vero
    /// raggiunto così manda credenziali e destinatari in chiaro sulla rete.
    None,
}

/// Configurazione del relay.
#[derive(Debug, Clone)]
pub struct SmtpSettings {
    pub host: String,
    pub port: u16,
    /// Assenti sul raccoglitore locale, che non autentica nessuno. Su un relay
    /// vero sono entrambe presenti: una sola delle due è una configurazione a
    /// metà, e l'adattatore la rifiuta all'avvio invece che al primo invio.
    pub username: Option<String>,
    pub password: Option<String>,
    pub from_address: String,
    pub from_name: String,
    pub encryption: MailEncryption,
    pub timeout: Duration,
}

/// Errori di **costruzione**, distinti da quelli di esercizio.
///
/// Stessa scelta dell'adattatore S3: qui si fallisce all'avvio, e un avvio che
/// fallisce è meglio di un servizio che accetta traffico e non recapita niente —
/// che per il recupero della password significa una persona chiusa fuori dal
/// proprio account senza nessun modo di rientrare.
#[derive(Debug, thiserror::Error)]
pub enum SmtpSetupError {
    #[error("indirizzo del mittente non valido: {0}")]
    Sender(String),
    #[error("relay non configurabile: {0}")]
    Relay(String),
    #[error(
        "credenziali SMTP incomplete: servono sia l'utenza sia la password, o nessuna delle due"
    )]
    PartialCredentials,
}

/// Recapito via SMTP.
///
/// # Che cosa **non** finisce nei log
///
/// `docs/04-sicurezza.md` vieta email, token e corpi nelle tracce, e un client SMTP
/// che registri la richiesta li registrerebbe tutti e tre insieme — è il posto più
/// facile del progetto in cui far uscire dati personali. Qui non si registra nulla
/// dell'invio riuscito, e del fallito solo la **classe** dell'errore: non il
/// destinatario, non l'oggetto, non il corpo.
///
/// `Notification` ha già un `Debug` redatto e questo adattatore non lo annulla.
pub struct SmtpNotifier {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpNotifier {
    /// Costruisce il trasporto. **Non apre la connessione**: quella si apre al
    /// primo invio, ed è `Notifier::probe` a dire se il relay risponde.
    pub fn new(settings: SmtpSettings) -> Result<Self, SmtpSetupError> {
        let address = settings
            .from_address
            .parse()
            .map_err(|_| SmtpSetupError::Sender(settings.from_address.clone()))?;
        let from = Mailbox::new(Some(settings.from_name.clone()), address);

        let credentials = match (&settings.username, &settings.password) {
            (Some(user), Some(password)) => Some(Credentials::new(user.clone(), password.clone())),
            (None, None) => None,
            // Una sola delle due è quasi sempre una variabile dimenticata, e il
            // sintomo arriverebbe al primo invio — cioè lontano dalla causa.
            _ => return Err(SmtpSetupError::PartialCredentials),
        };

        let mut builder = match settings.encryption {
            // `dangerous` nel nome è di lettre e vuol dire «senza TLS»: è il caso
            // del raccoglitore locale, e la configurazione lo dichiara apposta.
            MailEncryption::None => {
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&settings.host)
            }
            MailEncryption::StartTls | MailEncryption::Tls => {
                let parameters = TlsParameters::new(settings.host.clone())
                    .map_err(|error| SmtpSetupError::Relay(error.to_string()))?;
                let tls = if settings.encryption == MailEncryption::Tls {
                    Tls::Wrapper(parameters)
                } else {
                    Tls::Required(parameters)
                };
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&settings.host).tls(tls)
            }
        };
        builder = builder.port(settings.port).timeout(Some(settings.timeout));
        if let Some(credentials) = credentials {
            builder = builder.credentials(credentials);
        }

        Ok(Self {
            transport: builder.build(),
            from,
        })
    }
}

#[async_trait]
impl Notifier for SmtpNotifier {
    async fn send(&self, notification: &Notification) -> Result<(), NotifyError> {
        let to: Mailbox = notification
            .to
            .as_str()
            .parse()
            .map_err(|_| NotifyError::RejectedRecipient)?;

        let (subject, body) = render(&notification.content);
        let message = MessageBuilder::new()
            .from(self.from.clone())
            .to(to)
            .subject(subject)
            .body(body)
            .map_err(|error| NotifyError::Backend {
                details: error.to_string(),
            })?;

        self.transport
            .send(message)
            .await
            .map(|_| ())
            .map_err(map_error)
    }

    /// `test_connection` apre la connessione, dice `NOOP` e chiude. Quello che
    /// **non** prova lo dice il commento della porta: che il messaggio arriverebbe.
    async fn probe(&self) -> Result<(), NotifyError> {
        match self.transport.test_connection().await {
            Ok(true) => Ok(()),
            Ok(false) => Err(NotifyError::Backend {
                details: "il relay SMTP non ha risposto".to_owned(),
            }),
            Err(error) => Err(NotifyError::Backend {
                details: error.to_string(),
            }),
        }
    }
}

/// Il testo dei messaggi.
///
/// **In italiano**, come l'interfaccia: è ciò che una persona legge, e la regola di
/// `CLAUDE.md` §3 sull'inglese riguarda ciò che si incontra dall'esterno del
/// repository. Vive qui e non nel service layer perché la resa appartiene
/// all'adattatore: passargli una stringa già composta significherebbe scrivere il
/// template dentro un caso d'uso.
///
/// Testo semplice e non HTML, e non è provvisorio: un'email che porta **un codice
/// da incollare** non guadagna niente da una resa ricca, e ogni client di posta la
/// mostra allo stesso modo.
fn render(content: &NotificationContent) -> (String, String) {
    match content {
        NotificationContent::TenantInvitation {
            organization_name,
            accept_token,
        } => (
            format!("Invito a {organization_name} su PropFlow"),
            format!(
                "{organization_name} ti ha invitato come inquilino su PropFlow.\n\n\
                 Registrati, poi apri «Ho un invito» e incolla questo codice:\n\n\
                 {accept_token}\n\n\
                 Il codice vale sette giorni e si usa una volta sola.\n"
            ),
        ),
        NotificationContent::PasswordReset {
            code,
            valid_minutes,
        } => (
            "Recupero della password di PropFlow".to_owned(),
            format!(
                "Hai chiesto di reimpostare la password del tuo account PropFlow.\n\n\
                 Incolla questo codice nella schermata di recupero:\n\n\
                 {code}\n\n\
                 Vale {valid_minutes} minuti e si usa una volta sola. Se non l'hai\n\
                 chiesto tu, non devi fare niente: senza il codice non cambia nulla.\n"
            ),
        ),
    }
}

/// La classe dell'errore, **senza il destinatario**.
///
/// Il codice di risposta del relay dice se ha rifiutato la casella o se non è
/// disponibile, e sono due cose diverse per chi legge i log: la prima è un
/// indirizzo sbagliato, la seconda è un guasto.
fn map_error(error: lettre::transport::smtp::Error) -> NotifyError {
    if error.is_permanent() {
        NotifyError::RejectedRecipient
    } else {
        NotifyError::Backend {
            details: error.to_string(),
        }
    }
}
