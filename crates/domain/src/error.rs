use std::fmt;

/// Errori che il dominio sa produrre. Non contengono mai dettagli di
/// infrastruttura: la traduzione verso HTTP avviene ai bordi.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// Un valore in ingresso non rispetta le regole del value object.
    #[error("campo non valido `{field}`: {message}")]
    Validation {
        field: &'static str,
        message: String,
    },

    /// Un'invariante dell'aggregato sarebbe violata dall'operazione richiesta.
    #[error("invariante violata: {0}")]
    Invariant(String),

    /// L'entità richiesta non esiste (o non è visibile a chi la chiede: la RLS
    /// rende i due casi indistinguibili, ed è voluto — non si conferma
    /// l'esistenza di dati che non si possono vedere).
    #[error("{entity} non trovato")]
    NotFound { entity: &'static str },

    /// L'attore non ha i permessi per l'operazione.
    #[error("operazione non consentita: {0}")]
    Forbidden(&'static str),
}

impl DomainError {
    pub fn validation(field: &'static str, message: impl fmt::Display) -> Self {
        Self::Validation {
            field,
            message: message.to_string(),
        }
    }

    pub fn invariant(message: impl fmt::Display) -> Self {
        Self::Invariant(message.to_string())
    }

    pub const fn not_found(entity: &'static str) -> Self {
        Self::NotFound { entity }
    }
}

pub type DomainResult<T> = Result<T, DomainError>;
