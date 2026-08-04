use chrono::{DateTime, Utc};

/// Orologio iniettabile.
///
/// Chiamare `Utc::now()` dentro il dominio renderebbe i test dipendenti dal tempo
/// reale: scadenze, lockout e rotazioni si verificano solo potendo decidere "adesso".
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}
