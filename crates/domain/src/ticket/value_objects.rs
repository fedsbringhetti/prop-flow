use std::fmt;

use crate::error::DomainResult;
use crate::shared::text;

/// Titolo della segnalazione.
///
/// I limiti replicano il `CHECK` dello schema
/// (`length(btrim(title)) BETWEEN 1 AND 200`): qui servono a restituire un `422`
/// leggibile invece di una violazione di vincolo.
///
/// **Non esiste un modo di cambiarlo dopo l'apertura**, e non è una dimenticanza:
/// il titolo è il testo con cui una persona ha descritto un guasto in un momento
/// preciso, e il thread esiste apposta per correggere il tiro. Un titolo riscritto
/// dopo tre messaggi rende incomprensibili i tre messaggi. La migration `0020`
/// rende la cosa strutturale togliendo la colonna dalla `GRANT`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketTitle(String);

impl TicketTitle {
    pub const MAX_LENGTH: usize = 200;

    pub fn parse(value: &str) -> DomainResult<Self> {
        Ok(Self(text::required("title", value, Self::MAX_LENGTH)?))
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

impl fmt::Display for TicketTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Descrizione del guasto, così come l'ha scritta chi lo ha subito.
///
/// Obbligatoria (`description TEXT NOT NULL`) e senza un tetto a schema: il tetto
/// è nostro e serve a non accettare un corpo arbitrariamente lungo, non a far
/// rispettare un vincolo del database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketDescription(String);

impl TicketDescription {
    pub const MAX_LENGTH: usize = 10_000;

    pub fn parse(value: &str) -> DomainResult<Self> {
        Ok(Self(text::required(
            "description",
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

/// Corpo di un messaggio del thread.
///
/// Stesso tetto della descrizione, e per la stessa ragione: lo schema chiede solo
/// che non sia vuoto (`length(btrim(body)) > 0`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageBody(String);

impl MessageBody {
    pub const MAX_LENGTH: usize = 10_000;

    pub fn parse(value: &str) -> DomainResult<Self> {
        Ok(Self(text::required("body", value, Self::MAX_LENGTH)?))
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

/// Riferimento leggibile della segnalazione: `TK-000042`.
///
/// **È l'unico identificatore del progetto che il dominio non sa produrre.** Ogni
/// altro nasce qui con `generate()`; questo lo assegna il database, che ha come
/// `DEFAULT` `'TK-' || to_char(nextval('app.ticket_reference_seq'), 'FM000000')`.
/// Per questo il tipo ha un `from_persistence` e **nessun** costruttore: generarlo
/// nell'applicazione lo metterebbe in competizione con la sequenza, e il primo
/// `INSERT` che non lo specificasse la farebbe avanzare comunque, producendo
/// collisioni sull'indice unico `tickets_reference_uk` a distanza di mesi dal
/// cambiamento.
///
/// La conseguenza è che [`super::Ticket::open`] restituisce un aggregato con
/// `reference` a `None`, riempito dal repository subito dopo la scrittura — con
/// una rilettura e **non** con un `RETURNING`, che sotto Row Level Security
/// fallisce: vedi `PgTicketRepository::insert`.
///
/// **Nota da non nascondere:** la sequenza è globale, non per organizzazione.
/// `TK-000042` dice a chiunque quante segnalazioni esistono sull'intera
/// piattaforma, non nella propria organizzazione.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketReference(String);

impl TicketReference {
    #[must_use]
    pub const fn from_persistence(value: String) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TicketReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_is_trimmed_and_bounded_like_the_schema() {
        assert_eq!(
            TicketTitle::parse("  Il boiler perde  ")
                .expect("valido")
                .as_str(),
            "Il boiler perde"
        );
        assert!(TicketTitle::parse("").is_err());
        assert!(TicketTitle::parse("   ").is_err());
        assert!(TicketTitle::parse(&"a".repeat(201)).is_err());
        assert!(TicketTitle::parse(&"a".repeat(200)).is_ok());
    }

    #[test]
    fn description_and_body_refuse_the_empty_string() {
        assert!(TicketDescription::parse("  ").is_err());
        assert!(MessageBody::parse("").is_err());
        assert_eq!(
            MessageBody::parse(" arrivo domani ")
                .expect("valido")
                .as_str(),
            "arrivo domani"
        );
    }

    /// Il tipo esiste per **non** poter essere costruito: se qualcuno aggiungesse
    /// un `parse` o un `generate`, questo commento resterebbe l'unico avviso. Il
    /// test fissa invece l'unica via ammessa.
    #[test]
    fn a_reference_only_comes_back_from_the_database() {
        let reference = TicketReference::from_persistence("TK-000042".to_owned());
        assert_eq!(reference.as_str(), "TK-000042");
        assert_eq!(reference.to_string(), "TK-000042");
    }
}
