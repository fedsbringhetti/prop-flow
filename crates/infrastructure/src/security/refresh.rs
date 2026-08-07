use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::Duration;
use sha2::{Digest, Sha256};

use propflow_application::ports::{
    CryptoError, OpaqueToken, OpaqueTokenFactory, RefreshTokenFactory, RefreshTokenMaterial,
};
use propflow_domain::identity::TokenHash;

/// 256 bit di entropia: fuori portata per un attacco a forza bruta, e non tanto
/// lunghi da dare problemi in un header, in un cookie o in un URL di invito.
const TOKEN_BYTES: usize = 32;

fn random_token() -> Result<String, CryptoError> {
    let mut raw = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut raw).map_err(|error| CryptoError::Backend {
        details: error.to_string(),
    })?;
    Ok(URL_SAFE_NO_PAD.encode(raw))
}

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
    #[must_use]
    pub const fn new(lifetime: Duration) -> Self {
        Self { lifetime }
    }
}

impl RefreshTokenFactory for OpaqueRefreshTokenFactory {
    fn generate(&self) -> Result<RefreshTokenMaterial, CryptoError> {
        let plaintext = random_token()?;
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

/// Token opachi non legati a una sessione: inviti, e in seguito verifica
/// dell'email e reimpostazione della password.
///
/// Stessa entropia e stesso digest dei refresh token, tipo separato: la scadenza di
/// un invito non ha niente a che vedere con quella di una sessione, e un tipo unico
/// costringerebbe a scegliere quale delle due `lifetime` significa cosa.
pub struct OpaqueTokenGenerator;

impl OpaqueTokenFactory for OpaqueTokenGenerator {
    fn generate(&self) -> Result<OpaqueToken, CryptoError> {
        let plaintext = random_token()?;
        let hash = hash_token(&plaintext);
        Ok(OpaqueToken { plaintext, hash })
    }

    fn hash(&self, plaintext: &str) -> TokenHash {
        hash_token(plaintext)
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
    fn invitation_tokens_share_the_hashing_of_refresh_tokens() {
        // Lo stesso digest, così un token presentato dal client si ritrova in
        // database qualunque delle due porte lo abbia generato.
        let invitations = OpaqueTokenGenerator;
        let token = invitations.generate().expect("generazione riuscita");
        assert_eq!(invitations.hash(&token.plaintext), token.hash);
        assert_eq!(
            OpaqueRefreshTokenFactory::new(Duration::days(30)).hash(&token.plaintext),
            token.hash
        );
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
