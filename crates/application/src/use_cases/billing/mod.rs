//! Bollette e addebiti: registrazione, correzione, validazione, riaddebito.
//!
//! **Le scritture sono dello staff** (`ensure_staff`): registrare una bolletta o
//! emettere un addebito è contabilità. **Le letture sono di ogni membro**
//! (`ensure_member`), e *quali* righe siano di chi le chiede lo decidono le policy
//! — `utility_bills_select` attraverso il contratto imputato, `charges_select`
//! attraverso `can_access_lease` — non un ramo applicativo sul ruolo. È la stessa
//! scelta delle segnalazioni e dei contratti.
//!
//! Due punti in cui il service layer fa ciò che lo schema **non** fa, ed entrambi
//! sono difese e non prudenza:
//!
//! 1. [`references`] verifica che stanza e contratto appartengano all'immobile
//!    della bolletta — nessun vincolo lo impone — e va rifatto a ogni correzione,
//!    perché quelle colonne restano scrivibili;
//! 2. l'emissione di un addebito rilegge il contratto prima di inserire, perché la
//!    `WITH CHECK` di `charges_insert` non chiama `can_access_lease` e senza quel
//!    controllo si scrive una **riga fantasma**, invisibile a tutti e impossibile
//!    da cancellare.
//!
//! Il perimetro della fetta è in `docs/09-stato-e-roadmap.md`.

mod bill;
mod charge;
mod read;
mod recharge;
mod references;

pub use bill::{
    RegisterUtilityBill, RegisterUtilityBillCommand, UpdateUtilityBill, UpdateUtilityBillCommand,
    ValidateUtilityBill, VoidUtilityBill,
};
pub use charge::{IssueCharge, IssueChargeCommand, VoidCharge, WriteOffCharge};
pub use read::{
    GetCharge, GetUtilityBill, ListCharges, ListChargesQuery, ListUtilityBills,
    ListUtilityBillsQuery,
};
pub use recharge::{Recharge, RechargeUtilityBill, RechargeUtilityBillCommand};
