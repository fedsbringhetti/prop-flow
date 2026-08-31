use std::fmt;

use rust_decimal::Decimal;

use crate::error::{DomainError, DomainResult};
use crate::shared::text;

/// Numero della fattura, così come il fornitore lo ha stampato.
///
/// A schema è `TEXT` senza vincoli, quindi il tetto è nostro. Ma qui la
/// normalizzazione conta più di un limite di comodo: questa colonna è metà di
/// `utility_bills_dedup_uk`, l'indice unico su
/// `(org_id, supplier_vat, invoice_number)` che rifiuta la stessa fattura
/// caricata due volte. Due stringhe che differiscono per uno spazio finale sono
/// **due fatture diverse per l'indice e la stessa per una persona**: per questo
/// il valore si normalizza qui, una volta sola, invece che in ogni chiamante.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoiceNumber(String);

impl InvoiceNumber {
    pub const MAX_LENGTH: usize = 100;

    pub fn parse(value: &str) -> DomainResult<Self> {
        Ok(Self(text::required(
            "invoice_number",
            value,
            Self::MAX_LENGTH,
        )?))
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

impl fmt::Display for InvoiceNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Partita IVA del fornitore.
///
/// **Non si valida il formato, e non è una dimenticanza.** La partita IVA
/// italiana ha undici cifre, ma un fornitore estero no, e rifiutare un
/// `DE811907980` significherebbe rendere non registrabile una bolletta valida.
/// Lo schema infatti la tiene a `TEXT`.
///
/// Ciò che si fa invece è **normalizzare**: maiuscole e nessuno spazio interno.
/// La ragione è la stessa di [`InvoiceNumber`] ed è l'indice di deduplica:
/// `it 12345678901` e `IT12345678901` sono la stessa azienda per chi legge e due
/// chiavi diverse per PostgreSQL, quindi la stessa fattura passerebbe due volte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplierVat(String);

impl SupplierVat {
    pub const MAX_LENGTH: usize = 20;

    pub fn parse(value: &str) -> DomainResult<Self> {
        let normalized: String = value
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-' && *c != '.')
            .collect::<String>()
            .to_uppercase();
        let normalized = text::required("supplier_vat", &normalized, Self::MAX_LENGTH)?;
        if !normalized.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(DomainError::validation(
                "supplier_vat",
                "must contain only letters and digits",
            ));
        }
        Ok(Self(normalized))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub const fn from_persistence(value: String) -> Self {
        Self(value)
    }
}

impl fmt::Display for SupplierVat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Consumo rilevato dalla bolletta, con la sua unità di misura.
///
/// **Non è un [`Money`](crate::shared::Money), e passarcelo è l'errore che questo
/// tipo esiste per impedire.** La colonna è `NUMERIC(14,4)`: quattro decimali,
/// mentre `Money::new` ne ammette due perché replica `app.money_amount`. Un
/// consumo di `123.4567 kWh` passato come importo darebbe un `422` che parla di
/// «amount», mandando a cercare l'errore fra le cifre della bolletta invece che
/// nel tipo sbagliato.
///
/// Numero e unità stanno **insieme** per la stessa ragione per cui `Money` tiene
/// insieme importo e valuta: `123.4567` da solo non dice niente, e lo schema —
/// che ha due colonne indipendenti — permetterebbe di scrivere un consumo senza
/// unità o un'unità senza consumo. Qui non è esprimibile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Consumption {
    value: Decimal,
    unit: ConsumptionUnit,
}

impl Consumption {
    /// Quattro decimali, come `NUMERIC(14,4)`.
    const MAX_SCALE: u32 = 4;
    /// 10^10: `NUMERIC(14,4)` lascia dieci cifre alla parte intera.
    const MAX_ABS: i64 = 10_000_000_000;

    pub fn new(value: Decimal, unit: ConsumptionUnit) -> DomainResult<Self> {
        if value.scale() > Self::MAX_SCALE {
            return Err(DomainError::validation(
                "consumption",
                "allows at most four decimal places",
            ));
        }
        if value.is_sign_negative() && !value.is_zero() {
            // Lo stesso dice `CHECK (consumption IS NULL OR consumption >= 0)`.
            return Err(DomainError::validation("consumption", "cannot be negative"));
        }
        if value.abs() >= Decimal::from(Self::MAX_ABS) {
            return Err(DomainError::validation(
                "consumption",
                "consumption out of range",
            ));
        }
        Ok(Self { value, unit })
    }

    #[must_use]
    pub const fn value(&self) -> Decimal {
        self.value
    }

    #[must_use]
    pub const fn unit(&self) -> &ConsumptionUnit {
        &self.unit
    }

    #[must_use]
    pub const fn from_persistence(value: Decimal, unit: ConsumptionUnit) -> Self {
        Self { value, unit }
    }
}

impl fmt::Display for Consumption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.value, self.unit)
    }
}

/// Unità di misura del consumo: `kWh`, `m3`, `Smc`, `GJ`.
///
/// Testo libero e **non** un enum, deliberatamente: le utenze italiane usano
/// unità che cambiano per fornitore e per tipo di utenza (`Smc` per il gas,
/// `m3` per l'acqua, `kWh` per l'elettricità e per il teleriscaldamento), e un
/// enum chiuso costringerebbe a una migration ogni volta che ne compare una
/// nuova su una fattura. Lo schema la tiene a `TEXT` per la stessa ragione.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumptionUnit(String);

impl ConsumptionUnit {
    pub const MAX_LENGTH: usize = 20;

    pub fn parse(value: &str) -> DomainResult<Self> {
        Ok(Self(text::required(
            "consumption_unit",
            value,
            Self::MAX_LENGTH,
        )?))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub const fn from_persistence(value: String) -> Self {
        Self(value)
    }
}

impl fmt::Display for ConsumptionUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Nome del fornitore dell'utenza.
///
/// Il tetto è nostro: a schema è `TEXT` nullo. Non entra in nessun evento — per
/// un fornitore individuale è il nome di una persona, e gli eventi trasportano
/// identificatori e nomi di campo, non valori (`CLAUDE.md` §2.18).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplierName(String);

impl SupplierName {
    pub const MAX_LENGTH: usize = 200;

    pub fn parse(value: &str) -> DomainResult<Self> {
        Ok(Self(text::required(
            "supplier_name",
            value,
            Self::MAX_LENGTH,
        )?))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub const fn from_persistence(value: String) -> Self {
        Self(value)
    }
}

impl fmt::Display for SupplierName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn dec(raw: &str) -> Decimal {
        Decimal::from_str(raw).expect("decimale di test valido")
    }

    fn unit(raw: &str) -> ConsumptionUnit {
        ConsumptionUnit::parse(raw).expect("unità valida")
    }

    #[test]
    fn an_invoice_number_is_trimmed_and_bounded() {
        assert_eq!(
            InvoiceNumber::parse("  2026/A/00417  ")
                .expect("valido")
                .as_str(),
            "2026/A/00417"
        );
        assert!(InvoiceNumber::parse("").is_err());
        assert!(InvoiceNumber::parse("   ").is_err());
        assert!(InvoiceNumber::parse(&"9".repeat(101)).is_err());
    }

    /// La normalizzazione **è** la funzione di questo tipo: `utility_bills_dedup_uk`
    /// confronta byte, e senza questa riga la stessa fattura entrerebbe due volte
    /// scrivendo la partita IVA con o senza spazi.
    #[test]
    fn a_vat_number_is_normalized_so_the_dedup_index_can_do_its_job() {
        let canonical = SupplierVat::parse("IT12345678901").expect("valida");
        for spelling in [
            "it12345678901",
            "IT 12345678901",
            "  it-123.456.789.01  ",
            "It 1234 5678 901",
        ] {
            assert_eq!(
                SupplierVat::parse(spelling).expect("valida"),
                canonical,
                "`{spelling}` deve normalizzare sulla stessa chiave"
            );
        }
    }

    /// Undici cifre è la forma italiana, non la regola: un fornitore tedesco ha
    /// una partita IVA diversa e la sua bolletta è comunque registrabile.
    #[test]
    fn a_foreign_vat_number_is_accepted() {
        assert!(SupplierVat::parse("DE811907980").is_ok());
        assert!(SupplierVat::parse("FRAB123456789").is_ok());
        // Ciò che si rifiuta è il vuoto e ciò che non è alfanumerico.
        assert!(SupplierVat::parse("   ").is_err());
        assert!(SupplierVat::parse("IT123/456").is_err());
    }

    /// La trappola per cui questo tipo esiste: `Money` ammette due decimali, la
    /// colonna del consumo ne ha quattro.
    #[test]
    fn a_consumption_keeps_the_four_decimals_that_money_would_refuse() {
        let consumption = Consumption::new(dec("123.4567"), unit("kWh")).expect("valido");
        assert_eq!(consumption.value(), dec("123.4567"));
        assert_eq!(consumption.unit().as_str(), "kWh");
        assert_eq!(consumption.to_string(), "123.4567 kWh");

        // Il quinto decimale invece no: la colonna lo arrotonderebbe in silenzio.
        assert!(Consumption::new(dec("1.00001"), unit("kWh")).is_err());
    }

    #[test]
    fn a_consumption_is_non_negative_and_bounded_like_the_column() {
        assert!(Consumption::new(dec("-0.0001"), unit("m3")).is_err());
        assert!(Consumption::new(dec("0"), unit("m3")).is_ok());
        assert!(Consumption::new(dec("10000000000"), unit("m3")).is_err());
        assert!(Consumption::new(dec("9999999999.9999"), unit("m3")).is_ok());
    }

    /// L'unità è testo libero perché le utenze italiane ne usano diverse per
    /// fornitore e per tipo: un enum chiuso costerebbe una migration a ogni
    /// bolletta con una sigla nuova.
    #[test]
    fn a_consumption_unit_is_free_text_within_a_bound() {
        for raw in ["kWh", "m3", "Smc", "GJ"] {
            assert_eq!(ConsumptionUnit::parse(raw).expect("valida").as_str(), raw);
        }
        assert!(ConsumptionUnit::parse("").is_err());
        assert!(ConsumptionUnit::parse(&"u".repeat(21)).is_err());
    }

    #[test]
    fn a_supplier_name_is_trimmed_and_bounded() {
        assert_eq!(
            SupplierName::parse("  Enel Energia S.p.A.  ")
                .expect("valido")
                .as_str(),
            "Enel Energia S.p.A."
        );
        assert!(SupplierName::parse(" ").is_err());
        assert!(SupplierName::parse(&"a".repeat(201)).is_err());
    }
}
