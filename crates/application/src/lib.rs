// `unwrap`/`expect` restano vietati nel codice di produzione. Nei test sono
// invece lo strumento corretto: un panic segnala una precondizione di test non
// soddisfatta, e mascherarla con un `if let` renderebbe il test verde a vuoto.
#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used, clippy::panic))]
//! Service layer: gli use case.
//!
//! Ogni use case è una struct con le porte iniettate e un unico `execute`, che
//! riceve il [`RequestContext`](propflow_domain::RequestContext) e un comando.
//! Qui vivono orchestrazione, autorizzazione applicativa e transazioni; le regole
//! di dominio stanno negli aggregati e i dettagli tecnici nell'infrastruttura.

pub mod authorization;
pub mod error;
pub mod password_policy;
pub mod ports;
pub mod use_cases;

pub use error::{AppError, AppResult, Conflict, UnsupportedQuery};
