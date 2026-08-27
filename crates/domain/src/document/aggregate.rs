use chrono::{DateTime, Utc};

use super::owner::AttachmentOwner;
use super::value_objects::{Checksum, FileName, MimeType, ObjectSize, StorageLocation};
use crate::error::{DomainError, DomainResult, Invariant};
use crate::shared::{DocumentId, OrgId, UserId};

/// Classificazione funzionale del documento.
///
/// Rispecchia `app.document_kind` per intero: a differenza dell'owner qui non c'è
/// una ragione per esporne un sottoinsieme, perché il tipo non porta con sé
/// dipendenze verso altri aggregati.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentKind {
    LeaseContract,
    IdDocument,
    UtilityBill,
    Invoice,
    Receipt,
    Photo,
    InspectionReport,
    EnergyCertificate,
    CadastralExtract,
    InsurancePolicy,
    Other,
}

impl DocumentKind {
    /// Nome stabile, usato negli eventi di dominio. Non è il valore del database:
    /// quello lo produce `db_enums`, e tenerli separati evita che una rinomina
    /// dell'uno cambi in silenzio l'altro.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LeaseContract => "lease_contract",
            Self::IdDocument => "id_document",
            Self::UtilityBill => "utility_bill",
            Self::Invoice => "invoice",
            Self::Receipt => "receipt",
            Self::Photo => "photo",
            Self::InspectionReport => "inspection_report",
            Self::EnergyCertificate => "energy_certificate",
            Self::CadastralExtract => "cadastral_extract",
            Self::InsurancePolicy => "insurance_policy",
            Self::Other => "other",
        }
    }
}

/// Documento allegato a un immobile, a una stanza o a un contratto.
///
/// L'aggregato descrive **il metadato**, non il contenuto: i byte stanno dietro
/// `ObjectStorage` e non attraversano mai il dominio. Ciò che il dominio custodisce
/// è il legame fra i due, cioè [`StorageLocation`], e le invarianti che rendono
/// quel legame ricostruibile: la chiave si deriva dagli identificatori, quindi da
/// una riga si ritrova sempre l'oggetto, e da un oggetto la riga.
///
/// **Fuori perimetro in questa fetta** (`docs/09-stato-e-roadmap.md`): il
/// versionamento (`version`, `supersedes_document_id`, `is_current`) e l'OCR
/// (`status`, `extracted_data`). Le colonne esistono nello schema e restano ai
/// loro default; vederle non è una ragione per riempirle.
#[derive(Debug, Clone)]
pub struct Document {
    id: DocumentId,
    org_id: OrgId,
    kind: DocumentKind,
    owner: AttachmentOwner,
    storage: StorageLocation,
    mime_type: MimeType,
    size_bytes: ObjectSize,
    checksum: Checksum,
    original_filename: FileName,
    /// Cifratura a riposo lato bucket. È `true` solo se il fornitore **ha
    /// confermato** di aver cifrato: la colonna dice il vero anche quando il
    /// bucket non è configurato per farlo.
    is_encrypted: bool,
    /// Modalità di cifratura registrata dal fornitore (`AES256`, `aws:kms`), **non
    /// una chiave**. Una chiave in questa colonna sarebbe una chiave in chiaro
    /// accanto al dato che protegge.
    encryption_key_ref: Option<String>,
    uploaded_by: Option<UserId>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    archived_at: Option<DateTime<Utc>>,
}

impl Document {
    /// Chiave dell'oggetto nello storage.
    ///
    /// `{org_id}/{document_id}` e **mai** il nome originale del file, per due
    /// ragioni indipendenti che portano alla stessa scelta:
    ///
    /// 1. il nome del file è un dato personale, e la chiave compare nei log del
    ///    fornitore di storage, che non è un posto dove decidiamo noi cosa si
    ///    conserva;
    /// 2. l'indice unico `documents_object_uk` è su
    ///    `(storage_provider, storage_bucket, object_key)`: due file omonimi
    ///    caricati sullo stesso bucket si scontrerebbero, e il secondo upload
    ///    fallirebbe per un motivo che al chiamante non si può spiegare.
    ///
    /// Il prefisso è l'organizzazione perché rende leggibile un `ls` del bucket e
    /// permette una policy per prefisso, non perché serva a isolare: l'isolamento
    /// è della Row Level Security.
    #[must_use]
    pub fn object_key(org_id: OrgId, id: DocumentId) -> String {
        format!("{org_id}/{id}")
    }

    /// Costruisce il metadato di un documento appena caricato.
    ///
    /// L'oggetto è già stato scritto quando si arriva qui: `storage`, `size_bytes`
    /// e `checksum` sono ciò che il fornitore ha confermato, non ciò che il client
    /// ha dichiarato.
    #[allow(clippy::too_many_arguments)]
    pub fn upload(parts: NewDocument) -> DomainResult<Self> {
        let NewDocument {
            id,
            org_id,
            kind,
            owner,
            storage,
            mime_type,
            size_bytes,
            checksum,
            original_filename,
            encryption_key_ref,
            uploaded_by,
            now,
        } = parts;

        Ok(Self {
            id,
            org_id,
            kind,
            owner,
            storage,
            mime_type,
            size_bytes,
            checksum,
            original_filename,
            is_encrypted: encryption_key_ref.is_some(),
            encryption_key_ref: crate::shared::text::optional(
                "encryption_key_ref",
                encryption_key_ref,
                255,
            )?,
            uploaded_by,
            created_at: now,
            updated_at: now,
            archived_at: None,
        })
    }

    /// Archivia il documento: soft delete.
    ///
    /// **L'oggetto nel bucket sopravvive alla riga.** In questa fetta nessuno lo
    /// rimuove, ed è un debito registrato: il ruolo applicativo non ha il
    /// privilegio `DELETE`, e la cancellazione dallo storage è un'operazione
    /// separata che va pensata insieme al purge di `retention_until`.
    ///
    /// Non esiste un `restore`: la policy `documents_update` ha
    /// `deleted_at IS NULL` nella `USING`, quindi un update su una riga archiviata
    /// non fallisce — aggiorna zero righe. Ripristinare richiede una migration
    /// sulla policy, non una query diversa.
    pub fn archive(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if self.archived_at.is_some() {
            return Err(DomainError::Invariant(Invariant::DocumentArchived));
        }
        self.archived_at = Some(now);
        self.updated_at = now;
        Ok(())
    }

    #[must_use]
    pub const fn id(&self) -> DocumentId {
        self.id
    }
    #[must_use]
    pub const fn org_id(&self) -> OrgId {
        self.org_id
    }
    #[must_use]
    pub const fn kind(&self) -> DocumentKind {
        self.kind
    }
    #[must_use]
    pub const fn owner(&self) -> AttachmentOwner {
        self.owner
    }
    #[must_use]
    pub const fn storage(&self) -> &StorageLocation {
        &self.storage
    }
    #[must_use]
    pub const fn mime_type(&self) -> &MimeType {
        &self.mime_type
    }
    #[must_use]
    pub const fn size_bytes(&self) -> ObjectSize {
        self.size_bytes
    }
    #[must_use]
    pub const fn checksum(&self) -> Checksum {
        self.checksum
    }
    #[must_use]
    pub const fn original_filename(&self) -> &FileName {
        &self.original_filename
    }
    #[must_use]
    pub const fn is_encrypted(&self) -> bool {
        self.is_encrypted
    }
    #[must_use]
    pub fn encryption_key_ref(&self) -> Option<&str> {
        self.encryption_key_ref.as_deref()
    }
    #[must_use]
    pub const fn uploaded_by(&self) -> Option<UserId> {
        self.uploaded_by
    }
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
    #[must_use]
    pub const fn archived_at(&self) -> Option<DateTime<Utc>> {
        self.archived_at
    }

    /// Ricostruzione da persistenza: non riesegue le invarianti, perché un
    /// documento già salvato deve poter essere riletto anche se le regole sono
    /// cambiate da allora.
    #[must_use]
    pub fn from_persistence(parts: DocumentParts) -> Self {
        let DocumentParts {
            id,
            org_id,
            kind,
            owner,
            storage,
            mime_type,
            size_bytes,
            checksum,
            original_filename,
            is_encrypted,
            encryption_key_ref,
            uploaded_by,
            created_at,
            updated_at,
            archived_at,
        } = parts;
        Self {
            id,
            org_id,
            kind,
            owner,
            storage,
            mime_type,
            size_bytes,
            checksum,
            original_filename,
            is_encrypted,
            encryption_key_ref,
            uploaded_by,
            created_at,
            updated_at,
            archived_at,
        }
    }
}

/// Dati di un documento appena caricato.
///
/// Una struct e non undici argomenti: la firma sarebbe una fila di tipi in cui
/// due `String` adiacenti si scambiano senza che il compilatore se ne accorga.
#[derive(Debug)]
pub struct NewDocument {
    pub id: DocumentId,
    pub org_id: OrgId,
    pub kind: DocumentKind,
    pub owner: AttachmentOwner,
    pub storage: StorageLocation,
    pub mime_type: MimeType,
    pub size_bytes: ObjectSize,
    pub checksum: Checksum,
    pub original_filename: FileName,
    pub encryption_key_ref: Option<String>,
    pub uploaded_by: Option<UserId>,
    pub now: DateTime<Utc>,
}

/// Dati grezzi di un documento letti dalla persistenza.
#[derive(Debug)]
pub struct DocumentParts {
    pub id: DocumentId,
    pub org_id: OrgId,
    pub kind: DocumentKind,
    pub owner: AttachmentOwner,
    pub storage: StorageLocation,
    pub mime_type: MimeType,
    pub size_bytes: ObjectSize,
    pub checksum: Checksum,
    pub original_filename: FileName,
    pub is_encrypted: bool,
    pub encryption_key_ref: Option<String>,
    pub uploaded_by: Option<UserId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::LeaseId;

    fn new_document(filename: &str, encryption: Option<&str>) -> DomainResult<Document> {
        let id = DocumentId::generate();
        let org_id = OrgId::generate();
        Document::upload(NewDocument {
            id,
            org_id,
            kind: DocumentKind::LeaseContract,
            owner: AttachmentOwner::Lease(LeaseId::generate()),
            storage: StorageLocation::new("s3", "propflow", &Document::object_key(org_id, id))?,
            mime_type: MimeType::parse("application/pdf")?,
            size_bytes: ObjectSize::new(1024)?,
            checksum: Checksum::from_bytes([7; 32]),
            original_filename: FileName::parse(filename)?,
            encryption_key_ref: encryption.map(str::to_owned),
            uploaded_by: Some(UserId::generate()),
            now: Utc::now(),
        })
    }

    /// La trappola più cara del modulo: la chiave dell'oggetto non deve contenere
    /// il nome del file. Se qualcuno la cambiasse, questo test lo dice prima che
    /// un nome di persona finisca nei log del fornitore di storage.
    #[test]
    fn the_object_key_never_contains_the_file_name() {
        let document = new_document("contratto-mario-rossi.pdf", None).expect("valido");
        let key = document.storage().object_key();
        assert!(
            !key.contains("mario"),
            "la chiave contiene il nome del file: {key}"
        );
        assert!(
            !key.contains(".pdf"),
            "la chiave contiene il nome del file: {key}"
        );
        assert_eq!(
            key,
            format!("{}/{}", document.org_id(), document.id()),
            "la chiave deve derivare dagli identificatori"
        );
    }

    #[test]
    fn encryption_is_recorded_only_when_the_provider_confirms_it() {
        let plain = new_document("a.pdf", None).expect("valido");
        assert!(!plain.is_encrypted());
        assert_eq!(plain.encryption_key_ref(), None);

        let encrypted = new_document("a.pdf", Some("AES256")).expect("valido");
        assert!(encrypted.is_encrypted());
        assert_eq!(encrypted.encryption_key_ref(), Some("AES256"));
    }

    #[test]
    fn a_document_starts_alive_and_can_be_archived_once() {
        let mut document = new_document("a.pdf", None).expect("valido");
        assert_eq!(document.archived_at(), None);

        let now = Utc::now();
        document.archive(now).expect("archiviabile");
        assert_eq!(document.archived_at(), Some(now));

        // Non c'è ripristino, e riarchiviare non è un no-op silenzioso: la policy
        // `documents_update` non vedrebbe la riga e l'update toccherebbe zero righe.
        let error = document.archive(Utc::now()).expect_err("deve rifiutare");
        assert!(matches!(
            error,
            DomainError::Invariant(Invariant::DocumentArchived)
        ));
    }
}
