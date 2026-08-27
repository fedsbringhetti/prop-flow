use std::sync::Arc;

use propflow_domain::document::Document;
use propflow_domain::ports::UnitOfWorkFactory;
use propflow_domain::shared::DocumentId;
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult};
use crate::ports::ObjectStorage;

/// URL pre-firmato con cui il client scarica direttamente dallo storage.
#[derive(Debug, Clone)]
pub struct DocumentDownload {
    pub url: String,
    pub expires_in_seconds: u32,
}

/// Produce l'URL di download di un documento.
///
/// Il contenuto **non transita dall'API**: l'asimmetria con l'upload è
/// deliberata. In caricamento i byte devono passare da noi perché il checksum lo
/// calcoliamo noi; in scaricamento non c'è nulla da verificare, e un URL
/// pre-firmato toglie dall'API il traffico più pesante.
///
/// L'autorizzazione avviene **prima** della firma e non dopo: l'URL firmato è un
/// permesso trasferibile e a tempo, quindi chi lo ottiene lo ottiene per sempre,
/// finché scade. Da qui il TTL corto (`[storage] presign_ttl_seconds`).
pub struct DownloadDocument {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    storage: Arc<dyn ObjectStorage>,
    ttl_seconds: u32,
}

impl DownloadDocument {
    #[must_use]
    pub const fn new(
        unit_of_work: Arc<dyn UnitOfWorkFactory>,
        storage: Arc<dyn ObjectStorage>,
        ttl_seconds: u32,
    ) -> Self {
        Self {
            unit_of_work,
            storage,
            ttl_seconds,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        id: DocumentId,
    ) -> AppResult<(Document, DocumentDownload)> {
        authorization::ensure_member(context)?;

        let document = {
            let mut uow = self.unit_of_work.begin(context).await?;
            uow.documents()
                .find_by_id(id)
                .await?
                .ok_or(AppError::not_found(Entity::Document))?
        };

        let url = self
            .storage
            .presigned_get_url(document.storage().object_key(), self.ttl_seconds)
            .await?;

        Ok((
            document,
            DocumentDownload {
                url,
                expires_in_seconds: self.ttl_seconds,
            },
        ))
    }
}
