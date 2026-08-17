use crate::error::{AppError, AppResult};
use propflow_domain::DomainError;

/// Lunghezza minima della password.
///
/// Dodici caratteri senza obbligo di classi miste: le regole di composizione
/// spingono verso password prevedibili ("Password1!"), mentre la lunghezza è la
/// leva che conta davvero contro il brute force offline.
pub const MIN_LENGTH: usize = 12;
pub const MAX_LENGTH: usize = 512;

/// Verifica la robustezza minima di una password in chiaro.
pub fn validate(plaintext: &str) -> AppResult<()> {
    let length = plaintext.chars().count();
    if length < MIN_LENGTH {
        return Err(AppError::Validation(DomainError::validation(
            "password",
            format!("must be at least {MIN_LENGTH} characters long"),
        )));
    }
    // Il limite alto non è estetico: Argon2 lavora sull'intero input, e una
    // password da un megabyte è un vettore di denial of service.
    if length > MAX_LENGTH {
        return Err(AppError::Validation(DomainError::validation(
            "password",
            format!("exceeds {MAX_LENGTH} characters"),
        )));
    }
    if plaintext.trim().is_empty() {
        return Err(AppError::Validation(DomainError::validation(
            "password",
            "cannot consist of whitespace only",
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_long_passphrase() {
        assert!(validate("cavallo batteria graffetta").is_ok());
    }

    #[test]
    fn rejects_short_blank_and_oversized() {
        assert!(validate("corta").is_err());
        assert!(validate(&" ".repeat(20)).is_err());
        assert!(validate(&"a".repeat(MAX_LENGTH + 1)).is_err());
    }
}
