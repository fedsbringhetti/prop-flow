use std::fmt;

use rust_decimal::Decimal;

use crate::error::{DomainError, DomainResult};

/// Codice valuta ISO 4217. Tre byte, nessuna allocazione.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Currency([u8; 3]);

impl Currency {
    pub const EUR: Self = Self(*b"EUR");

    pub fn parse(raw: impl AsRef<str>) -> DomainResult<Self> {
        let upper = raw.as_ref().trim().to_ascii_uppercase();
        let bytes = upper.as_bytes();
        if bytes.len() != 3 || !bytes.iter().all(u8::is_ascii_uppercase) {
            return Err(DomainError::validation(
                "currency",
                "must be a three-letter ISO 4217 code",
            ));
        }
        Ok(Self([bytes[0], bytes[1], bytes[2]]))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        // I byte sono ASCII maiuscoli per costruzione: il ramo di fallback è
        // irraggiungibile e serve solo a evitare `unwrap`, vietato dai lint.
        std::str::from_utf8(&self.0).unwrap_or("EUR")
    }
}

impl Default for Currency {
    fn default() -> Self {
        Self::EUR
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<String> for Currency {
    type Error = DomainError;
    fn try_from(value: String) -> DomainResult<Self> {
        Self::parse(value)
    }
}

impl From<Currency> for String {
    fn from(value: Currency) -> Self {
        value.as_str().to_owned()
    }
}

/// Importo monetario con valuta.
///
/// I limiti replicano il dominio `app.money_amount` (`NUMERIC(14,2)`): due
/// decimali e dodici cifre intere. Validare qui evita che un importo con quattro
/// decimali venga silenziosamente arrotondato dal database.
// Nessun PartialOrd: confrontare 10 EUR con 10 USD non è un'operazione di dominio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Money {
    amount: Decimal,
    currency: Currency,
}

impl Money {
    const MAX_SCALE: u32 = 2;
    /// 10^12: il massimo rappresentabile da `NUMERIC(14,2)` è 999_999_999_999.99.
    const MAX_ABS: i64 = 1_000_000_000_000;

    pub fn new(amount: Decimal, currency: Currency) -> DomainResult<Self> {
        if amount.scale() > Self::MAX_SCALE {
            return Err(DomainError::validation(
                "amount",
                "allows at most two decimal places",
            ));
        }
        if amount.abs() >= Decimal::from(Self::MAX_ABS) {
            return Err(DomainError::validation("amount", "amount out of range"));
        }
        Ok(Self { amount, currency })
    }

    /// Come [`Money::new`] ma rifiuta i negativi: per canoni, prezzi e depositi,
    /// dove un importo negativo non ha significato di dominio.
    pub fn non_negative(amount: Decimal, currency: Currency) -> DomainResult<Self> {
        if amount.is_sign_negative() && !amount.is_zero() {
            return Err(DomainError::validation("amount", "cannot be negative"));
        }
        Self::new(amount, currency)
    }

    #[must_use]
    pub const fn amount(&self) -> Decimal {
        self.amount
    }

    #[must_use]
    pub const fn currency(&self) -> Currency {
        self.currency
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.amount, self.currency)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(raw: &str) -> Decimal {
        Decimal::from_str(raw).expect("decimale di test valido")
    }

    #[test]
    fn accepts_two_decimals() {
        let money = Money::new(dec("1234.56"), Currency::EUR).expect("valido");
        assert_eq!(money.amount(), dec("1234.56"));
        assert_eq!(money.currency().as_str(), "EUR");
    }

    #[test]
    fn rejects_more_than_two_decimals() {
        assert!(Money::new(dec("1.005"), Currency::EUR).is_err());
    }

    #[test]
    fn rejects_out_of_scale_amounts() {
        assert!(Money::new(dec("1000000000000.00"), Currency::EUR).is_err());
    }

    #[test]
    fn non_negative_rejects_negatives_but_allows_zero() {
        assert!(Money::non_negative(dec("-0.01"), Currency::EUR).is_err());
        assert!(Money::non_negative(dec("0"), Currency::EUR).is_ok());
    }

    #[test]
    fn currency_parsing_is_case_insensitive() {
        assert_eq!(Currency::parse("eur").expect("valida"), Currency::EUR);
        assert!(Currency::parse("EURO").is_err());
        assert!(Currency::parse("E1R").is_err());
    }
}
