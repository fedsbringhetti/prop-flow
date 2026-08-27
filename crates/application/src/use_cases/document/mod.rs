//! Documenti: caricamento, elenco, download e archiviazione.
//!
//! Il perimetro di questa fetta è dichiarato in `docs/09-stato-e-roadmap.md`.
//! Restano fuori il versionamento (V2) e l'OCR (modulo AI), le cui colonne
//! esistono già nello schema e restano ai loro default: vedere una colonna non è
//! una ragione per riempirla.

mod archive;
mod download;
mod list;
mod owner_access;
mod upload;

pub use archive::{ArchiveDocument, ArchiveDocumentCommand};
pub use download::{DocumentDownload, DownloadDocument};
pub use list::{ListDocuments, ListDocumentsCommand};
pub use upload::{UploadDocument, UploadDocumentCommand};
