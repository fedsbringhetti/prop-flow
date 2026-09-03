use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::value_objects::Confidence;

/// Campo della bolletta che una lettura automatica sa proporre.
///
/// Un enum chiuso e non un testo libero, e la ragione non è la pulizia: da questi
/// nomi nasce la **forma** salvata in `ai_jobs.output`, che è ciò su cui
/// `ai_jobs_output_ix` — `gin (output jsonb_path_ops)` — permette di interrogare
/// «quanto spesso il fornitore trova il numero di fattura, e con che confidenza».
/// Con una chiave libera quella domanda si potrebbe fare solo dopo aver
/// normalizzato a mano un insieme di refusi.
///
/// **Non ci sono `room_id`, `lease_id` e `utility_type`**: dicono *a chi è
/// imputata* e *che utenza è*, che sono decisioni di chi registra la bolletta e
/// non cose stampate sulla carta in un modo che una macchina possa leggere senza
/// conoscere il portafoglio immobiliare. `total_amount` c'è ed è il campo che
/// conta di più: è quello che poi qualcuno pagherà.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BillField {
    SupplierName,
    SupplierVat,
    InvoiceNumber,
    IssueDate,
    DueDate,
    PeriodStart,
    PeriodEnd,
    TotalAmount,
    TaxableAmount,
    VatAmount,
    Consumption,
}

impl BillField {
    /// Tutti i campi, nell'ordine in cui compaiono su una bolletta.
    ///
    /// Serve all'adattatore, alla resa HTTP e ai test: un elenco scritto tre
    /// volte diverge alla prima aggiunta.
    pub const ALL: [Self; 11] = [
        Self::SupplierName,
        Self::SupplierVat,
        Self::InvoiceNumber,
        Self::IssueDate,
        Self::DueDate,
        Self::PeriodStart,
        Self::PeriodEnd,
        Self::TotalAmount,
        Self::TaxableAmount,
        Self::VatAmount,
        Self::Consumption,
    ];

    /// Nome stabile del campo. È una chiave di `ocr_extracted` e di
    /// `ai_jobs.output`, quindi rinominarlo cambia dati già scritti: coincide con
    /// il nome della colonna della bolletta apposta, perché chi rivede la
    /// proposta guarda quella colonna.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SupplierName => "supplier_name",
            Self::SupplierVat => "supplier_vat",
            Self::InvoiceNumber => "invoice_number",
            Self::IssueDate => "issue_date",
            Self::DueDate => "due_date",
            Self::PeriodStart => "period_start",
            Self::PeriodEnd => "period_end",
            Self::TotalAmount => "total_amount",
            Self::TaxableAmount => "taxable_amount",
            Self::VatAmount => "vat_amount",
            Self::Consumption => "consumption",
        }
    }

    /// Rilettura da persistenza. `None` per un nome sconosciuto: la regola degli
    /// enum di questo progetto vale anche per una chiave JSON, e un campo che il
    /// codice non conosce deve essere rumoroso invece di sparire.
    #[must_use]
    pub fn from_key(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|field| field.as_str() == value)
    }
}

/// Il valore che la macchina propone per un campo.
///
/// Tipizzato e non una stringa: `ocr_extracted` è la proposta che una persona
/// deve poter accettare con un gesto, e una data che arriva come `"01/08/2026"`
/// costringerebbe a indovinare il formato al momento di riempire il modulo. Chi
/// non riesce a interpretare un valore **non lo propone**: un campo assente è una
/// cosa che si vede, un campo con dentro testo non interpretabile no.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractedValue {
    Text(String),
    Date(NaiveDate),
    /// Un importo. Resta `Decimal` e non [`Money`](crate::shared::Money): la
    /// valuta non è un campo che l'OCR propone, resta `EUR` come per il resto del
    /// modulo contabile (ADR-030).
    Amount(Decimal),
    /// Un consumo, con la sua unità: `123.4567` da solo non dice niente, per la
    /// stessa ragione per cui [`Consumption`](crate::billing::Consumption) tiene
    /// insieme i due.
    Quantity {
        value: Decimal,
        unit: String,
    },
}

/// Un campo trovato, con quanto la macchina se ne fida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedField {
    pub field: BillField,
    pub value: ExtractedValue,
    pub confidence: Confidence,
}

/// Ciò che una lettura automatica ha trovato su una bolletta.
///
/// **Porta i valori**, ed è per questo che finisce in `utility_bills.ocr_extracted`
/// e non in `ai_jobs.output`: il contenuto di una bolletta è dato della bolletta,
/// eredita la RLS della bolletta — che comprende l'inquilino imputato — e il
/// commento della migration `0010` chiede che in `ai_jobs` vadano «SOLO parametri
/// e riferimenti». In `ai_jobs.output` va invece la sola [forma](Self::shape):
/// quali campi, con quale confidenza, **senza i valori**.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OcrReading {
    fields: Vec<ExtractedField>,
}

impl OcrReading {
    /// Una lettura vuota è ammessa e significa «non ho trovato niente di
    /// interpretabile», che è un esito e non un errore: il job resta `succeeded`
    /// con confidenza assente, e chi rivede vede una proposta vuota invece di un
    /// errore che non sa a cosa attribuire.
    #[must_use]
    pub fn new(fields: Vec<ExtractedField>) -> Self {
        Self { fields }
    }

    #[must_use]
    pub fn fields(&self) -> &[ExtractedField] {
        &self.fields
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// La confidenza complessiva della lettura: il **minimo** fra quelle dei
    /// campi trovati, e `None` se non ne è stato trovato nessuno.
    ///
    /// Il minimo e non la media, ed è una decisione presa guardandola.
    /// `utility_bills.ocr_confidence` è il numero che chi rivede vede accanto
    /// alla bolletta: con la media, dieci campi letti a `0.99` e il **totale**
    /// letto a `0.40` darebbero `0.94`, cioè una bolletta che sembra a posto e ha
    /// sbagliato l'unica cifra che qualcuno pagherà. Il minimo dice «c'è qualcosa
    /// da guardare», che è ciò per cui il numero esiste. La confidenza per campo
    /// resta comunque leggibile in `ocr_extracted`, quindi non si perde nulla:
    /// si sceglie solo cosa dice il riassunto.
    #[must_use]
    pub fn overall_confidence(&self) -> Option<Confidence> {
        self.fields.iter().map(|found| found.confidence).min()
    }

    /// La **forma** dell'esito: quali campi, con quale confidenza, senza i
    /// valori. È ciò che va in `ai_jobs.output`.
    ///
    /// Serve a misurare l'accuratezza per campo senza rileggere le bollette, che
    /// è la sola domanda che conta su un modulo di OCR — funziona abbastanza? —
    /// e che senza questa separazione richiederebbe di aprire i dati delle
    /// bollette per rispondere.
    #[must_use]
    pub fn shape(&self) -> Vec<(BillField, Confidence)> {
        self.fields
            .iter()
            .map(|found| (found.field, found.confidence))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn confidence(raw: &str) -> Confidence {
        Confidence::new(Decimal::from_str(raw).expect("decimale valido")).expect("confidenza")
    }

    fn reading() -> OcrReading {
        OcrReading::new(vec![
            ExtractedField {
                field: BillField::SupplierName,
                value: ExtractedValue::Text("Enel Energia".to_owned()),
                confidence: confidence("0.99"),
            },
            ExtractedField {
                field: BillField::TotalAmount,
                value: ExtractedValue::Amount(Decimal::from_str("128.40").expect("importo")),
                confidence: confidence("0.4"),
            },
        ])
    }

    /// La decisione che il commento di `overall_confidence` motiva: con la media
    /// questa lettura direbbe `0.695`, cioè nasconderebbe il totale letto male.
    #[test]
    fn the_overall_confidence_is_the_weakest_field() {
        assert_eq!(reading().overall_confidence(), Some(confidence("0.4")));
    }

    #[test]
    fn a_reading_without_fields_has_no_confidence() {
        assert_eq!(OcrReading::default().overall_confidence(), None);
        assert!(OcrReading::default().is_empty());
    }

    /// Il confine fra le due colonne JSONB: la forma non porta i valori.
    #[test]
    fn the_shape_carries_the_fields_and_not_their_values() {
        let shape = reading().shape();
        assert_eq!(
            shape,
            vec![
                (BillField::SupplierName, confidence("0.99")),
                (BillField::TotalAmount, confidence("0.4")),
            ]
        );
    }

    #[test]
    fn every_field_has_a_distinct_key_that_survives_a_round_trip() {
        let mut keys: Vec<&str> = BillField::ALL.iter().map(|field| field.as_str()).collect();
        let total = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), total, "due campi condividono una chiave");

        for field in BillField::ALL {
            assert_eq!(BillField::from_key(field.as_str()), Some(field));
        }
        assert_eq!(BillField::from_key("qualcosa_di_nuovo"), None);
    }
}
