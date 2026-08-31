//! Bollette e addebiti: il primo modulo **contabile** del progetto.
//!
//! È la prima volta che l'applicazione scrive una cifra che qualcuno dovrà
//! versare a qualcun altro, e ne segue il tono di tutto il modulo: una bolletta
//! si corregge finché nessuno l'ha usata, un addebito **non si riscrive** perché
//! è una pretesa già comunicata a una persona.
//!
//! Due aggregati e una via sola perché si leghino:
//!
//! ```text
//! UtilityBill:  draft ──validate──► validated ──recharge──► allocated
//!                 ▲                    │  │                     │
//!                 └──── apply ─────────┘  └──── recharge ───────┘  (ripetibile)
//!                 └──── void ──► (sparisce: status=void + deleted_at)
//!
//! Charge:       (niente draft) ──issue──► issued ──void──────► void
//!                                            └────write_off──► written_off
//! ```
//!
//! Il riaddebito è una **sotto-risorsa della bolletta**
//! (`POST /utility-bills/{id}/recharge`) e non un campo nella creazione
//! dell'addebito: accettare `utility_bill_id` nel corpo di
//! `POST /leases/{id}/charges` nasconderebbe in una creazione un effetto su un
//! altro aggregato — lo stesso motivo per cui le transizioni delle segnalazioni
//! sono sotto-risorse e non un `PATCH` sullo stato.
//!
//! Il perimetro di questa fetta è dichiarato in `docs/09-stato-e-roadmap.md`.
//! Restano fuori l'OCR, i pagamenti (e con loro gli stati `paid` e
//! `partially_paid`, che qui nessuno scrive), il pagamento della bolletta al
//! fornitore, la contestazione, la generazione ricorrente dei canoni e la
//! ripartizione fra coinquilini, che è il modulo delle spese co-living.

mod bill;
mod charge;
mod value_objects;

pub use bill::{
    BillStatus, NewUtilityBill, UtilityBill, UtilityBillChanges, UtilityBillParts, UtilityType,
};
pub use charge::{Charge, ChargeBalance, ChargeParts, ChargeStatus, ChargeType, NewCharge};
pub use value_objects::{Consumption, ConsumptionUnit, InvoiceNumber, SupplierName, SupplierVat};
