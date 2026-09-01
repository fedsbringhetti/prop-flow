use std::fmt;

use rust_decimal::Decimal;

use crate::error::{DomainError, DomainResult};

/// Peso di un membro nella ripartizione `weighted`.
///
/// **Non è un importo, ed è la ragione per cui esiste come tipo.**
/// `app.expense_group_members.weight` è `NUMERIC(6,3) CHECK (weight > 0)`, mentre
/// gli importi sono `app.money_amount`, cioè `NUMERIC(14,2)`. Passare un peso come
/// [`Money`](crate::shared::Money) accetterebbe due decimali dove ne servono tre —
/// una stanza contata `1.5` volte passa, `0.333` no — e verrebbe rifiutato o
/// troncato dal database, che è il momento peggiore per scoprirlo. È la stessa
/// distinzione che [`Consumption`](crate::billing::Consumption) fa nelle bollette.
///
/// Il caso d'uso è quello dichiarato dal commento dello schema: «peso per split
/// `weighted` (es. stanza doppia = 2)». Chi occupa la doppia paga di più.
///
/// **Strettamente positivo**, come il `CHECK`: un peso nullo significherebbe un
/// membro che non partecipa, e quello si esprime uscendo dal gruppo o restando
/// fuori dalla spesa — non con un numero che poi divide per zero se tutti lo
/// usano.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Weight(Decimal);

impl Weight {
    /// Tre decimali, come `NUMERIC(6,3)`.
    pub const MAX_SCALE: u32 = 3;
    /// 10^3: `NUMERIC(6,3)` rappresenta al massimo 999.999.
    const MAX_ABS: i64 = 1_000;

    pub fn parse(value: Decimal) -> DomainResult<Self> {
        if value.scale() > Self::MAX_SCALE {
            return Err(DomainError::validation(
                "weight",
                "allows at most three decimal places",
            ));
        }
        if value <= Decimal::ZERO {
            return Err(DomainError::validation(
                "weight",
                "must be greater than zero",
            ));
        }
        if value >= Decimal::from(Self::MAX_ABS) {
            return Err(DomainError::validation("weight", "weight out of range"));
        }
        Ok(Self(value))
    }

    /// Il peso predefinito: una parte a testa. È il `DEFAULT 1` dello schema.
    #[must_use]
    pub fn one() -> Self {
        Self(Decimal::ONE)
    }

    #[must_use]
    pub const fn value(self) -> Decimal {
        self.0
    }

    /// Il peso in millesimi, come intero esatto.
    ///
    /// La ripartizione lavora in interi e non in [`Decimal`]: vedi
    /// [`Split`](super::Split), dove la ragione è spiegata per esteso. Qui basta
    /// sapere che la scala è al massimo tre, quindi la moltiplicazione per mille è
    /// esatta e non può perdere cifre.
    #[must_use]
    pub fn milli(self) -> i128 {
        (self.0 * Decimal::from(1_000))
            .trunc()
            .try_into()
            .unwrap_or(0)
    }

    /// Ricostruzione da persistenza: non rivalida, come ogni `from_persistence`.
    #[must_use]
    pub const fn from_persistence(value: Decimal) -> Self {
        Self(value)
    }
}

impl Default for Weight {
    fn default() -> Self {
        Self::one()
    }
}

impl fmt::Display for Weight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
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
    fn a_weight_accepts_three_decimals_and_refuses_the_fourth() {
        assert_eq!(
            Weight::parse(dec("1.125")).expect("valido").value(),
            dec("1.125")
        );
        assert!(Weight::parse(dec("1.1255")).is_err());
    }

    /// Il `CHECK (weight > 0)` dello schema, in forma di test: uno zero non è un
    /// peso, ed è la sola via per cui la somma dei pesi potrebbe essere zero.
    #[test]
    fn a_weight_is_strictly_positive() {
        assert!(Weight::parse(Decimal::ZERO).is_err());
        assert!(Weight::parse(dec("-1")).is_err());
        assert!(Weight::parse(dec("0.001")).is_ok());
    }

    #[test]
    fn a_weight_stays_inside_numeric_6_3() {
        assert!(Weight::parse(dec("999.999")).is_ok());
        assert!(Weight::parse(dec("1000")).is_err());
    }

    /// La conversione in millesimi è esatta: è il presupposto dell'aritmetica
    /// intera della ripartizione, e se saltasse le quote non tornerebbero.
    #[test]
    fn milli_is_exact() {
        assert_eq!(Weight::one().milli(), 1_000);
        assert_eq!(Weight::parse(dec("2")).expect("valido").milli(), 2_000);
        assert_eq!(Weight::parse(dec("1.5")).expect("valido").milli(), 1_500);
        assert_eq!(Weight::parse(dec("0.001")).expect("valido").milli(), 1);
        assert_eq!(
            Weight::parse(dec("999.999")).expect("valido").milli(),
            999_999
        );
    }
}
