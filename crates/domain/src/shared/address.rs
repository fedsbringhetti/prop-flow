use std::fmt;

use crate::error::{DomainError, DomainResult};
use crate::shared::text;

/// Codice paese ISO 3166-1 alpha-2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CountryCode([u8; 2]);

impl CountryCode {
    pub const IT: Self = Self(*b"IT");

    pub fn parse(raw: impl AsRef<str>) -> DomainResult<Self> {
        let upper = raw.as_ref().trim().to_ascii_uppercase();
        let bytes = upper.as_bytes();
        if bytes.len() != 2 || !bytes.iter().all(u8::is_ascii_uppercase) {
            return Err(DomainError::validation(
                "country",
                "must be an ISO 3166-1 alpha-2 code",
            ));
        }
        Ok(Self([bytes[0], bytes[1]]))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or("IT")
    }
}

impl Default for CountryCode {
    fn default() -> Self {
        Self::IT
    }
}

impl fmt::Display for CountryCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<String> for CountryCode {
    type Error = DomainError;
    fn try_from(value: String) -> DomainResult<Self> {
        Self::parse(value)
    }
}

impl From<CountryCode> for String {
    fn from(value: CountryCode) -> Self {
        value.as_str().to_owned()
    }
}

/// Indirizzo di un immobile.
///
/// I campi sono separati e non un blob: `city` e `postal_code` sono filtri di
/// ricerca di primo livello e nello schema hanno indici propri.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    street: String,
    extra: Option<String>,
    city: String,
    province: Option<String>,
    postal_code: Option<String>,
    country: CountryCode,
}

impl Address {
    pub fn new(
        street: impl Into<String>,
        extra: Option<String>,
        city: impl Into<String>,
        province: Option<String>,
        postal_code: Option<String>,
        country: CountryCode,
    ) -> DomainResult<Self> {
        Ok(Self {
            street: text::required("address_street", &street.into(), 200)?,
            extra: text::optional("address_extra", extra, 200)?,
            city: text::required("city", &city.into(), 120)?,
            province: text::optional("province", province, 60)?,
            postal_code: text::optional("postal_code", postal_code, 16)?,
            country,
        })
    }

    #[must_use]
    pub fn street(&self) -> &str {
        &self.street
    }
    #[must_use]
    pub fn extra(&self) -> Option<&str> {
        self.extra.as_deref()
    }
    #[must_use]
    pub fn city(&self) -> &str {
        &self.city
    }
    #[must_use]
    pub fn province(&self) -> Option<&str> {
        self.province.as_deref()
    }
    #[must_use]
    pub fn postal_code(&self) -> Option<&str> {
        self.postal_code.as_deref()
    }
    #[must_use]
    pub const fn country(&self) -> CountryCode {
        self.country
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_street_and_city() {
        assert!(Address::new("  ", None, "Milano", None, None, CountryCode::IT).is_err());
        assert!(Address::new("Via Roma 1", None, "", None, None, CountryCode::IT).is_err());
    }

    #[test]
    fn blanks_become_none() {
        let address = Address::new(
            " Via Roma 1 ",
            Some("   ".into()),
            "Milano",
            Some("MI".into()),
            None,
            CountryCode::IT,
        )
        .expect("valido");
        assert_eq!(address.street(), "Via Roma 1");
        assert_eq!(address.extra(), None);
        assert_eq!(address.province(), Some("MI"));
    }
}
