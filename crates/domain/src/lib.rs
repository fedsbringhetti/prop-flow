// `unwrap`/`expect` restano vietati nel codice di produzione. Nei test sono
// invece lo strumento corretto: un panic segnala una precondizione di test non
// soddisfatta, e mascherarla con un `if let` renderebbe il test verde a vuoto.
#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used, clippy::panic))]
//! Dominio PropFlow: entità, value object, eventi e **porte**.
//!
//! Questo crate non dipende dall'infrastruttura. Non c'è `sqlx`, non c'è `axum`,
//! non c'è `tokio`: le dipendenze ammesse sono solo tipi di dato (`uuid`, `chrono`,
//! `rust_decimal`) più `serde` per i derive sugli eventi, che sono contratti verso
//! l'esterno. Il divieto è verificato in CI da `ops/ci/check-domain-purity.sh`,
//! non lasciato alla code review.

pub mod context;
pub mod error;
pub mod events;
pub mod identity;
pub mod lease;
pub mod ports;
pub mod property;
pub mod shared;

pub use context::RequestContext;
pub use error::{Denial, DomainError, DomainResult, Entity, Invariant};
