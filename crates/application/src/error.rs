use propflow_domain::DomainError;
use propflow_domain::ports::RepositoryError;

/// Errori del service layer. Sono la sola cosa che i bordi HTTP devono tradurre:
/// né `DomainError` né `RepositoryError` arrivano fino agli handler.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AppError {
    #[error("dati non validi: {0}")]
    Validation(#[from] DomainError),

    /// Credenziali assenti o non valide. Il messaggio è volutamente generico:
    /// distinguere "utente inesistente" da "password errata" regala a un
    /// attaccante un oracolo per enumerare gli account.
    #[error("autenticazione non riuscita")]
    Unauthenticated,

    #[error("operazione non consentita: {0}")]
    Forbidden(&'static str),

    #[error("{entity} non trovato")]
    NotFound { entity: &'static str },

    #[error("{resource} già esistente")]
    Conflict { resource: &'static str },

    /// Un vincolo del database ha rifiutato la scrittura per un motivo che non
    /// sappiamo tradurre in un messaggio di dominio.
    #[error("violazione di integrità: {details}")]
    Integrity { details: String },

    #[error("errore interno")]
    Internal { details: String },
}

impl AppError {
    pub fn internal(details: impl std::fmt::Display) -> Self {
        Self::Internal {
            details: details.to_string(),
        }
    }

    #[must_use]
    pub const fn not_found(entity: &'static str) -> Self {
        Self::NotFound { entity }
    }
}

impl From<RepositoryError> for AppError {
    fn from(error: RepositoryError) -> Self {
        match error {
            // Il nome del vincolo è un dettaglio dello schema: resta nei log,
            // non nella risposta. La mappatura verso una risorsa parlante la fa
            // lo use case, che sa cosa stava scrivendo.
            RepositoryError::Conflict { constraint } => Self::Integrity {
                details: format!("vincolo `{constraint}`"),
            },
            RepositoryError::Integrity { details } => Self::Integrity { details },
            // La RLS ha rifiutato una scrittura che il service layer aveva già
            // autorizzato: è un difetto nostro, non un errore del chiamante.
            RepositoryError::RowLevelSecurity => {
                Self::Forbidden("accesso negato dalle policy di riga")
            }
            RepositoryError::Backend { details } => Self::Internal { details },
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
