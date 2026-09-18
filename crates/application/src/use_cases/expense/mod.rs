//! Spese co-living: gruppi, composizione, spese con ripartizione, saldi,
//! regolamenti.
//!
//! **È il denaro che i coinquilini si devono fra loro**, e non una pretesa del
//! proprietario: quella è il modulo degli addebiti. Il proprietario può guardare —
//! `can_access_expense_group` risponde vero per lo staff — ma non ne è parte, e
//! nessuna riga di questo modulo entra nella sua contabilità.
//!
//! Ne segue che **quasi tutte le operazioni sono `ensure_member` e non
//! `ensure_staff`**, che è l'opposto della fetta contabile. Non è una concessione:
//! le policy della `0014` erano già scritte così — `expense_groups_insert` chiede
//! `is_org_member`, tutto il resto chiede `can_access_expense_group`. Un co-living
//! che non può organizzarsi da sé non è il prodotto descritto in
//! `docs/01-prodotto.md`.
//!
//! # Quattro punti in cui il service layer fa ciò che lo schema **non** fa
//!
//! 1. **il gruppo si rilegge sotto la RLS del chiamante prima di ogni scrittura.**
//!    `app.expense_groups` non ha il trigger `tg_assert_org_matches_property` che
//!    hanno `rooms`, `leases`, `tickets` e `utility_bills` — verificato nella
//!    `0013`, non dedotto — quindi un `property_id` di un'altra organizzazione
//!    entrerebbe senza che nulla protesti;
//! 2. **chi ha anticipato una spesa si verifica contro i membri attivi.** Su
//!    `app.expenses.paid_by_user_id` non c'è alcun vincolo verso la membership, e
//!    il danno sarebbe silenzioso: la vista dei saldi parte dai membri, quindi quel
//!    credito non comparirebbe a nessuno e la somma dei saldi smetterebbe di fare
//!    zero;
//! 3. **si esce da un gruppo per sé stessi**, mentre la `USING` di
//!    `expense_group_members_update` ammette qualunque membro attivo. È una regola
//!    del contratto dell'API e **non un confine di sicurezza** — vedi
//!    [`LeaveGroup`];
//! 4. **la membership si rilegge prima di iscrivere.**
//!    `expense_group_members_group_user_uk` non è parziale, quindi chi rientra è un
//!    `UPDATE` e non un `INSERT`.
//!
//! # La trappola che costa di più
//!
//! Il trigger `expense_shares_balanced` è **`DEFERRABLE INITIALLY DEFERRED`**:
//! l'errore che segnala quote che non sommano all'importo esce dal **`COMMIT`**, non
//! dall'`INSERT` che l'ha causato. Il commit di [`RecordExpense`] passa quindi da
//! `commit_expense`, che lo mappa; scriverlo come `uow.commit().await?` per
//! abitudine trasformerebbe un errore di dominio in un `500`.
//!
//! Il perimetro della fetta è in `docs/09-stato-e-roadmap.md`.

mod balance;
mod group;
mod record;
mod settlement;
mod shared;

pub use balance::GetGroupBalances;
pub use group::{
    AddGroupMember, AddGroupMemberCommand, ArchiveExpenseGroup, CreateExpenseGroup,
    CreateExpenseGroupCommand, GetExpenseGroup, LeaveGroup, ListExpenseGroups,
    ListExpenseGroupsQuery,
};
pub use record::{
    CancelExpense, GetExpense, ListExpenses, ListExpensesQuery, RecordExpense, RecordExpenseCommand,
};
pub use settlement::{
    CancelSettlement, ConfirmSettlement, DeclareSettlement, DeclareSettlementCommand,
    ListSettlements, ListSettlementsQuery, RejectSettlement,
};
