use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::Duration;
use sha2::{Digest, Sha256};

use propflow_application::ports::{CryptoError, RefreshTokenFactory, RefreshTokenMaterial};
use propflow_domain::identity::TokenHash;

/// Refresh token opachi da 256 bit di entropia.
///
/// Opachi e non JWT: un refresh token deve poter essere revocato prima della
/// scadenza, e un JWT autoconsistente resta valido anche dopo che l'utente ha
/// fatto logout o che la sessione è stata compromessa. Qui la fonte di verità è la
/// riga in `app.user_sessions`, che si può revocare.
pub struct OpaqueRefreshTokenFactory {
    lifetime: Duration,
}

impl OpaqueRefreshTokenFactory {
    /// 256 bit: fuori portata per un attacco a forza bruta, e non tanto lunghi da
    /// dare problemi in un header o in un cookie.
    const TOKEN_BYTES: usize = 32;

    #[must_use]
    pub const fn new(lifetime: Duration) -> Self {
        Self { lifetime }
    }
}

impl RefreshTokenFactory for OpaqueRefreshTokenFactory {
    fn generate(&self) -> Result<RefreshTokenMaterial, CryptoError> {
        let mut raw = [0u8; Self::TOKEN_BYTES];
        getrandom::fill(&mut raw).map_err(|error| CryptoError::Backend {
            details: error.to_string(),
        })?;
        let plaintext = URL_SAFE_NO_PAD.encode(raw);
        let hash = hash_token(&plaintext);
        Ok(RefreshTokenMaterial { plaintext, hash })
    }

    fn hash(&self, plaintext: &str) -> TokenHash {
        hash_token(plaintext)
    }

    fn lifetime(&self) -> Duration {
        self.lifetime
    }
}

/// SHA-256 sulla rappresentazione testuale, cioè esattamente ciò che il client
/// rispedisce. Un digest semplice basta: il token ha 256 bit di entropia, quindi
/// non esiste il dizionario che rende utile un KDF lento come per le password.
fn hash_token(plaintext: &str) -> TokenHash {
    let digest = Sha256::digest(plaintext.as_bytes());
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&digest);
    TokenHash::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_tokens_are_unique_and_hash_stably() {
        let factory = OpaqueRefreshTokenFactory::new(Duration::days(30));
        let first = factory.generate().expect("generazione riuscita");
        let second = factory.generate().expect("generazione riuscita");
        assert_ne!(first.plaintext, second.plaintext);
        assert_eq!(factory.hash(&first.plaintext), first.hash);
        assert_ne!(first.hash, second.hash);
    }

    #[test]
    fn token_is_url_safe() {
        let factory = OpaqueRefreshTokenFactory::new(Duration::days(30));
        let token = factory.generate().expect("generazione riuscita").plaintext;
        assert!(
            token
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "token non url-safe: {token}"
        );
    }
}
