use std::fmt;

use crate::error::{DomainError, DomainResult};

/// Indirizzo email normalizzato e validato.
///
/// La normalizzazione (trim + minuscolo) avviene qui e non nel database: la
/// colonna è `CITEXT` con un `CHECK`, ma affidarsi solo a quello significherebbe
/// scoprire l'errore come violazione di vincolo invece che come 422 leggibile.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Email(String);

impl Email {
    pub fn parse(raw: impl AsRef<str>) -> DomainResult<Self> {
        let normalized = raw.as_ref().trim().to_lowercase();

        if normalized.is_empty() {
            return Err(DomainError::validation("email", "non può essere vuota"));
        }
        if normalized.len() > 254 {
            return Err(DomainError::validation("email", "supera i 254 caratteri"));
        }
        if normalized.chars().any(char::is_whitespace) {
            return Err(DomainError::validation("email", "non può contenere spazi"));
        }

        // Stesso criterio del dominio `app.email_address` in migrations/0001:
        // esattamente una chiocciola, parte locale non vuota, dominio con punto.
        let mut parts = normalized.split('@');
        let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
            return Err(DomainError::validation(
                "email",
                "deve contenere una sola @",
            ));
        };
        if local.is_empty() {
            return Err(DomainError::validation("email", "parte locale mancante"));
        }
        match domain.rsplit_once('.') {
            Some((name, tld)) if !name.is_empty() && tld.len() >= 2 => {}
            _ => return Err(DomainError::validation("email", "dominio non valido")),
        }

        Ok(Self(normalized))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Email {
    type Error = DomainError;
    fn try_from(value: String) -> DomainResult<Self> {
        Self::parse(value)
    }
}

impl From<Email> for String {
    fn from(value: Email) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_case_and_whitespace() {
        let email = Email::parse("  Mario.Rossi@Example.IT ").expect("valida");
        assert_eq!(email.as_str(), "mario.rossi@example.it");
    }

    #[test]
    fn rejects_malformed_addresses() {
        for candidate in [
            "",
            "senza-chiocciola",
            "a@b",
            "due@@chiocciole.it",
            "@example.it",
            "a b@example.it",
        ] {
            assert!(
                Email::parse(candidate).is_err(),
                "{candidate} doveva essere rifiutata"
            );
        }
    }
}
