use std::fmt;

use crate::error::{DomainError, DomainResult};
use crate::shared::text;

/// Chiave di idempotenza di un pagamento, **fornita dal client**.
///
/// È la decisione più importante del modulo, e l'unica in cui sbagliare costa
/// denaro. `app.payments.idempotency_key` è `NOT NULL` con l'indice unico
/// `payments_idempotency_uk` su `(org_id, idempotency_key)`: se a generarla fosse
/// il server, ogni ritentativo produrrebbe una chiave nuova, quindi **un secondo
/// pagamento**, quindi un saldo che scende **due volte**. Un doppio invio — un tap
/// ripetuto, un retry di rete, un client che non ha visto la risposta —
/// dimezzerebbe l'insoluto senza che nessuno se ne accorga, e la traccia in
/// `audit_log` mostrerebbe due inserimenti perfettamente legittimi. Vedi ADR-033.
///
/// **Il tetto è nostro**: a schema la colonna è `TEXT` senza vincoli. Il valore
/// arriva dall'header `Idempotency-Key`, quindi si accettano solo caratteri
/// stampabili ASCII senza spazi — un header con spazi o byte di controllo non
/// sopravvive intatto al primo proxy, e due chiavi che differiscono per uno spazio
/// sono due pagamenti per l'indice e uno solo per chi guarda. È la stessa ragione
/// per cui [`InvoiceNumber`](crate::billing::InvoiceNumber) normalizza.
///
/// **Non si normalizza la cassa**: `A1` e `a1` restano chiavi diverse, perché il
/// generatore è il client e nulla dice che le tratti come uguali. Confonderle
/// rifiuterebbe come duplicato un pagamento legittimo, che è il danno opposto e
/// peggiore.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    /// Abbondante per un UUID (36) o un ULID (26), che è ciò che i client
    /// generano. Il limite esiste per fermare una chiave che sia in realtà un
    /// corpo di richiesta, non per stringere sui formati.
    pub const MAX_LENGTH: usize = 200;

    pub fn parse(value: &str) -> DomainResult<Self> {
        let value = text::required("idempotency_key", value, Self::MAX_LENGTH)?;
        if !value.chars().all(|c| c.is_ascii_graphic()) {
            return Err(DomainError::validation(
                "idempotency_key",
                "must contain only printable ASCII characters, without spaces",
            ));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Ricostruzione da persistenza: non rivalida, come ogni `from_persistence`.
    #[must_use]
    pub const fn from_persistence(value: String) -> Self {
        Self(value)
    }
}

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_trimmed_and_bounded() {
        assert_eq!(
            IdempotencyKey::parse("  0192f3a1-1c7e-7a3b-9f4d-2c8e1b5a7d90  ")
                .expect("valida")
                .as_str(),
            "0192f3a1-1c7e-7a3b-9f4d-2c8e1b5a7d90"
        );
        assert!(IdempotencyKey::parse("   ").is_err());
        assert!(IdempotencyKey::parse(&"k".repeat(IdempotencyKey::MAX_LENGTH + 1)).is_err());
    }

    /// Un header con spazi o byte di controllo non sopravvive intatto al primo
    /// proxy, e la differenza fra due chiavi è la differenza fra un pagamento e
    /// due.
    #[test]
    fn a_key_refuses_what_an_http_header_would_not_carry_intact() {
        assert!(IdempotencyKey::parse("chiave con spazi").is_err());
        assert!(IdempotencyKey::parse("chiave\tcon\ttab").is_err());
        assert!(IdempotencyKey::parse("chiave\ncon\na\ncapo").is_err());
        assert!(IdempotencyKey::parse("chiave-àccentata").is_err());
    }

    /// La cassa **non** si normalizza: il generatore è il client, e trattare `A1`
    /// e `a1` come la stessa chiave rifiuterebbe come duplicato un pagamento
    /// legittimo.
    #[test]
    fn case_is_not_normalized() {
        let upper = IdempotencyKey::parse("A1").expect("valida");
        let lower = IdempotencyKey::parse("a1").expect("valida");
        assert_ne!(upper, lower);
    }
}
