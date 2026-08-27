// `unwrap`/`expect` restano vietati nel codice di produzione. Nei test sono
// invece lo strumento corretto: un panic segnala una precondizione di test non
// soddisfatta, e mascherarla con un `if let` renderebbe il test verde a vuoto.
#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used, clippy::panic))]
//! Infrastruttura: implementazioni concrete delle porte del dominio e
//! dell'application layer.
//!
//! Qui vive tutto ciò che è sostituibile — PostgreSQL, Argon2, JWT, OTLP — e
//! nient'altro conosce questi nomi. Il dominio dichiara le porte, questo crate
//! le riempie.

pub mod clock;
pub mod db;
pub mod security;
pub mod storage;
pub mod telemetry;

pub use clock::SystemClock;
pub use db::{DatabasePools, PgUnitOfWorkFactory};
pub use storage::{S3ObjectStorage, S3Settings};
