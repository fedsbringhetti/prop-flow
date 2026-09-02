use propflow_domain::DomainError;
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer};

pub mod auth;
pub mod billing;
pub mod documents;
pub mod expenses;
pub mod health;
pub mod invitations;
pub mod leases;
pub mod payments;
pub mod properties;
pub mod rooms;
pub mod tickets;

/// Distingue "campo assente" da "campo presente e null".
///
/// Con `Option<T>` semplice le due cose sono indistinguibili, e un PATCH non
/// potrebbe mai cancellare un valore. Vive qui e non nel modulo di una singola
/// risorsa perché è la semantica del PATCH di tutta l'API.
pub(crate) fn explicit_null<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// Converte un importo dal filo.
///
/// Gli importi sono stringhe decimali e non numeri JSON: un float binario non
/// rappresenta esattamente 0.10, e in contabilità la differenza si accumula. Il
/// nome del campo si passa perché finisce nell'errore per campo del 422.
pub(crate) fn amount(field: &'static str, raw: &str) -> Result<Decimal, DomainError> {
    raw.trim()
        .parse::<Decimal>()
        .map_err(|_| DomainError::validation(field, "must be a decimal amount"))
}

/// Formatta un importo per il filo: sempre due decimali.
///
/// La scala di un `Decimal` dipende da come è stato costruito — `Decimal::ZERO` ha
/// scala zero, un valore riletto da `NUMERIC(14,2)` ha scala due — quindi senza
/// normalizzare lo stesso deposito uscirebbe come `"0"` nella risposta alla
/// creazione e `"0.00"` alla lettura successiva. Il dominio ammette al massimo due
/// decimali, quindi il troncamento non può perdere informazione.
pub(crate) fn format_amount(value: Decimal) -> String {
    format!("{value:.2}")
}
