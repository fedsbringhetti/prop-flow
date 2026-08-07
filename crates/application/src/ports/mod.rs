//! Porte tecniche richieste dagli use case: cifratura delle password, emissione
//! dei token, storage oggetti, notifiche. Tutte astratte dietro trait, così
//! sostituire l'implementazione non tocca il service layer.

mod notify;
mod security;
mod storage;

pub use notify::{Notification, NotificationContent, Notifier, NotifyError};
pub use security::{
    AccessTokenClaims, CryptoError, OpaqueToken, OpaqueTokenFactory, PasswordHasher,
    RefreshTokenFactory, RefreshTokenMaterial, TokenIssuer,
};
pub use storage::{ObjectStorage, StorageError, StoredObject};
