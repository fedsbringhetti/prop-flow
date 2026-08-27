//! Storage oggetti.
//!
//! Un solo adattatore, S3, che parla anche con MinIO: sono lo stesso protocollo, e
//! cambiare fornitore è cambiare endpoint e stile del percorso. La porta è
//! `propflow_application::ports::ObjectStorage`.

mod s3;

pub use s3::{S3ObjectStorage, S3Settings, S3SetupError};
