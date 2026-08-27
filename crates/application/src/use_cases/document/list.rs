use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::document::AttachmentOwner;
use propflow_domain::ports::{DocumentPage, DocumentQuery, UnitOfWorkFactory};
use propflow_domain::shared::DocumentId;

use super::owner_access::ensure_owner_is_visible;
use crate::authorization;
use crate::error::AppResult;

#[derive(Debug, Clone, Copy)]
pub struct ListDocumentsCommand {
    pub owner: AttachmentOwner,
    pub after: Option<DocumentId>,
    pub limit: Option<u16>,
}

/// Elenca i documenti allegati a un oggetto.
///
/// `ensure_member`: l'inquilino vede i documenti del proprio contratto e del
/// proprio immobile, non gli altri della stessa organizzazione. La distinzione la
/// fa `app.can_access_attachment` attraverso le policy, non un ramo applicativo
/// sul ruolo.
pub struct ListDocuments {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl ListDocuments {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        command: ListDocumentsCommand,
    ) -> AppResult<DocumentPage> {
        authorization::ensure_member(context)?;

        // Il limite si **tronca** invece di essere rifiutato: un client che chiede
        // 10 000 righe ottiene la pagina massima, non un errore da gestire. Stessa
        // regola di immobili e contratti (`docs/05-api.md` §Paginazione).
        let limit = command
            .limit
            .unwrap_or(DocumentQuery::DEFAULT_LIMIT)
            .clamp(1, DocumentQuery::MAX_LIMIT);

        let mut uow = self.unit_of_work.begin(context).await?;
        // Un owner non visibile dà 404 invece di un elenco vuoto: una lista vuota
        // sul contratto di un altro suggerirebbe che quel contratto esiste e non
        // ha allegati.
        ensure_owner_is_visible(uow.as_mut(), command.owner).await?;

        let page = uow
            .documents()
            .list_by_owner(&DocumentQuery {
                owner: command.owner,
                after: command.after,
                limit,
            })
            .await?;
        Ok(page)
    }
}
