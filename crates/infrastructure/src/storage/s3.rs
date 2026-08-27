use std::time::Duration;

use async_trait::async_trait;
use propflow_application::ports::{ObjectStorage, StorageError, StoredObject};
use reqwest::StatusCode;
use rusty_s3::actions::{GetObject, PutObject, S3Action as _};
use rusty_s3::{Bucket, Credentials, UrlStyle};
use sha2::{Digest as _, Sha256};
use url::Url;

/// Header con cui S3 dichiara di aver cifrato l'oggetto a riposo (`AES256`,
/// `aws:kms`). MinIO lo restituisce con la stessa semantica.
const SSE_HEADER: &str = "x-amz-server-side-encryption";

/// Vita della firma delle richieste che l'API esegue da sé.
///
/// Corta e non configurabile: la URL non lascia mai questo processo — è firmata,
/// usata subito e buttata. Solo l'URL di download è configurabile, perché quello
/// finisce nelle mani del client.
const INTERNAL_SIGNATURE_TTL: Duration = Duration::from_secs(60);

/// Configurazione dell'adattatore.
#[derive(Debug, Clone)]
pub struct S3Settings {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    /// `true` per MinIO e per S3 raggiunto da un endpoint personalizzato: lo stile
    /// virtual-host richiede un DNS con il nome del bucket come sottodominio, che
    /// in locale non esiste.
    pub path_style: bool,
    pub request_timeout: Duration,
}

/// Errori di costruzione, distinti da quelli di esercizio: qui si fallisce
/// all'avvio, e un avvio che fallisce è meglio di un servizio che accetta traffico
/// e restituisce 500.
#[derive(Debug, thiserror::Error)]
pub enum S3SetupError {
    #[error("endpoint dello storage non valido: {0}")]
    Endpoint(String),
    #[error("bucket non valido: {0}")]
    Bucket(String),
    #[error("client HTTP non costruibile: {0}")]
    Client(String),
}

/// Adattatore S3, valido anche per MinIO.
///
/// `rusty-s3` firma le richieste e non le esegue (Sans-IO): il trasporto è
/// `reqwest`, già nell'albero come dipendenza di `opentelemetry-otlp`. Scartato
/// `aws-sdk-s3`: corretto per costruzione ma decine di crate in più, contro un
/// bisogno che è la firma di tre richieste.
pub struct S3ObjectStorage {
    bucket: Bucket,
    credentials: Credentials,
    client: reqwest::Client,
}

impl S3ObjectStorage {
    /// Costruisce l'adattatore.
    ///
    /// Installa anche il provider crittografico di rustls, se nessuno lo ha già
    /// fatto: `reqwest` con la feature `rustls-no-provider` va in **panico** alla
    /// costruzione del client se il default non c'è. La feature `rustls` lo
    /// eviterebbe, ma tira `aws-lc-rs`, cioè il toolchain C e cmake che il
    /// progetto non vuole nell'immagine di build — la stessa ragione per cui
    /// `jsonwebtoken` usa `rust_crypto`. `ring` è già nell'albero, portato da
    /// `sqlx` con `tls-rustls-ring`.
    pub fn new(settings: &S3Settings) -> Result<Self, S3SetupError> {
        // `install_default` fallisce se un provider c'è già: è esattamente il caso
        // in cui non c'è nulla da fare, non un errore.
        let _ = rustls::crypto::ring::default_provider().install_default();

        let endpoint: Url = settings
            .endpoint
            .parse()
            .map_err(|error: url::ParseError| S3SetupError::Endpoint(error.to_string()))?;
        let style = if settings.path_style {
            UrlStyle::Path
        } else {
            UrlStyle::VirtualHost
        };
        let bucket = Bucket::new(
            endpoint,
            style,
            settings.bucket.clone(),
            settings.region.clone(),
        )
        .map_err(|error| S3SetupError::Bucket(error.to_string()))?;

        let client = reqwest::Client::builder()
            .timeout(settings.request_timeout)
            .build()
            .map_err(|error| S3SetupError::Client(error.to_string()))?;

        Ok(Self {
            bucket,
            credentials: Credentials::new(
                settings.access_key_id.clone(),
                settings.secret_access_key.clone(),
            ),
            client,
        })
    }

    fn backend(details: impl std::fmt::Display) -> StorageError {
        StorageError::Backend {
            details: details.to_string(),
        }
    }
}

#[async_trait]
impl ObjectStorage for S3ObjectStorage {
    async fn put(
        &self,
        key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<StoredObject, StorageError> {
        // Il digest si calcola **qui**, sui byte che stiamo per scrivere: un
        // checksum dichiarato dal client certificherebbe solo che il client sa
        // calcolare uno SHA-256.
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let checksum: [u8; 32] = hasher.finalize().into();
        let size_bytes = bytes.len() as u64;

        let mut action = PutObject::new(&self.bucket, Some(&self.credentials), key);
        // `content-type` entra fra gli header **firmati**: se restasse fuori dalla
        // firma sarebbe un header che il fornitore può ignorare, e il documento
        // tornerebbe come `application/octet-stream` al download.
        action
            .headers_mut()
            .insert("content-type", content_type.to_owned());
        let url = action.sign(INTERNAL_SIGNATURE_TTL);

        let response = self
            .client
            .put(url)
            // Lo stesso valore dell'header firmato, altrimenti la firma non torna.
            .header("content-type", content_type)
            .body(bytes)
            .send()
            .await
            .map_err(Self::backend)?;

        let status = response.status();
        // La modalità di cifratura si **legge dalla risposta** invece di essere
        // dichiarata: è ciò che rende vera la colonna `documents.is_encrypted`
        // anche quando il bucket non è configurato come si crede.
        let encryption = response
            .headers()
            .get(SSE_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);

        if !status.is_success() {
            // Il corpo di errore di S3 è XML e può contenere la chiave
            // dell'oggetto: resta fuori dal messaggio, che finisce nei log.
            return Err(Self::backend(format!("PUT rifiutato con stato {status}")));
        }

        Ok(StoredObject {
            provider: "s3".to_owned(),
            bucket: self.bucket.name().to_owned(),
            key: key.to_owned(),
            size_bytes,
            checksum_sha256: checksum,
            encryption,
        })
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let action = GetObject::new(&self.bucket, Some(&self.credentials), key);
        let url = action.sign(INTERNAL_SIGNATURE_TTL);

        let response = self.client.get(url).send().await.map_err(Self::backend)?;

        if response.status() == StatusCode::NOT_FOUND {
            return Err(StorageError::NotFound);
        }
        if !response.status().is_success() {
            return Err(Self::backend(format!(
                "GET rifiutato con stato {}",
                response.status()
            )));
        }

        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(Self::backend)
    }

    async fn presigned_get_url(&self, key: &str, ttl_seconds: u32) -> Result<String, StorageError> {
        let action = GetObject::new(&self.bucket, Some(&self.credentials), key);
        Ok(action
            .sign(Duration::from_secs(u64::from(ttl_seconds)))
            .to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> S3Settings {
        S3Settings {
            endpoint: "http://127.0.0.1:9000".to_owned(),
            region: "us-east-1".to_owned(),
            bucket: "propflow".to_owned(),
            access_key_id: "chiave".to_owned(),
            secret_access_key: "segreto".to_owned(),
            path_style: true,
            request_timeout: Duration::from_secs(5),
        }
    }

    /// Con MinIO e con un endpoint personalizzato lo stile deve essere `path`: il
    /// virtual-host richiede un DNS con il bucket come sottodominio, che in locale
    /// non esiste e in produzione va predisposto.
    #[test]
    fn path_style_puts_the_bucket_in_the_path() {
        let storage = S3ObjectStorage::new(&settings()).expect("configurazione valida");
        let url = storage
            .bucket
            .object_url("019f/019a")
            .expect("chiave valida");
        assert_eq!(url.as_str(), "http://127.0.0.1:9000/propflow/019f/019a");
    }

    /// L'URL pre-firmato deve portare la firma e una scadenza: senza `X-Amz-Expires`
    /// sarebbe un permesso perpetuo.
    #[tokio::test]
    async fn the_presigned_url_carries_a_signature_and_an_expiry() {
        let storage = S3ObjectStorage::new(&settings()).expect("configurazione valida");
        let url = storage
            .presigned_get_url("019f/019a", 300)
            .await
            .expect("firma prodotta");
        assert!(url.contains("X-Amz-Signature="), "{url}");
        assert!(url.contains("X-Amz-Expires=300"), "{url}");
        assert!(
            url.starts_with("http://127.0.0.1:9000/propflow/019f/019a?"),
            "{url}"
        );
    }

    #[test]
    fn an_invalid_endpoint_fails_at_construction() {
        let mut broken = settings();
        broken.endpoint = "non-una-url".to_owned();
        assert!(matches!(
            S3ObjectStorage::new(&broken),
            Err(S3SetupError::Endpoint(_))
        ));

        let mut ftp = settings();
        ftp.endpoint = "ftp://esempio.test".to_owned();
        assert!(matches!(
            S3ObjectStorage::new(&ftp),
            Err(S3SetupError::Bucket(_))
        ));
    }
}
