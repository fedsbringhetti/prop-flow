//! Spese co-living: il denaro che i coinquilini si devono **fra loro**.
//!
//! È una cosa diversa da tutto il resto del progetto. Non è una pretesa del
//! proprietario verso l'inquilino — quella è
//! [`Charge`](crate::billing::Charge) — è un conto **fra pari**. Il proprietario
//! può guardarlo, perché `can_access_expense_group` risponde vero per lo staff
//! dell'organizzazione, ma non ne è parte e nessuna riga di questo modulo entra
//! nella sua contabilità.
//!
//! ```text
//! ExpenseGroup ──join/leave──►  ExpenseGroupMember  (weight)
//!       │
//!       ├── Expense ──split──►  ExpenseShare        somma == amount, sempre
//!       │
//!       └── Settlement:  (chi paga) ──declare──►  pending ──confirm (chi riceve)──► confirmed
//!                                                    ├─────reject  (chi riceve)───► rejected
//!                                                    └─────cancel  (chi paga)─────► cancelled
//! ```
//!
//! I saldi **si leggono** da `app.v_expense_group_balances` e non si ricalcolano:
//! vedi [`GroupBalance`].
//!
//! # Le tre cose da sapere prima di toccare questo modulo
//!
//! 1. **La somma delle quote è l'importo, sempre.** Il trigger
//!    `expense_shares_balanced` lo impone, e lo impone al `COMMIT` perché è
//!    `DEFERRABLE INITIALLY DEFERRED`: l'errore non esce dalla query che l'ha
//!    causato. L'aritmetica che lo garantisce sta in [`Split`], lavora in interi e
//!    non divide mai;
//! 2. **una spesa e le sue quote nascono insieme.** [`Expense::record`]
//!    restituisce entrambe, e non esiste un modo di costruire l'una senza le
//!    altre;
//! 3. **tre tabelle su quattro non hanno `deleted_at`.**
//!    `expense_group_members` ha `left_at`, `expense_shares` e `settlements` non
//!    hanno niente. Solo `app.expenses` e `app.expense_groups` hanno il soft
//!    delete. Filtrare `deleted_at` per abitudine dà un errore di colonna
//!    inesistente: fastidioso e innocuo, perché fallisce subito.
//!
//! # Il perimetro, dichiarato in `docs/09-stato-e-roadmap.md`
//!
//! Restano fuori: la **semplificazione dei debiti** (il «chi paga chi» a numero
//! minimo di bonifici, che è una comodità di presentazione e non un fatto
//! contabile); i tre metodi di ripartizione oltre `equal` e `weighted`;
//! `expense_shares.is_settled`, che è una seconda definizione di «saldato» che la
//! vista non guarda; il legame fra regolamento e pagamento, per cui vedi
//! [`Settlement`]; la **ripartizione automatica di una bolletta**
//! (`expenses.utility_bill_id`), che richiederebbe di decidere quale bolletta
//! riguarda quale gruppo — domanda a cui oggi non c'è modo di rispondere, perché
//! `expense_groups.property_id` è opzionale; e gli **allegati alle spese**, perché
//! [`AttachmentOwner`](crate::document::AttachmentOwner) non comprende `expense`.

// `aggregate` e non `expense`: un modulo con lo stesso nome di quello che lo
// contiene è un errore di lint, e il nome segue comunque la convenzione di
// `payment/` e `ticket/`. Le altre due entità con un ciclo di vita proprio —
// gruppo e regolamento — hanno un file per nome, perché lì il nome dice di più.
mod aggregate;
mod balance;
mod group;
mod settlement;
mod split;
mod value_objects;

pub use aggregate::{
    Expense, ExpenseCategory, ExpenseParts, ExpenseShare, ExpenseShareParts, NewExpense,
    RecordedExpense,
};
pub use balance::GroupBalance;
pub use group::{
    ExpenseGroup, ExpenseGroupKind, ExpenseGroupMember, ExpenseGroupMemberParts, ExpenseGroupParts,
    NewExpenseGroup, NewExpenseGroupMember,
};
pub use settlement::{NewSettlement, Settlement, SettlementParts, SettlementStatus};
pub use split::{Participant, Share, Split, SplitMethod};
pub use value_objects::Weight;
