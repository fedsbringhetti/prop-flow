use std::str::FromStr;

use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;

use propflow_application::ports::{OcrError, OcrOutcome, OcrProvider};
use propflow_domain::ai::{
    BillField, Confidence, ExtractedField, ExtractedValue, OcrReading, ProviderUsage,
};

/// Lettura automatica deterministica di una bolletta in chiaro.
///
/// **Non è un segnaposto**, per la stessa ragione per cui `LoggingEventBus` non lo
/// è: rende osservabile e verificabile il percorso — coda, worker, job, stato,
/// revisione — prima che un fornitore esista, e quel percorso è ciò che questa
/// fetta compra. Il fornitore vero non è scelto, e la ragione non è tecnica: vedi
/// [`OcrProvider`].
///
/// # Come legge
///
/// Tratta il documento come testo e cerca righe nella forma `Etichetta: valore`,
/// che è la forma in cui una bolletta italiana stampa i propri campi. Per ogni
/// riga:
///
/// 1. l'**etichetta** — ciò che precede il primo `:` — si normalizza tenendo solo
///    le lettere, minuscole, e si confronta con quelle note;
/// 2. il **valore** si interpreta secondo il tipo del campo. Se non si interpreta,
///    il campo **non viene proposto**: un campo assente è una cosa che si vede,
///    un campo con dentro testo non interpretabile no;
/// 3. la **confidenza** è `1 − (caratteri illeggibili / caratteri della riga)`.
///
/// # Perché la confidenza si calcola così
///
/// Perché serviva un modo *deterministico* di produrre le tre uscite che i test
/// devono coprire — lettura buona, lettura a confidenza bassa, fallimento — senza
/// un interruttore finto nel corpo dell'adattatore. Una riga pulita dà `1.0000`;
/// una riga con dentro caratteri che una scansione sporca produrrebbe dà meno, in
/// proporzione a quanti sono; un documento in cui non si riconosce **nessuna**
/// etichetta è un rifiuto.
///
/// Un fornitore vero non calcolerà la confidenza così — la calcolerà come vuole
/// lui — e non importa: ciò che deve restare vero è che il numero esca dalla porta
/// e arrivi fino a `ocr_confidence`, e questo qui lo verifica.
///
/// # Cosa questo adattatore non fa
///
/// Non fa OCR: non guarda un'immagine. Un PDF o un JPEG lo rifiuta come
/// [`OcrError::UnsupportedDocument`], che è la verità — non sa leggerli — e che è
/// anche il ramo di errore che i test devono poter esercitare.
pub struct DeterministicOcrProvider;

impl DeterministicOcrProvider {
    /// Il nome che finisce in `ai_jobs.provider_name`.
    ///
    /// Esplicito e non «test» o «fake»: una riga di `ai_jobs` deve dire quale
    /// macchina ha prodotto quella lettura, e il giorno in cui un fornitore vero
    /// affiancherà questo, distinguere le due popolazioni di job sarà la prima
    /// cosa che serve per misurare l'accuratezza.
    pub const NAME: &'static str = "deterministic";

    /// Le etichette riconosciute, già normalizzate.
    ///
    /// Più di una per campo dove le bollette italiane non concordano: `Totale` e
    /// `Importo totale` sono lo stesso campo per chi legge, e imporne una sola
    /// significherebbe non leggere metà delle bollette.
    const LABELS: &'static [(&'static str, BillField)] = &[
        ("fornitore", BillField::SupplierName),
        ("partitaiva", BillField::SupplierVat),
        ("piva", BillField::SupplierVat),
        ("numerofattura", BillField::InvoiceNumber),
        ("fattura", BillField::InvoiceNumber),
        ("dataemissione", BillField::IssueDate),
        ("emissione", BillField::IssueDate),
        ("scadenza", BillField::DueDate),
        ("periododal", BillField::PeriodStart),
        ("periodoal", BillField::PeriodEnd),
        ("totale", BillField::TotalAmount),
        ("importototale", BillField::TotalAmount),
        ("imponibile", BillField::TaxableAmount),
        ("iva", BillField::VatAmount),
        ("consumo", BillField::Consumption),
    ];

    /// Interpreta il valore secondo il tipo del campo.
    ///
    /// `None` quando il valore non si interpreta: il campo si salta invece di
    /// proporre una stringa che chi rivede dovrebbe decifrare.
    fn parse_value(field: BillField, raw: &str) -> Option<ExtractedValue> {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }
        match field {
            BillField::SupplierName | BillField::SupplierVat | BillField::InvoiceNumber => {
                Some(ExtractedValue::Text(raw.to_owned()))
            }
            BillField::IssueDate
            | BillField::DueDate
            | BillField::PeriodStart
            | BillField::PeriodEnd => Self::parse_date(raw).map(ExtractedValue::Date),
            BillField::TotalAmount | BillField::TaxableAmount | BillField::VatAmount => {
                Self::parse_amount(raw).map(ExtractedValue::Amount)
            }
            BillField::Consumption => {
                // Valore e unità viaggiano insieme: `123.4567` da solo non dice
                // niente, ed è la ragione per cui `Consumption` esiste.
                let (value, unit) = raw.split_once(char::is_whitespace)?;
                let value = Self::parse_amount(value)?;
                let unit = unit.trim();
                (!unit.is_empty()).then(|| ExtractedValue::Quantity {
                    value,
                    unit: unit.to_owned(),
                })
            }
        }
    }

    /// `gg/mm/aaaa` come le stampano le bollette italiane, oppure ISO.
    fn parse_date(raw: &str) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(raw, "%d/%m/%Y")
            .or_else(|_| NaiveDate::parse_from_str(raw, "%Y-%m-%d"))
            .ok()
    }

    /// Un importo come lo scrive una fattura italiana: virgola decimale,
    /// eventuali punti di migliaia, eventuale simbolo di valuta.
    ///
    /// Il simbolo si **scarta** invece di leggerlo: la valuta resta `EUR` per
    /// tutto il modulo contabile (ADR-030), e proporre una valuta che il resto
    /// del sistema non sa reggere sarebbe peggio che non proporla.
    fn parse_amount(raw: &str) -> Option<Decimal> {
        let cleaned: String = raw
            .chars()
            .filter(|character| {
                character.is_ascii_digit() || *character == ',' || *character == '.'
            })
            .collect();
        // Con entrambi i separatori il punto è quello delle migliaia:
        // `1.234,56`. Con la sola virgola, è il decimale.
        let normalized = if cleaned.contains(',') {
            cleaned.replace('.', "").replace(',', ".")
        } else {
            cleaned
        };
        let value = Decimal::from_str(&normalized).ok()?;
        (!value.is_sign_negative()).then_some(value)
    }

    /// `1 − (caratteri illeggibili / caratteri della riga)`, a quattro decimali.
    ///
    /// «Illeggibile» è tutto ciò che su una bolletta non ci sarebbe: lettere,
    /// cifre, spazi e la punteggiatura che serve a numeri e date sono puliti, il
    /// resto è sporco di scansione.
    fn line_confidence(line: &str) -> Confidence {
        let total = line.chars().count();
        if total == 0 {
            return Confidence::from_persistence(Decimal::ONE);
        }
        let noise = line
            .chars()
            .filter(|character| {
                !(character.is_alphanumeric()
                    || character.is_whitespace()
                    || matches!(
                        character,
                        ':' | '.' | ',' | '/' | '-' | '€' | '%' | '(' | ')' | '\''
                    ))
            })
            .count();
        let ratio = Decimal::from(noise) / Decimal::from(total);
        // `round_dp` e non un troncamento: la colonna è `NUMERIC(5,4)` e un
        // quinto decimale verrebbe arrotondato dal database comunque, ma allora
        // il valore che il dominio ha validato non sarebbe quello scritto.
        let confidence = (Decimal::ONE - ratio).round_dp(4);
        Confidence::from_persistence(confidence.max(Decimal::ZERO))
    }
}

#[async_trait]
impl OcrProvider for DeterministicOcrProvider {
    fn name(&self) -> &str {
        Self::NAME
    }

    async fn read(&self, content: &[u8], mime_type: &str) -> Result<OcrOutcome, OcrError> {
        let text = std::str::from_utf8(content).map_err(|_| OcrError::UnsupportedDocument {
            details: format!("`{mime_type}` non è un documento di testo leggibile"),
        })?;

        let mut fields: Vec<ExtractedField> = Vec::new();
        for line in text.lines() {
            let Some((label, value)) = line.split_once(':') else {
                continue;
            };
            let normalized: String = label
                .chars()
                .filter(|character| character.is_alphabetic())
                .flat_map(char::to_lowercase)
                .collect();
            let Some((_, field)) = Self::LABELS
                .iter()
                .find(|(candidate, _)| *candidate == normalized)
            else {
                continue;
            };
            // Il primo che vince: una bolletta che stampa due volte lo stesso
            // campo lo ha stampato una volta in testa e una nel riepilogo, e la
            // prima occorrenza è quella buona.
            if fields.iter().any(|found| found.field == *field) {
                continue;
            }
            let Some(value) = Self::parse_value(*field, value) else {
                continue;
            };
            fields.push(ExtractedField {
                field: *field,
                value,
                confidence: Self::line_confidence(line),
            });
        }

        if fields.is_empty() {
            // Nessuna etichetta riconosciuta: non è una lettura vuota, è un
            // documento che questo fornitore non sa leggere. La distinzione conta,
            // perché la prima lascia la bolletta in revisione e la seconda la
            // lascia in bozza.
            return Err(OcrError::UnsupportedDocument {
                details: format!("nessun campo riconosciuto in un documento `{mime_type}`"),
            });
        }

        Ok(OcrOutcome {
            reading: OcrReading::new(fields),
            usage: ProviderUsage {
                model: Some("rule-based/1".to_owned()),
                provider_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
                // Nessun token e **nessun costo**: questo adattatore non chiama
                // nessuno. `ai_jobs.cost_amount` resta nullo invece di dichiarare
                // uno zero che sembrerebbe una misura.
                tokens_input: None,
                tokens_output: None,
                cost: None,
                latency_ms: Some(0),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLEAN: &str = "\
Fornitore: Enel Energia
Partita IVA: IT12345678901
Numero fattura: 2026/A/00417
Data emissione: 01/08/2026
Scadenza: 31/08/2026
Periodo dal: 01/06/2026
Periodo al: 31/07/2026
Imponibile: 151,00
IVA: 33,20
Totale: 184,20 €
Consumo: 123,4567 Smc
";

    async fn read(content: &str) -> Result<OcrOutcome, OcrError> {
        DeterministicOcrProvider
            .read(content.as_bytes(), "text/plain")
            .await
    }

    fn value(outcome: &OcrOutcome, field: BillField) -> Option<&ExtractedValue> {
        outcome
            .reading
            .fields()
            .iter()
            .find(|found| found.field == field)
            .map(|found| &found.value)
    }

    fn dec(raw: &str) -> Decimal {
        Decimal::from_str(raw).expect("decimale di test valido")
    }

    /// Prima uscita: una scansione pulita.
    #[tokio::test]
    async fn a_clean_scan_is_read_completely_and_with_full_confidence() {
        let outcome = read(CLEAN).await.expect("lettura riuscita");
        assert_eq!(outcome.reading.fields().len(), BillField::ALL.len());
        assert_eq!(
            outcome.reading.overall_confidence(),
            Confidence::new(Decimal::ONE).ok()
        );
        assert_eq!(
            value(&outcome, BillField::SupplierName),
            Some(&ExtractedValue::Text("Enel Energia".to_owned()))
        );
        assert_eq!(
            value(&outcome, BillField::TotalAmount),
            Some(&ExtractedValue::Amount(dec("184.20")))
        );
        assert_eq!(
            value(&outcome, BillField::DueDate),
            Some(&ExtractedValue::Date(
                NaiveDate::from_ymd_opt(2026, 8, 31).expect("data valida")
            ))
        );
        assert_eq!(
            value(&outcome, BillField::Consumption),
            Some(&ExtractedValue::Quantity {
                value: dec("123.4567"),
                unit: "Smc".to_owned(),
            })
        );
    }

    /// Seconda uscita: la scansione sporca. Il campo si legge lo stesso, e la
    /// confidenza scende — che è il segnale per chi rivede, non un errore.
    #[tokio::test]
    async fn a_smudged_line_is_read_with_less_confidence() {
        let outcome = read("Fornitore: Enel Energia\nTotale⌗⌗⌗⌗: 184,20\n")
            .await
            .expect("lettura riuscita anche se sporca");

        assert_eq!(
            value(&outcome, BillField::TotalAmount),
            Some(&ExtractedValue::Amount(dec("184.20"))),
            "il valore si legge comunque"
        );
        let overall = outcome
            .reading
            .overall_confidence()
            .expect("una confidenza c'è");
        assert!(
            overall.value() < Decimal::ONE,
            "la confidenza deve scendere: {overall}"
        );
        // È il campo peggiore a dettare il riassunto, non la media.
        let worst = outcome
            .reading
            .fields()
            .iter()
            .find(|found| found.field == BillField::TotalAmount)
            .expect("il totale c'è");
        assert_eq!(overall, worst.confidence);
    }

    /// Terza uscita: il rifiuto. Sono i due modi in cui questo adattatore non sa
    /// leggere, e diventano lo stesso `error_code`.
    #[tokio::test]
    async fn a_document_it_cannot_read_is_refused_and_not_returned_empty() {
        let binary = DeterministicOcrProvider
            .read(&[0xFF, 0xD8, 0xFF, 0xE0], "image/jpeg")
            .await;
        assert!(matches!(binary, Err(OcrError::UnsupportedDocument { .. })));

        let prose = read("Questa non è una bolletta, è una lettera.\n").await;
        assert!(matches!(prose, Err(OcrError::UnsupportedDocument { .. })));
    }

    /// Un valore che non si interpreta **non si propone**: meglio un campo che
    /// manca di un campo che chi rivede deve decifrare.
    #[tokio::test]
    async fn a_value_that_does_not_parse_is_left_out_rather_than_guessed() {
        let outcome =
            read("Fornitore: Enel Energia\nTotale: centottantaquattro euro\nScadenza: presto\n")
                .await
                .expect("il fornitore basta a non far fallire la lettura");
        assert_eq!(outcome.reading.fields().len(), 1);
        assert_eq!(value(&outcome, BillField::TotalAmount), None);
        assert_eq!(value(&outcome, BillField::DueDate), None);
    }

    /// Le migliaia con il punto e i decimali con la virgola: `1.234,56` è
    /// milleduecentotrentaquattro, non uno virgola due.
    #[test]
    fn an_italian_amount_keeps_its_thousands_separator_straight() {
        assert_eq!(
            DeterministicOcrProvider::parse_amount("1.234,56"),
            Some(dec("1234.56"))
        );
        assert_eq!(
            DeterministicOcrProvider::parse_amount("184,20 €"),
            Some(dec("184.20"))
        );
        assert_eq!(
            DeterministicOcrProvider::parse_amount("184.20"),
            Some(dec("184.20"))
        );
    }

    /// Deterministico vuol dire questo: due letture dello stesso documento danno
    /// lo stesso risultato. Senza, i test sarebbero aneddoti.
    #[tokio::test]
    async fn the_same_document_always_reads_the_same() {
        let first = read(CLEAN).await.expect("lettura");
        let second = read(CLEAN).await.expect("lettura");
        assert_eq!(first.reading, second.reading);
    }
}
