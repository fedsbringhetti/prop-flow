use std::sync::Arc;

use propflow_domain::events::{
    DomainEvent, DomainEventPayload, EventMetadata, TicketMessagePosted,
};
use propflow_domain::ports::{Clock, TicketMessagePage, TicketMessageQuery, UnitOfWorkFactory};
use propflow_domain::shared::{TicketId, TicketMessageId};
use propflow_domain::ticket::{MessageBody, NewTicketMessage, TicketActor, TicketMessage};
use propflow_domain::{Denial, Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct PostTicketMessageCommand {
    pub ticket_id: TicketId,
    pub body: MessageBody,
    /// Nota interna: visibile al solo staff dell'organizzazione.
    pub is_internal: bool,
}

/// Scrive un messaggio nel thread di una segnalazione.
///
/// **Una nota interna richiede lo staff, e il controllo va fatto qui.** La policy
/// `ticket_messages_insert` lo ripete
/// (`NOT is_internal OR app.is_org_staff(org_id)`), ma se il service layer non lo
/// anticipasse il rifiuto arriverebbe dalla Row Level Security come
/// `403 auth.denied_by_policy`, che è il codice riservato ai **nostri** difetti:
/// un inquilino che tenta di scrivere una nota interna non è un nostro difetto, è
/// una richiesta da rifiutare dicendogli cosa gli manca.
pub struct PostTicketMessage {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl PostTicketMessage {
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
        command: PostTicketMessageCommand,
    ) -> AppResult<TicketMessage> {
        let org_id = authorization::ensure_member(context)?;
        if command.is_internal && !context.is_staff() {
            return Err(AppError::Forbidden(Denial::StaffRoleRequired));
        }
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        // La segnalazione si legge prima di scrivere: un `ticket_id` non visibile
        // deve dare `404` e non il `403` della policy di riga, che direbbe al
        // chiamante che il problema è suo quando invece la segnalazione, per lui,
        // non esiste.
        let ticket = uow
            .tickets()
            .find_by_id(command.ticket_id)
            .await?
            .ok_or(AppError::not_found(Entity::Ticket))?;

        let message = TicketMessage::post(NewTicketMessage {
            id: TicketMessageId::generate(),
            org_id,
            ticket_id: ticket.id(),
            author_id: context.user_id(),
            author_kind: TicketActor::from_role(authorization::effective_role(context)),
            body: command.body,
            is_internal: command.is_internal,
            now,
        });

        uow.ticket_messages().insert(&message).await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::TicketMessagePosted(TicketMessagePosted::from_message(
                    &message,
                )),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(message)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ListTicketMessagesCommand {
    pub ticket_id: TicketId,
    pub after: Option<TicketMessageId>,
    pub limit: Option<u16>,
}

/// Elenca i messaggi di una segnalazione.
///
/// **Le note interne non si filtrano qui.** Lo fa la policy
/// `ticket_messages_select`, che ha `NOT is_internal OR app.is_org_staff(org_id)`:
/// un filtro applicativo sarebbe una seconda definizione della stessa regola, e le
/// due divergerebbero alla prima modifica di una delle due.
pub struct ListTicketMessages {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl ListTicketMessages {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        command: ListTicketMessagesCommand,
    ) -> AppResult<TicketMessagePage> {
        authorization::ensure_member(context)?;

        let limit = command
            .limit
            .unwrap_or(TicketMessageQuery::DEFAULT_LIMIT)
            .clamp(1, TicketMessageQuery::MAX_LIMIT);

        let mut uow = self.unit_of_work.begin(context).await?;
        // Una segnalazione non visibile dà `404` invece di un thread vuoto: un
        // elenco vuoto sulla segnalazione di un altro suggerirebbe che esiste e
        // che nessuno ci ha ancora scritto.
        let ticket = uow
            .tickets()
            .find_by_id(command.ticket_id)
            .await?
            .ok_or(AppError::not_found(Entity::Ticket))?;

        let page = uow
            .ticket_messages()
            .list_for_ticket(&TicketMessageQuery {
                ticket_id: ticket.id(),
                after: command.after,
                limit,
            })
            .await?;
        Ok(page)
    }
}
