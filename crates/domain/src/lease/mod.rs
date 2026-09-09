//! Contratti di locazione e parti contrattuali.
//!
//! È il modulo che rende bidirezionale il prodotto: la visibilità dell'inquilino
//! su un immobile passa dai contratti (`app.can_access_property`), quindi senza
//! contratti il ruolo `tenant` esiste nello schema e non può fare nulla.

mod aggregate;
mod party;

pub use aggregate::{Lease, LeaseDraft, LeaseParts, LeaseStatus, LeaseType, RentFrequency};
pub use party::{LeaseParty, LeasePartyChanges, LeasePartyParts, LeasePartyRole, NewLeaseParty};
