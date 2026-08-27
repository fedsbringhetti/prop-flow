use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::document::{
    AttachmentOwner, Checksum, Document, DocumentParts, FileName, MimeType, ObjectSize,
    StorageLocation,
};
use propflow_domain::ports::{
    DocumentPage, DocumentQuery, DocumentRepository, RepositoryError, RepositoryResult,
};
use propflow_domain::shared::{DocumentId, OrgId, UserId};

use super::user::try_get;
use crate::db::db_enums;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

/// Colonne lette.
///
/// **Non ci sono `status`, `extracted_data`, `version`, `supersedes_document_id`
/// né `is_current`.** Non è una dimenticanza: OCR e versionamento sono fuori dal
/// perimetro di questa fetta (`docs/09-stato-e-roadmap.md`), il dominio non li
/// modella, e leggere una colonna che nessuno sa interpretare significa doverla
/// mappare su un tipo inventato per l'occasione.
macro_rules! select_document {
    () => {
        "
    SELECT id,
           org_id,
           kind::text       AS kind,
           owner_type::text AS owner_type,
           owner_id,
           storage_provider,
           storage_bucket,
           object_key,
           mime_type,
           size_bytes,
           checksum_sha256,
           original_filename,
           is_encrypted,
           encryption_key_ref,
           uploaded_by,
           created_at,
           updated_at,
           deleted_at
      FROM app.documents
"
    };
}

#[async_trait]
impl DocumentRepository for PgUnitOfWork {
    async fn find_by_id(&mut self, id: DocumentId) -> RepositoryResult<Option<Document>> {
        let row = sqlx::query(concat!(
            select_document!(),
            " WHERE id = $1 AND deleted_at IS NULL"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(map_document).transpose()
    }

    async fn list_by_owner(&mut self, query: &DocumentQuery) -> RepositoryResult<DocumentPage> {
        // `is_current` compare nel `WHERE` anche se il versionamento è fuori
        // perimetro: `documents_owner_ix` è **parziale** su
        // `deleted_at IS NULL AND is_current`, e una query che non li nomina
        // entrambi non lo usa. Oggi la colonna vale `true` per tutti, quindi il
        // predicato non cambia il risultato — cambia il piano.
        //
        // Il cursore keyset sfrutta l'ordinamento degli UUIDv7: ordinare per id
        // equivale a ordinare per data di creazione, senza una colonna in più.
        // Si chiede una riga in più del limite per sapere se esiste una pagina
        // successiva senza un secondo `COUNT`.
        let fetch = i64::from(query.limit) + 1;
        let rows = sqlx::query(concat!(
            select_document!(),
            " WHERE owner_type = $1::app.attachment_owner_type
                AND owner_id = $2
                AND deleted_at IS NULL
                AND is_current
                AND ($3::uuid IS NULL OR id > $3)
              ORDER BY id
              LIMIT $4"
        ))
        .bind(db_enums::attachment_owner_type_to_db(
            query.owner.owner_type(),
        ))
        .bind(query.owner.owner_id())
        .bind(query.after.map(DocumentId::as_uuid))
        .bind(fetch)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        let has_more = rows.len() as i64 > i64::from(query.limit);
        let items = rows
            .into_iter()
            .take(query.limit as usize)
            .map(map_document)
            .collect::<RepositoryResult<Vec<_>>>()?;
        let next_cursor = has_more.then(|| items.last().map(Document::id)).flatten();

        Ok(DocumentPage { items, next_cursor })
    }

    async fn insert(&mut self, document: &Document) -> RepositoryResult<()> {
        // `status`, `version`, `is_current` ed `extracted_data` restano ai default
        // dello schema: non nominarli è il modo di non dichiarare un valore che
        // questa fetta non sa gestire.
        sqlx::query(
            "INSERT INTO app.documents (
                 id, org_id, kind, owner_type, owner_id,
                 storage_provider, storage_bucket, object_key,
                 mime_type, size_bytes, checksum_sha256, original_filename,
                 is_encrypted, encryption_key_ref, uploaded_by, created_at, updated_at
             ) VALUES (
                 $1, $2, $3::app.document_kind, $4::app.attachment_owner_type, $5,
                 $6, $7, $8,
                 $9, $10, $11, $12,
                 $13, $14, $15, $16, $17
             )",
        )
        .bind(document.id().as_uuid())
        .bind(document.org_id().as_uuid())
        .bind(db_enums::document_kind_to_db(document.kind()))
        .bind(db_enums::attachment_owner_type_to_db(
            document.owner().owner_type(),
        ))
        .bind(document.owner().owner_id())
        .bind(document.storage().provider())
        .bind(document.storage().bucket())
        .bind(document.storage().object_key())
        .bind(document.mime_type().as_str())
        .bind(document.size_bytes().as_i64())
        .bind(document.checksum().as_bytes().as_slice())
        .bind(document.original_filename().as_str())
        .bind(document.is_encrypted())
        .bind(document.encryption_key_ref())
        .bind(document.uploaded_by().map(UserId::as_uuid))
        .bind(document.created_at())
        .bind(document.updated_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn archive(&mut self, document: &Document) -> RepositoryResult<()> {
        let result = sqlx::query(
            "UPDATE app.documents
                SET deleted_at = $2, updated_at = $2
              WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(document.id().as_uuid())
        .bind(document.archived_at().unwrap_or_else(Utc::now))
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        // Zero righe con RLS attiva significa "non aggiornabile da questo
        // contesto": la `USING` di `documents_update` chiede staff oppure
        // `uploaded_by = current_user_id()`. Il service layer verifica lo stesso
        // criterio prima di arrivare qui, quindi se scatta è un difetto nostro.
        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }
}

fn map_document(row: PgRow) -> RepositoryResult<Document> {
    let invalid = |field: &str, detail: String| RepositoryError::Backend {
        details: format!("{field} non valido in database: {detail}"),
    };

    let id: Uuid = try_get(&row, "id")?;
    let org_id: Uuid = try_get(&row, "org_id")?;
    let kind: String = try_get(&row, "kind")?;
    let owner_type: String = try_get(&row, "owner_type")?;
    let owner_id: Uuid = try_get(&row, "owner_id")?;
    let uploaded_by: Option<Uuid> = try_get(&row, "uploaded_by")?;
    let created_at: DateTime<Utc> = try_get(&row, "created_at")?;
    let updated_at: DateTime<Utc> = try_get(&row, "updated_at")?;
    let archived_at: Option<DateTime<Utc>> = try_get(&row, "deleted_at")?;

    let raw_checksum: Vec<u8> = try_get(&row, "checksum_sha256")?;
    let checksum: [u8; 32] = raw_checksum.try_into().map_err(|bytes: Vec<u8>| {
        // Lo schema ha `CHECK (octet_length(checksum_sha256) = 32)`, quindi qui si
        // arriva solo se il vincolo è stato tolto o aggirato: meglio un errore
        // rumoroso che un digest troncato che sembra valido.
        invalid(
            "checksum_sha256",
            format!("{} byte invece di 32", bytes.len()),
        )
    })?;

    Ok(Document::from_persistence(DocumentParts {
        id: DocumentId::from_uuid(id),
        org_id: OrgId::from_uuid(org_id),
        kind: db_enums::document_kind_from_db(&kind)?,
        owner: AttachmentOwner::from_persistence(
            db_enums::attachment_owner_type_from_db(&owner_type)?,
            owner_id,
        ),
        storage: StorageLocation::from_persistence(
            try_get(&row, "storage_provider")?,
            try_get(&row, "storage_bucket")?,
            try_get(&row, "object_key")?,
        ),
        mime_type: MimeType::from_persistence(try_get(&row, "mime_type")?),
        size_bytes: ObjectSize::from_persistence(try_get(&row, "size_bytes")?),
        checksum: Checksum::from_bytes(checksum),
        original_filename: FileName::from_persistence(try_get(&row, "original_filename")?),
        is_encrypted: try_get(&row, "is_encrypted")?,
        encryption_key_ref: try_get(&row, "encryption_key_ref")?,
        uploaded_by: uploaded_by.map(UserId::from_uuid),
        created_at,
        updated_at,
        archived_at,
    }))
}
