//! Porte tecniche richieste dagli use case: cifratura delle password, emissione
//! dei token, storage oggetti. Tutte astratte dietro trait, così sostituire
//! l'implementazione non tocca il service layer.

mod security;
mod storage;

pub use security::{
    AccessTokenClaims, CryptoError, PasswordHasher, RefreshTokenFactory, RefreshTokenMaterial,
    TokenIssuer,
};
pub use storage::{ObjectStorage, StorageError, StoredObject};
