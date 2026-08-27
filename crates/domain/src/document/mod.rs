//! Documenti allegati agli aggregati del dominio.
//!
//! Un documento è **metadati più un riferimento a un oggetto**: il contenuto non
//! passa mai da qui, sta dietro la porta `ObjectStorage`. Il dominio conosce la
//! chiave dell'oggetto perché è ciò che rende il metadato utile, non il byte.

mod aggregate;
mod owner;
mod value_objects;

pub use aggregate::{Document, DocumentKind, DocumentParts, NewDocument};
pub use owner::{AttachmentOwner, AttachmentOwnerType};
pub use value_objects::{Checksum, FileName, MimeType, ObjectSize, StorageLocation};
