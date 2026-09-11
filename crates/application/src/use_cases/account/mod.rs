//! Il proprio account: leggerlo, correggerlo, cambiare la password.
//!
//! **Nessuno di questi casi d'uso prende un identificatore di utente**, e non è
//! una comodità: è la forma che tiene chiusa la domanda di
//! [ADR-036](../../../../../docs/08-decisioni.md) — nessuna rotta proietta un
//! utente per identificatore. Un `PATCH /users/{id}` che accettasse solo il
//! proprio sarebbe la stessa cosa con una guardia applicativa davanti, cioè la
//! porta aperta e un cartello.
//!
//! Chi si legge è `context.require_user()`, che viene dal token e non dal corpo
//! della richiesta.

mod password;
mod profile;

pub use password::{ChangeMyPassword, ChangeMyPasswordCommand};
pub use profile::{GetMyAccount, UpdateMyProfile, UpdateMyProfileCommand};
