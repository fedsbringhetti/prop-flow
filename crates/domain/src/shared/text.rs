use crate::error::{DomainError, DomainResult};

/// Normalizza un campo testuale obbligatorio: trim, non vuoto, lunghezza massima.
/// Gli stessi limiti sono ripetuti come `CHECK` nello schema — qui servono a
/// restituire un 422 leggibile invece di una violazione di vincolo.
pub(crate) fn required(field: &'static str, value: &str, max: usize) -> DomainResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DomainError::validation(field, "is required"));
    }
    if trimmed.chars().count() > max {
        return Err(DomainError::validation(
            field,
            format!("exceeds {max} characters"),
        ));
    }
    Ok(trimmed.to_owned())
}

/// Normalizza un campo opzionale: le stringhe vuote o di soli spazi diventano `None`,
/// così il database non accumula la distinzione priva di senso fra `''` e `NULL`.
pub(crate) fn optional(
    field: &'static str,
    value: Option<String>,
    max: usize,
) -> DomainResult<Option<String>> {
    match value {
        None => Ok(None),
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            if trimmed.chars().count() > max {
                return Err(DomainError::validation(
                    field,
                    format!("exceeds {max} characters"),
                ));
            }
            Ok(Some(trimmed.to_owned()))
        }
    }
}
