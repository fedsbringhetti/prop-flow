use std::sync::Arc;

use propflow_domain::document::{
    AttachmentOwner, Checksum, Document, DocumentKind, FileName, MimeType, NewDocument, ObjectSize,
    StorageLocation,
};
use propflow_domain::events::{DocumentUploaded, DomainEvent, DomainEventPayload, EventMetadata};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::DocumentId;
use propflow_domain::{DomainError, RequestContext};

use super::owner_access::ensure_owner_is_visible;
use crate::authorization;
use crate::error::AppResult;
use crate::ports::ObjectStorage;

/// Comando di caricamento.
///
/// `content` sono i byte già in memoria: il limite di corpo della rotta li tiene
/// entro una dimensione nota, e lo streaming verso lo storage è ciò che si
/// guadagna passando in seguito a un `PUT` pre-firmato — che però renderebbe MIME
/// e checksum dichiarazioni del client (vedi il piano in `docs/09`).
pub struct UploadDocumentCommand {
    pub owner: AttachmentOwner,
    pub kind: DocumentKind,
    pub original_filename: String,
    pub mime_type: String,
    pub content: Vec<u8>,
}

/// Carica un documento e ne registra i metadati.
pub struct UploadDocument {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    storage: Arc<dyn ObjectStorage>,
    clock: Arc<dyn Clock>,
}

impl UploadDocument {
    #[must_use]
    pub const fn new(
        unit_of_work: Arc<dyn UnitOfWorkFactory>,
        storage: Arc<dyn ObjectStorage>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            unit_of_work,
            storage,
            clock,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        command: UploadDocumentCommand,
    ) -> AppResult<Document> {
        // `ensure_member` e non `ensure_staff`: l'inquilino allega al **proprio**
        // contratto, ed è precisamente ciò che `documents_insert` ammette con
        // `can_access_attachment`. Quali oggetti siano suoi lo decide la RLS.
        let org_id = authorization::ensure_member(context)?;
        let now = self.clock.now();

        // I value object si costruiscono prima di toccare lo storage: un MIME
        // malformato deve dare 422 senza aver scritto un oggetto che nessuna riga
        // referenzierà mai.
        let original_filename = FileName::parse(&command.original_filename)?;
        let mime_type = MimeType::parse(&command.mime_type)?;
        if command.content.is_empty() {
            return Err(DomainError::validation("content", "is required").into());
        }

        let id = DocumentId::generate();
        let key = Document::object_key(org_id, id);

        // L'owner si verifica **prima** di scrivere l'oggetto: un 404 non deve
        // lasciare byte nel bucket. La transazione si chiude subito dopo, perché
        // tenerla aperta durante la chiamata di rete allo storage occuperebbe una
        // connessione del pool per tutta la durata dell'upload.
        {
            let mut uow = self.unit_of_work.begin(context).await?;
            ensure_owner_is_visible(uow.as_mut(), command.owner).await?;
        }

        // L'oggetto si scrive **prima** della transazione che registra la riga.
        // L'ordine non è indifferente: se il commit fallisce resta un oggetto
        // orfano, che è recuperabile con una scansione del bucket; l'ordine
        // inverso lascerebbe una riga che punta al nulla, cioè un documento che
        // l'API elenca e non sa scaricare.
        let stored = self
            .storage
            .put(&key, mime_type.as_str(), command.content)
            .await?;

        let document = Document::upload(NewDocument {
            id,
            org_id,
            kind: command.kind,
            owner: command.owner,
            storage: StorageLocation::new(&stored.provider, &stored.bucket, &stored.key)?,
            mime_type,
            size_bytes: ObjectSize::new(stored.size_bytes)?,
            checksum: Checksum::from_bytes(stored.checksum_sha256),
            original_filename,
            encryption_key_ref: stored.encryption,
            uploaded_by: context.user_id(),
            now,
        })?;

        let mut uow = self.unit_of_work.begin(context).await?;
        // Ricontrollato dentro la transazione che scrive: fra la verifica di prima
        // e questa l'owner potrebbe essere stato archiviato. Costa una lettura e
        // trasforma un `403 auth.denied_by_policy` in un `404` onesto.
        ensure_owner_is_visible(uow.as_mut(), command.owner).await?;
        uow.documents().insert(&document).await?;
        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::DocumentUploaded(DocumentUploaded::new(
                    document.id(),
                    org_id,
                    document.owner(),
                    document.kind(),
                )),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        uow.commit().await?;

        Ok(document)
    }
}
