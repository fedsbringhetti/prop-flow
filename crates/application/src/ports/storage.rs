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
    pub checksum_sha256: [u8; 32],
}

/// Storage oggetti per i documenti.
///
/// Dichiarata ora e implementata allo step dei documenti: la porta esiste perché
/// il vincolo "nessun vendor lock-in" si rispetta decidendo la forma
/// dell'astrazione prima di scrivere il primo adapter, non dopo.
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
