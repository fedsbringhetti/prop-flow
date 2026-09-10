use std::sync::Arc;

use propflow_domain::events::{
    DomainEvent, DomainEventPayload, EventMetadata, TicketReportCorrected,
};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::TicketId;
use propflow_domain::ticket::TicketCorrection;
use propflow_domain::{Entity, RequestContext};

use super::read::{TicketView, view_of};
use super::transition::reporter_while_open;
use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct CorrectTicketReportCommand {
    pub id: TicketId,
    pub correction: TicketCorrection,
}

/// Corregge ciò che è stato dichiarato aprendo: titolo, descrizione e la scelta
/// sulla visibilità.
///
/// # Perché non è il `PATCH` dei costi allargato
///
/// Sarebbe una rotta con **due autorizzazioni diverse secondo i campi presenti** —
/// i costi allo staff, il testo a chi ha segnalato — e con la semantica del
/// `PATCH` i campi sono *assenti*: l'autorizzazione dipenderebbe da quali chiavi
/// ci sono nel JSON, e un campo aggiunto un anno dopo senza toccare quel ramo
/// sarebbe scrivibile da chi non deve. È un difetto muto, che non compare in
/// nessun test finché qualcuno non manda quella combinazione.
///
/// Qui l'autorizzazione è **una sola per l'intera rotta**, e i tre campi stanno
/// insieme perché sono tutti e tre ciò che chi ha segnalato ha dichiarato. Vedi
/// [ADR-048](../../../../../docs/08-decisioni.md).
///
/// # Chi corregge
///
/// Lo staff sempre, chi ha segnalato finché è `open`: [`reporter_while_open`], la
/// stessa funzione dell'annullamento e non una copia.
///
/// # Non è una transizione
///
/// Non passa da `run()` e non emette `TicketStatusChanged`, perché nessuno stato
/// si muove. L'evento è `TicketReportCorrected`, che porta i **nomi** dei campi
/// toccati e non i valori: titolo e descrizione sono testo scritto da una persona,
/// e non finiscono su un broker.
///
/// # Il privilegio, che fino alla `0036` non c'era
///
/// `title` e `description` non erano scrivibili dal ruolo applicativo: la
/// migration `0036` le riconcede. Senza di essa questo caso d'uso compilerebbe e
/// la scrittura fallirebbe con `permission denied` nei test di integrazione.
pub struct CorrectTicketReport {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl CorrectTicketReport {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>, clock: Arc<dyn Clock>) -> Self {
        Self {
            unit_of_work,
            clock,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        command: CorrectTicketReportCommand,
    ) -> AppResult<TicketView> {
        let org_id = authorization::ensure_member(context)?;
        let now = self.clock.now();
        let changed_fields = changed_field_names(&command.correction);

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut ticket = uow
            .tickets()
            .find_by_id(command.id)
            .await?
            .ok_or(AppError::not_found(Entity::Ticket))?;

        // Dopo la lettura e prima di qualunque scrittura: sapere se chi chiama ha
        // segnalato richiede di aver letto la riga, come per le transizioni.
        reporter_while_open(context, &ticket)?;

        let changed = ticket.correct(command.correction, now)?;
        if !changed {
            // Nessun campo toccato: niente UPDATE, niente evento, niente riga di
            // audit. Un PATCH vuoto non è un errore, ma non è nemmeno una
            // modifica — è la stessa scelta di `UpdateTicketCosts`.
            return view_of(&mut *uow, ticket).await;
        }

        uow.tickets().update(&ticket).await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::TicketReportCorrected(TicketReportCorrected {
                    ticket_id: ticket.id(),
                    org_id,
                    changed_fields,
                }),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;

        let view = view_of(&mut *uow, ticket).await?;
        uow.commit().await?;

        Ok(view)
    }
}

/// I campi che il chiamante ha **nominato**, non quelli che sono cambiati.
///
/// È la stessa scelta di `TicketCosts::changed_field_names`, e la si nota solo
/// scrivendo lo stesso valore due volte: l'evento non viene emesso affatto se
/// nulla è cambiato, quindi la differenza non si vede da fuori.
fn changed_field_names(correction: &TicketCorrection) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if correction.title.is_some() {
        fields.push("title");
    }
    if correction.description.is_some() {
        fields.push("description");
    }
    if correction.visible_to_residents.is_some() {
        fields.push("visible_to_residents");
    }
    fields
}
