use std::fmt;

use rust_decimal::Decimal;

use crate::error::{DomainError, DomainResult};
use crate::shared::Currency;

/// Quanto la macchina dichiara di fidarsi di ciò che ha letto: un numero fra 0 e
/// 1.
///
/// Replica il dominio `app.confidence`, che è `NUMERIC(5,4) CHECK (VALUE >= 0 AND
/// VALUE <= 1)`. Quattro decimali e non due: **non è un importo**, e passare da
/// qui un [`Money`](crate::shared::Money) o viceversa è l'errore che questo tipo
/// esiste per impedire — `0.9412` troncato a due decimali diventa `0.94`, che su
/// una soglia di rifiuto è una lettura diversa.
///
/// **Una confidenza bassa non è un fallimento.** È un segnale per chi rivede: il
/// job resta `succeeded` e la bolletta arriva comunque a `pending_review`. Non
/// esiste una soglia di rifiuto automatico in questa fetta, e sceglierla richiede
/// di aver visto le confidenze di un fornitore vero su bollette vere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Confidence(Decimal);

impl Confidence {
    /// Quattro decimali, come `NUMERIC(5,4)`.
    const MAX_SCALE: u32 = 4;

    pub fn new(value: Decimal) -> DomainResult<Self> {
        if value.scale() > Self::MAX_SCALE {
            return Err(DomainError::validation(
                "confidence",
                "allows at most four decimal places",
            ));
        }
        if value < Decimal::ZERO || value > Decimal::ONE {
            return Err(DomainError::validation(
                "confidence",
                "must be between 0 and 1",
            ));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn value(self) -> Decimal {
        self.0
    }

    /// Ricostruzione da persistenza: non rivalida, come ogni `from_persistence`.
    #[must_use]
    pub const fn from_persistence(value: Decimal) -> Self {
        Self(value)
    }
}

/// **Sempre quattro decimali**, come `NUMERIC(5,4)`.
///
/// Non è cosmetica, ed è un difetto trovato con il giro a mano invece che con la
/// suite: la scala di un `Decimal` dipende da come è stato costruito. Una
/// confidenza calcolata dall'adattatore come `1` ha scala zero, la stessa
/// confidenza riletta dal database ha scala quattro — e la risposta HTTP mostrava
/// `"1"` per i campi e `"1.0000"` per il totale, due grafie dello stesso numero
/// nello stesso corpo. È lo stesso difetto che `format_amount` risolve per gli
/// importi, e la cura è la stessa: normalizzare in un posto solo. Qui è il tipo,
/// perché `Display` è ciò che usano sia il bordo HTTP sia il codificatore JSON.
///
/// Il troncamento non può perdere informazione: il costruttore rifiuta più di
/// quattro decimali.
impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4}", self.0)
    }
}

/// Costo di una chiamata a un fornitore AI.
///
/// **Non è un [`Money`](crate::shared::Money), e non per pignoleria.**
/// `ai_jobs.cost_amount` è `NUMERIC(12,6)` ed è una delle due eccezioni volute
/// del controllo 8 delle invarianti dello schema — l'altra è la frequenza del
/// canone. Sei decimali perché una singola chiamata costa frazioni di centesimo:
/// passata come importo, `0.000012` verrebbe troncata a `0.00`, cioè un costo
/// diventerebbe zero proprio nel tipo che serve a sapere quanto si sta spendendo.
///
/// È la stessa ragione per cui esistono [`Weight`](crate::expense::Weight) e
/// [`Consumption`](crate::billing::Consumption): una colonna con una scala sua
/// merita un tipo suo, altrimenti la scala si perde al primo passaggio.
///
/// **In questa fetta vale sempre zero**, perché l'adattatore è deterministico e
/// non chiama nessuno. `ai_jobs_cost_ix` esiste perché un tetto di spesa per
/// organizzazione sia possibile, e quel tetto è fuori perimetro: tararlo su zero
/// non si potrebbe provare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiCost {
    amount: Decimal,
    currency: Currency,
}

impl AiCost {
    /// Sei decimali, come `NUMERIC(12,6)`.
    const MAX_SCALE: u32 = 6;
    /// 10^6: `NUMERIC(12,6)` lascia sei cifre alla parte intera.
    const MAX_ABS: i64 = 1_000_000;

    pub fn new(amount: Decimal, currency: Currency) -> DomainResult<Self> {
        if amount.scale() > Self::MAX_SCALE {
            return Err(DomainError::validation(
                "cost_amount",
                "allows at most six decimal places",
            ));
        }
        if amount.is_sign_negative() && !amount.is_zero() {
            // Lo stesso dice `CHECK (cost_amount IS NULL OR cost_amount >= 0)`.
            return Err(DomainError::validation("cost_amount", "cannot be negative"));
        }
        if amount.abs() >= Decimal::from(Self::MAX_ABS) {
            return Err(DomainError::validation("cost_amount", "cost out of range"));
        }
        Ok(Self { amount, currency })
    }

    #[must_use]
    pub const fn amount(self) -> Decimal {
        self.amount
    }

    #[must_use]
    pub const fn currency(self) -> Currency {
        self.currency
    }

    #[must_use]
    pub const fn from_persistence(amount: Decimal, currency: Currency) -> Self {
        Self { amount, currency }
    }
}

impl fmt::Display for AiCost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.amount, self.currency.as_str())
    }
}

/// Ciò che una chiamata al fornitore è costata, in tutti i sensi in cui
/// `ai_jobs` sa registrarlo.
///
/// Sta insieme e non sparso fra gli argomenti di `succeed` perché sono sei valori
/// opzionali dello stesso genere: passarne due invertiti compilerebbe senza che
/// nulla se ne accorga.
///
/// I contatori sono `u32` e la colonna è `INTEGER`: la conversione la controlla
/// [`AiJob::succeed`](super::AiJob::succeed), perché un fornitore che dichiarasse
/// più di due miliardi di token romperebbe la scrittura in un punto dove
/// l'errore non direbbe più da dove viene.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProviderUsage {
    /// Il modello effettivamente usato, come lo nomina il fornitore.
    pub model: Option<String>,
    pub provider_version: Option<String>,
    pub tokens_input: Option<u32>,
    pub tokens_output: Option<u32>,
    pub cost: Option<AiCost>,
    pub latency_ms: Option<u32>,
}

impl ProviderUsage {
    /// Il massimo che una colonna `INTEGER` regge.
    pub const MAX_COUNTER: u32 = i32::MAX as u32;

    /// Verifica che i contatori stiano in una colonna `INTEGER`.
    ///
    /// Sta nel dominio e non nel repository perché è un'invariante del dato, non
    /// un dettaglio di PostgreSQL: un conteggio di token che non ci sta in un
    /// intero con segno non è un conteggio, è un difetto dell'adattatore.
    pub(super) fn ensure_storable(&self) -> DomainResult<()> {
        for (field, value) in [
            ("tokens_input", self.tokens_input),
            ("tokens_output", self.tokens_output),
            ("latency_ms", self.latency_ms),
        ] {
            if value.is_some_and(|value| value > Self::MAX_COUNTER) {
                return Err(DomainError::validation(field, "counter out of range"));
            }
        }
        Ok(())
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
    fn confidence_lives_between_zero_and_one() {
        assert!(Confidence::new(dec("0")).is_ok());
        assert!(Confidence::new(dec("1")).is_ok());
        assert!(Confidence::new(dec("0.9412")).is_ok());
        assert!(Confidence::new(dec("1.0001")).is_err());
        assert!(Confidence::new(dec("-0.0001")).is_err());
    }

    /// Il tipo esiste per la scala: cinque decimali non stanno in
    /// `NUMERIC(5,4)`, e accettarli qui li farebbe arrotondare al database.
    #[test]
    fn confidence_refuses_a_fifth_decimal() {
        assert!(Confidence::new(dec("0.94125")).is_err());
    }

    /// La ragione per cui `AiCost` non è `Money`: sei decimali contro due.
    #[test]
    fn a_cost_keeps_the_six_decimals_that_money_would_lose() {
        let cost = AiCost::new(dec("0.000012"), Currency::EUR).expect("costo valido");
        assert_eq!(cost.amount(), dec("0.000012"));
        assert!(AiCost::new(dec("0.0000001"), Currency::EUR).is_err());
        assert!(AiCost::new(dec("-0.01"), Currency::EUR).is_err());
    }

    /// Il difetto che il giro a mano ha trovato: due grafie dello stesso numero.
    #[test]
    fn a_confidence_always_reads_with_four_decimals() {
        // Costruita a scala zero, riletta a scala quattro: stessa resa.
        assert_eq!(
            Confidence::new(dec("1")).expect("valida").to_string(),
            "1.0000"
        );
        assert_eq!(
            Confidence::from_persistence(dec("1.0000")).to_string(),
            "1.0000"
        );
        assert_eq!(
            Confidence::new(dec("0.62")).expect("valida").to_string(),
            "0.6200"
        );
        assert_eq!(
            Confidence::new(dec("0")).expect("valida").to_string(),
            "0.0000"
        );
    }

    #[test]
    fn a_counter_beyond_an_integer_column_is_refused() {
        let usage = ProviderUsage {
            tokens_input: Some(ProviderUsage::MAX_COUNTER),
            ..ProviderUsage::default()
        };
        assert!(usage.ensure_storable().is_ok());

        let usage = ProviderUsage {
            tokens_input: Some(ProviderUsage::MAX_COUNTER + 1),
            ..ProviderUsage::default()
        };
        assert!(usage.ensure_storable().is_err());
    }
}
