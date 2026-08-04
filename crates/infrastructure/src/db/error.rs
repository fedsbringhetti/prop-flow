use propflow_domain::ports::RepositoryError;

/// Traduce un errore di SQLx in un errore di dominio.
///
/// La distinzione fra conflitto, violazione di integrità e rifiuto della RLS non
/// è cosmetica: il primo è un `409` per il chiamante, il secondo un `409` o `422`,
/// il terzo un difetto del service layer che ha autorizzato ciò che non doveva.
/// Appiattirli su un errore generico significherebbe restituire `500` a un utente
/// che ha semplicemente riusato un'email.
pub fn map_sqlx_error(error: sqlx::Error) -> RepositoryError {
    match &error {
        sqlx::Error::Database(db_error) => {
            let code = db_error.code().unwrap_or_default().to_string();
            match code.as_str() {
                // 23505 unique_violation
                "23505" => RepositoryError::Conflict {
                    constraint: db_error.constraint().unwrap_or("sconosciuto").to_owned(),
                },
                // 23503 foreign_key_violation, 23514 check_violation,
                // 23P01 exclusion_violation, 23502 not_null_violation
                "23503" | "23514" | "23P01" | "23502" => RepositoryError::Integrity {
                    details: format!("{} ({})", db_error.constraint().unwrap_or("vincolo"), code),
                },
                // 42501 insufficient_privilege: DELETE revocato, oppure una
                // scrittura su audit_log.
                "42501" => RepositoryError::RowLevelSecurity,
                _ => RepositoryError::Backend {
                    details: format!("{code}: {db_error}"),
                },
            }
        }
        // Nessuna riga aggiornata su una UPDATE mirata: dal punto di vista del
        // chiamante la riga non è visibile, che sotto RLS equivale a non esistere.
        sqlx::Error::RowNotFound => RepositoryError::Backend {
            details: "riga non trovata".to_owned(),
        },
        other => RepositoryError::Backend {
            details: other.to_string(),
        },
    }
}
