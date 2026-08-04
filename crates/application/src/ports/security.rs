use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};

use propflow_domain::identity::{OrgRole, PasswordHash, PlatformRole, TokenHash};
use propflow_domain::shared::{OrgId, UserId};

#[derive(Debug, Clone, thiserror::Error)]
pub enum CryptoError {
    #[error("token non valido")]
    InvalidToken,
    #[error("token scaduto")]
    ExpiredToken,
    #[error("operazione crittografica fallita: {details}")]
    Backend { details: String },
}

/// Cifratura e verifica delle password.
///
/// I metodi sono `async` non per capriccio: Argon2 è deliberatamente lento
/// (decine di millisecondi) e occupa CPU. Eseguirlo direttamente su un worker
/// async bloccherebbe il reattore sotto carico, quindi l'implementazione lo
/// sposta su un thread pool e la porta deve permetterglielo.
#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, plaintext: &str) -> Result<PasswordHash, CryptoError>;

    /// Verifica in tempo costante rispetto all'esito.
    async fn verify(&self, plaintext: &str, hash: &PasswordHash) -> Result<bool, CryptoError>;

    /// Hash di riferimento su cui spendere lo stesso tempo quando l'utente non
    /// esiste: senza questo, la differenza di latenza fra "email sconosciuta" e
    /// "password errata" è un oracolo per enumerare gli account.
    fn dummy_hash(&self) -> &PasswordHash;
}

/// Contenuto di un access token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessTokenClaims {
    pub user_id: UserId,
    pub org_id: Option<OrgId>,
    pub org_role: Option<OrgRole>,
    pub platform_role: PlatformRole,
    pub expires_at: DateTime<Utc>,
}

/// Emissione e verifica degli access token.
pub trait TokenIssuer: Send + Sync {
    fn issue(&self, claims: &AccessTokenClaims) -> Result<String, CryptoError>;
    fn verify(&self, token: &str) -> Result<AccessTokenClaims, CryptoError>;
    /// Durata di validità di un access token appena emesso.
    fn access_token_lifetime(&self) -> Duration;
}

/// Un refresh token appena generato: il valore in chiaro da consegnare al client
/// una sola volta, e l'hash da persistere.
#[derive(Debug, Clone)]
pub struct RefreshTokenMaterial {
    pub plaintext: String,
    pub hash: TokenHash,
}

/// Generazione di refresh token opachi.
///
/// Opachi e non JWT: un refresh token deve poter essere revocato, e un JWT
/// autoconsistente è valido finché non scade, qualunque cosa accada nel frattempo.
pub trait RefreshTokenFactory: Send + Sync {
    fn generate(&self) -> Result<RefreshTokenMaterial, CryptoError>;
    /// Calcola l'hash di un token presentato dal client, per cercarlo in database.
    fn hash(&self, plaintext: &str) -> TokenHash;
    fn lifetime(&self) -> Duration;
}
