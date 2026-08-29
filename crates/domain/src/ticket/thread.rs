use chrono::{DateTime, Utc};

use super::aggregate::{TicketActor, TicketStatus};
use super::value_objects::MessageBody;
use crate::shared::{OrgId, TicketId, TicketMessageId, UserId};

/// Messaggio del thread di una segnalazione.
///
/// **Non si modifica e non si cancella, nemmeno in modo soft.** Non è una scelta
/// applicativa da rivedere: `app.ticket_messages` ha due sole policy, `_select` e
/// `_insert`, quindi un `UPDATE` viene rifiutato dalla Row Level Security. La
/// colonna `deleted_at` esiste sulla tabella e nessuno può valorizzarla. Un thread
/// di conversazione in cui le righe non si riscrivono è una proprietà, non un
/// limite — ma qualunque cosa prometta «cancella il messaggio» starebbe promettendo
/// ciò che il database rifiuta, e servirebbe una migration nuova sulla policy.
///
/// Per questo l'entità non ha metodi di mutazione: nasce e si legge.
#[derive(Debug, Clone)]
pub struct TicketMessage {
    id: TicketMessageId,
    org_id: OrgId,
    ticket_id: TicketId,
    author_id: Option<UserId>,
    author_kind: TicketActor,
    body: MessageBody,
    /// Nota interna, visibile al solo staff dell'organizzazione.
    ///
    /// È un campo del messaggio e non una risorsa separata: `is_internal` esiste
    /// già e la policy di SELECT lo filtra per l'inquilino. Una tabella separata
    /// avrebbe due thread da fondere in lettura e due punti dove sbagliare
    /// l'ordine.
    is_internal: bool,
    created_at: DateTime<Utc>,
}

impl TicketMessage {
    /// Scrive un messaggio nel thread.
    ///
    /// `is_internal` è un dato e non un permesso: chi può scriverne uno lo decide
    /// il service layer con `ensure_staff`, e la policy `ticket_messages_insert`
    /// lo ripete. Il dominio si limita a portarlo.
    #[must_use]
    pub fn post(parts: NewTicketMessage) -> Self {
        let NewTicketMessage {
            id,
            org_id,
            ticket_id,
            author_id,
            author_kind,
            body,
            is_internal,
            now,
        } = parts;
        Self {
            id,
            org_id,
            ticket_id,
            author_id,
            author_kind,
            body,
            is_internal,
            created_at: now,
        }
    }

    #[must_use]
    pub const fn id(&self) -> TicketMessageId {
        self.id
    }
    #[must_use]
    pub const fn org_id(&self) -> OrgId {
        self.org_id
    }
    #[must_use]
    pub const fn ticket_id(&self) -> TicketId {
        self.ticket_id
    }
    #[must_use]
    pub const fn author_id(&self) -> Option<UserId> {
        self.author_id
    }
    #[must_use]
    pub const fn author_kind(&self) -> TicketActor {
        self.author_kind
    }
    #[must_use]
    pub const fn body(&self) -> &MessageBody {
        &self.body
    }
    #[must_use]
    pub const fn is_internal(&self) -> bool {
        self.is_internal
    }
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Ricostruzione da persistenza. Coincide con [`TicketMessage::post`] perché
    /// il messaggio non ha invarianti oltre a quelle del corpo, che sono già state
    /// applicate quando la riga è stata scritta.
    #[must_use]
    pub fn from_persistence(parts: NewTicketMessage) -> Self {
        Self::post(parts)
    }
}

/// Dati di un messaggio, in scrittura come in lettura.
#[derive(Debug, Clone)]
pub struct NewTicketMessage {
    pub id: TicketMessageId,
    pub org_id: OrgId,
    pub ticket_id: TicketId,
    pub author_id: Option<UserId>,
    pub author_kind: TicketActor,
    pub body: MessageBody,
    pub is_internal: bool,
    pub now: DateTime<Utc>,
}

/// Una riga dello storico delle transizioni.
///
/// **Il dominio la legge e non la scrive.** La inserisce il trigger
/// `tg_ticket_status_history`, che scatta `AFTER INSERT OR UPDATE OF status`: ne
/// segue che l'apertura produce già da sé la riga `NULL → open`, e che aggiungerne
/// una dall'applicazione la raddoppierebbe. `app.ticket_status_history` non ha
/// nemmeno una policy di `INSERT`: una scrittura applicativa verrebbe rifiutata, e
/// il trigger funziona solo perché è `SECURITY DEFINER`.
///
/// `changed_by` lo riempie `app.current_user_id()`, che fuori da un contesto di
/// richiesta è nullo — ed è la ragione per cui il campo è opzionale.
#[derive(Debug, Clone)]
pub struct TicketStatusChange {
    ticket_id: TicketId,
    from_status: Option<TicketStatus>,
    to_status: TicketStatus,
    changed_by: Option<UserId>,
    reason: Option<String>,
    changed_at: DateTime<Utc>,
}

impl TicketStatusChange {
    /// Unico costruttore: lo storico entra nel dominio solo dalla persistenza.
    #[must_use]
    pub const fn from_persistence(
        ticket_id: TicketId,
        from_status: Option<TicketStatus>,
        to_status: TicketStatus,
        changed_by: Option<UserId>,
        reason: Option<String>,
        changed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            ticket_id,
            from_status,
            to_status,
            changed_by,
            reason,
            changed_at,
        }
    }

    #[must_use]
    pub const fn ticket_id(&self) -> TicketId {
        self.ticket_id
    }
    #[must_use]
    pub const fn from_status(&self) -> Option<TicketStatus> {
        self.from_status
    }
    #[must_use]
    pub const fn to_status(&self) -> TicketStatus {
        self.to_status
    }
    #[must_use]
    pub const fn changed_by(&self) -> Option<UserId> {
        self.changed_by
    }
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
    #[must_use]
    pub const fn changed_at(&self) -> DateTime<Utc> {
        self.changed_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(is_internal: bool) -> TicketMessage {
        TicketMessage::post(NewTicketMessage {
            id: TicketMessageId::generate(),
            org_id: OrgId::generate(),
            ticket_id: TicketId::generate(),
            author_id: Some(UserId::generate()),
            author_kind: TicketActor::Owner,
            body: MessageBody::parse("Passo domani mattina").expect("corpo valido"),
            is_internal,
            now: Utc::now(),
        })
    }

    #[test]
    fn a_message_carries_its_visibility() {
        assert!(!message(false).is_internal());
        assert!(message(true).is_internal());
    }

    /// Il test non verifica un comportamento ma l'**assenza** di uno: se qualcuno
    /// aggiungesse un `edit` o un `delete`, questo commento resterebbe l'unico
    /// avviso che il database li rifiuta.
    #[test]
    fn a_message_has_no_way_to_change_after_it_is_written() {
        let posted = message(false);
        let body = posted.body().as_str().to_owned();
        assert_eq!(posted.body().as_str(), body);
    }

    #[test]
    fn the_first_history_row_has_no_previous_status() {
        let change = TicketStatusChange::from_persistence(
            TicketId::generate(),
            None,
            TicketStatus::Open,
            None,
            None,
            Utc::now(),
        );
        assert_eq!(change.from_status(), None, "la scrive il trigger in INSERT");
        assert_eq!(change.to_status(), TicketStatus::Open);
    }
}
