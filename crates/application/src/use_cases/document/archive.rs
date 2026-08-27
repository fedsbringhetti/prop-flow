use std::sync::Arc;

use propflow_domain::events::{DocumentArchived, DomainEvent, DomainEventPayload, EventMetadata};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::DocumentId;
use propflow_domain::{Denial, Entity, RequestContext};

use super::owner_access::can_archive;
use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy)]
pub struct ArchiveDocumentCommand {
    pub id: DocumentId,
}

/// Archivia un documento: soft delete della riga.
///
/// **L'oggetto nel bucket resta.** Il ruolo applicativo non ha il privilegio
/// `DELETE` e la porta `ObjectStorage` non espone una cancellazione: rimuovere i
/// byte è un lavoro separato, da pensare insieme al purge di `retention_until`,
/// che lo schema prevede come job applicativo e non come trigger. È registrato fra
/// i debiti noti.
pub struct ArchiveDocument {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl ArchiveDocument {
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
        command: ArchiveDocumentCommand,
    ) -> AppResult<()> {
        let org_id = authorization::ensure_member(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut document = uow
            .documents()
            .find_by_id(command.id)
            .await?
            .ok_or(AppError::not_found(Entity::Document))?;

        // Stesso criterio della `USING` di `documents_update`. Senza, il rifiuto
        // arriverebbe dalla RLS come `auth.denied_by_policy`, che significa
        // "difetto nostro"; `org.staff_role_required` dice invece al chiamante che
        // cosa gli manca — e per un documento che non ha caricato lui, è
        // esattamente il ruolo di staff.
        if !can_archive(context, document.uploaded_by()) {
            return Err(AppError::Forbidden(Denial::StaffRoleRequired));
        }

        document.archive(now)?;
        uow.documents().archive(&document).await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::DocumentArchived(DocumentArchived::new(
                    document.id(),
                    org_id,
                    document.owner(),
                    authorization::effective_role(context),
                )),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(())
    }
}
