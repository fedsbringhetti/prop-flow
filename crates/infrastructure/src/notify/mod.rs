//! Posta in uscita.
//!
//! Un solo adattatore, SMTP, che parla con qualunque relay: un raccoglitore locale
//! in sviluppo, un fornitore vero in produzione. È la stessa forma dello storage —
//! `rusty-s3` parla con qualunque S3 compatibile — e la stessa ragione: il fornitore è
//! un **dato di configurazione** e non un ramo del codice
//! (`docs/06-sviluppo.md` §Aggiungere un provider esterno).
//!
//! La porta è `propflow_application::ports::Notifier`.

mod smtp;

pub use smtp::{MailEncryption, SmtpNotifier, SmtpSettings, SmtpSetupError};
