use argon2::password_hash::{
    PasswordHash as EncodedHash, PasswordHasher as _, PasswordVerifier, SaltString,
};
use argon2::{Algorithm, Argon2, Params, Version};
use async_trait::async_trait;

use propflow_application::ports::{CryptoError, PasswordHasher};
use propflow_domain::identity::PasswordHash;

/// Cifratura delle password con Argon2id.
///
/// I parametri seguono il profilo consigliato da OWASP: 19 MiB di memoria, tre
/// iterazioni, un grado di parallelismo. La memoria è la difesa che conta contro
/// il cracking su GPU, dove la potenza di calcolo è abbondante ma la RAM per
/// thread no.
pub struct Argon2PasswordHasher {
    hasher: Argon2<'static>,
    /// Hash su cui spendere tempo quando l'utente non esiste, così la latenza non
    /// distingue "email sconosciuta" da "password errata".
    dummy: PasswordHash,
}

impl Argon2PasswordHasher {
    const MEMORY_KIB: u32 = 19 * 1024;
    const ITERATIONS: u32 = 3;
    const PARALLELISM: u32 = 1;

    pub fn new() -> Result<Self, CryptoError> {
        let params = Params::new(Self::MEMORY_KIB, Self::ITERATIONS, Self::PARALLELISM, None)
            .map_err(|error| CryptoError::Backend {
                details: error.to_string(),
            })?;
        let hasher = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        // L'hash di riferimento si calcola una volta all'avvio su una password
        // casuale che nessuno conosce.
        let mut secret = [0u8; 32];
        getrandom::fill(&mut secret).map_err(|error| CryptoError::Backend {
            details: error.to_string(),
        })?;
        let dummy_plaintext = hex(&secret);
        let dummy = hash_with(&hasher, &dummy_plaintext)?;

        Ok(Self { hasher, dummy })
    }
}

#[async_trait]
impl PasswordHasher for Argon2PasswordHasher {
    async fn hash(&self, plaintext: &str) -> Result<PasswordHash, CryptoError> {
        // Argon2 è volutamente costoso: eseguirlo sul reattore async bloccherebbe
        // altre richieste per decine di millisecondi ciascuna.
        let hasher = self.hasher.clone();
        let plaintext = plaintext.to_owned();
        tokio_block(move || hash_with(&hasher, &plaintext)).await
    }

    async fn verify(&self, plaintext: &str, hash: &PasswordHash) -> Result<bool, CryptoError> {
        let hasher = self.hasher.clone();
        let plaintext = plaintext.to_owned();
        let encoded = hash.as_str().to_owned();
        tokio_block(move || {
            let parsed = EncodedHash::new(&encoded).map_err(|error| CryptoError::Backend {
                details: error.to_string(),
            })?;
            // `verify_password` distingue "password errata" da "hash illeggibile":
            // la prima è un esito normale, il secondo un dato corrotto.
            match hasher.verify_password(plaintext.as_bytes(), &parsed) {
                Ok(()) => Ok(true),
                Err(argon2::password_hash::Error::Password) => Ok(false),
                Err(error) => Err(CryptoError::Backend {
                    details: error.to_string(),
                }),
            }
        })
        .await
    }

    fn dummy_hash(&self) -> &PasswordHash {
        &self.dummy
    }
}

fn hash_with(hasher: &Argon2<'static>, plaintext: &str) -> Result<PasswordHash, CryptoError> {
    let mut salt_bytes = [0u8; 16];
    getrandom::fill(&mut salt_bytes).map_err(|error| CryptoError::Backend {
        details: error.to_string(),
    })?;
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|error| CryptoError::Backend {
        details: error.to_string(),
    })?;
    let encoded = hasher
        .hash_password(plaintext.as_bytes(), &salt)
        .map_err(|error| CryptoError::Backend {
            details: error.to_string(),
        })?
        .to_string();
    PasswordHash::new(encoded).map_err(|error| CryptoError::Backend {
        details: error.to_string(),
    })
}

/// Esegue un lavoro CPU-bound fuori dal reattore async.
async fn tokio_block<T, F>(work: F) -> Result<T, CryptoError>
where
    F: FnOnce() -> Result<T, CryptoError> + Send + 'static,
    T: Send + 'static,
{
    match tokio::task::spawn_blocking(work).await {
        Ok(result) => result,
        Err(error) => Err(CryptoError::Backend {
            details: format!("thread di hashing terminato in modo anomalo: {error}"),
        }),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
