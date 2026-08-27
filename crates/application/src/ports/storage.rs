use async_trait::async_trait;

/// Errori dello storage oggetti.
#[derive(Debug, Clone, thiserror::Error)]
pub enum StorageError {
    #[error("oggetto non trovato")]
    NotFound,
    #[error("storage non disponibile: {details}")]
    Backend { details: String },
}

/// Riferimento a un oggetto salvato, indipendente dal fornitore.
#[derive(Debug, Clone)]
pub struct StoredObject {
    pub provider: String,
    pub bucket: String,
    pub key: String,
    pub size_bytes: u64,
    /// Digest calcolato **da chi ha scritto l'oggetto**, non dichiarato dal
    /// chiamante: è la ragione per cui l'upload passa dall'API invece di usare un
    /// `PUT` pre-firmato.
    pub checksum_sha256: [u8; 32],
    /// Modalità di cifratura a riposo **confermata dal fornitore** (`AES256`,
    /// `aws:kms`), oppure `None` se l'oggetto non è cifrato.
    ///
    /// È una risposta e non una richiesta: la cifratura è configurata sul bucket,
    /// e leggere ciò che il fornitore dichiara di aver fatto è l'unico modo perché
    /// la colonna `documents.is_encrypted` dica il vero anche quando il bucket non
    /// è configurato come si crede. **Limite da non nascondere:** con SSE la
    /// chiave è del fornitore, quindi non protegge da un accesso amministrativo al
    /// bucket.
    pub encryption: Option<String>,
}

/// Storage oggetti per i documenti.
///
/// La porta è stata dichiarata prima di avere un adattatore, come chiede
/// `docs/06-sviluppo.md` §Aggiungere un provider esterno: decidere la forma
/// dell'astrazione dopo il primo adattatore significa modellarla su quel
/// fornitore.
#[async_trait]
pub trait ObjectStorage: Send + Sync {
    async fn put(
        &self,
        key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<StoredObject, StorageError>;
    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError>;
    /// URL pre-firmato per il download diretto dal client, senza far transitare
    /// il contenuto dall'API.
    async fn presigned_get_url(&self, key: &str, ttl_seconds: u32) -> Result<String, StorageError>;
}
