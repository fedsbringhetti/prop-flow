use std::fmt;

use chrono::{DateTime, Utc};

use crate::error::{DomainError, DomainResult};
use crate::shared::{CountryCode, Currency, OrgId, text};

/// Natura dell'organizzazione.
///
/// Nell'MVP esiste solo `Personal` (un proprietario privato). `Business` e
/// `Condominium` sono già nell'enum del database perché la V3 (condomini,
/// multi-proprietario) deve poterli usare senza una migrazione dello schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrgKind {
    Personal,
    Business,
    Condominium,
}

impl OrgKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Business => "business",
            Self::Condominium => "condominium",
        }
    }
}

/// Identificatore leggibile dell'organizzazione, usato negli URL.
/// Stesso vincolo del database: `^[a-z0-9][a-z0-9-]{1,62}$`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Slug(String);

impl Slug {
    pub fn parse(raw: impl AsRef<str>) -> DomainResult<Self> {
        let value = raw.as_ref().trim().to_lowercase();
        let valid_length = (2..=63).contains(&value.len());
        let valid_start = value
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
        let valid_body = value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');

        if !(valid_length && valid_start && valid_body) {
            return Err(DomainError::validation(
                "slug",
                "ammessi minuscole, cifre e trattini, da 2 a 63 caratteri",
            ));
        }
        Ok(Self(value))
    }

    /// Slug derivato dal nome più un suffisso preso dall'identificatore.
    ///
    /// Il suffisso rende la collisione praticamente impossibile senza un ciclo di
    /// retry sul vincolo unique del database: la generazione resta una funzione
    /// pura e la registrazione non ha rami di ripetizione.
    #[must_use]
    pub fn derive(name: &str, org_id: OrgId) -> Self {
        let mut base: String = name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        base = base.trim_matches('-').to_owned();
        base.truncate(48);
        while base.contains("--") {
            base = base.replace("--", "-");
        }
        let base = base.trim_matches('-');

        let suffix: String = org_id
            .as_uuid()
            .simple()
            .to_string()
            .chars()
            .rev()
            .take(6)
            .collect();

        let slug = if base.is_empty() {
            format!("org-{suffix}")
        } else {
            format!("{base}-{suffix}")
        };
        Self(slug)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Confine di isolamento dei dati: ogni entità di business appartiene a una
/// organizzazione, e le policy RLS filtrano su `org_id`.
#[derive(Debug, Clone)]
pub struct Organization {
    id: OrgId,
    name: String,
    slug: Slug,
    kind: OrgKind,
    country: CountryCode,
    timezone: String,
    default_currency: Currency,
    created_at: DateTime<Utc>,
}

impl Organization {
    /// Organizzazione personale creata all'onboarding di un proprietario.
    pub fn personal(id: OrgId, name: &str, now: DateTime<Utc>) -> DomainResult<Self> {
        let name = text::required("name", name, 200)?;
        Ok(Self {
            id,
            slug: Slug::derive(&name, id),
            name,
            kind: OrgKind::Personal,
            country: CountryCode::IT,
            timezone: "Europe/Rome".to_owned(),
            default_currency: Currency::EUR,
            created_at: now,
        })
    }

    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn from_persistence(
        id: OrgId,
        name: String,
        slug: Slug,
        kind: OrgKind,
        country: CountryCode,
        timezone: String,
        default_currency: Currency,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            slug,
            kind,
            country,
            timezone,
            default_currency,
            created_at,
        }
    }

    #[must_use]
    pub const fn id(&self) -> OrgId {
        self.id
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub const fn slug(&self) -> &Slug {
        &self.slug
    }
    #[must_use]
    pub const fn kind(&self) -> OrgKind {
        self.kind
    }
    #[must_use]
    pub const fn country(&self) -> CountryCode {
        self.country
    }
    #[must_use]
    pub fn timezone(&self) -> &str {
        &self.timezone
    }
    #[must_use]
    pub const fn default_currency(&self) -> Currency {
        self.default_currency
    }
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_slug_is_valid_and_stable() {
        let id = OrgId::generate();
        let slug = Slug::derive("Immobili   di Mario Rossi & Figli!", id);
        assert!(Slug::parse(slug.as_str()).is_ok(), "slug generato: {slug}");
        assert_eq!(slug, Slug::derive("Immobili   di Mario Rossi & Figli!", id));
        assert!(!slug.as_str().contains("--"));
    }

    #[test]
    fn derived_slug_survives_names_without_alphanumerics() {
        let slug = Slug::derive("!!! ???", OrgId::generate());
        assert!(slug.as_str().starts_with("org-"));
        assert!(Slug::parse(slug.as_str()).is_ok());
    }

    #[test]
    fn slug_parsing_rejects_invalid_shapes() {
        for candidate in ["a", "-abc", "Abc!", &"a".repeat(64)] {
            assert!(
                Slug::parse(candidate).is_err(),
                "{candidate} doveva essere rifiutato"
            );
        }
    }
}
