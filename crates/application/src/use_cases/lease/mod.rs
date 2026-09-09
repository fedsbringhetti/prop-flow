//! Use case dei contratti di locazione.
//!
//! `GetLease` e `ListLeases` usano `ensure_member` e non `ensure_staff`: è il
//! punto in cui l'inquilino legge il proprio contratto, e *quale* sia il proprio
//! lo decide la Row Level Security attraverso `lease_parties`.

mod activate;
mod add_party;
mod cancel;
mod correct_party;
mod draft;
mod get;
mod list;
mod remove_party;
mod terminate;

pub use activate::{ActivateLease, ActivateLeaseCommand};
pub use add_party::{AddLeaseParty, AddLeasePartyCommand};
pub use cancel::{CancelLease, CancelLeaseCommand};
pub use correct_party::{CorrectLeaseParty, CorrectLeasePartyCommand};
pub use draft::{DraftLease, DraftLeaseCommand};
pub use get::{GetLease, LeaseDetail};
pub use list::{LeaseListPage, ListLeases, ListLeasesQuery};
pub use remove_party::{RemoveLeaseParty, RemoveLeasePartyCommand};
pub use terminate::{TerminateLease, TerminateLeaseCommand};

use propflow_domain::lease::{Lease, LeaseStatus};

/// Contratto con lo stato effettivo già calcolato.
///
/// La derivazione dipende da "oggi", e "oggi" lo decide il `Clock` iniettato: se
/// la calcolasse il bordo HTTP servirebbe un orologio anche là, e i test non
/// potrebbero più decidere in che giorno si trovano.
#[derive(Debug, Clone)]
pub struct LeaseView {
    pub lease: Lease,
    pub effective_status: LeaseStatus,
}

impl LeaseView {
    #[must_use]
    pub fn new(lease: Lease, today: chrono::NaiveDate) -> Self {
        let effective_status = lease.effective_status(today);
        Self {
            lease,
            effective_status,
        }
    }
}
