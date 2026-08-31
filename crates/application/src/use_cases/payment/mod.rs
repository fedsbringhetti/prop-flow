//! Pagamenti: registrazione, dichiarazione, conferma, rifiuto, lettura.
//!
//! **Due vie di ingresso, e una sola porta da cui il saldo si muove.** Lo staff
//! registra un incasso visto sull'estratto conto e questo nasce `succeeded`;
//! l'inquilino dichiara di aver pagato e la sua riga nasce `pending`, che è ciò che
//! rende la dichiarazione visibile senza renderla vera. A muovere il saldo è solo
//! il passaggio a `succeeded`, alla registrazione o alla conferma.
//!
//! Non si sta forzando niente sullo schema: le tre righe che disegnano questo
//! flusso erano già nella `0014` e nella `0015` — `payments_insert` ammette
//! `is_org_staff(org_id) OR payer_user_id = current_user_id()`, `payments_update` è
//! staff soltanto, e `app.v_charge_balances` conta i soli `succeeded` e
//! `partially_refunded`.
//!
//! Tre punti in cui il service layer fa ciò che lo schema **non** fa, e tutti e tre
//! sono difese e non prudenza:
//!
//! 1. **la chiave di idempotenza si rilegge prima di inserire.** L'indice
//!    `payments_idempotency_uk` impedisce il doppio inserimento, ma da solo lo
//!    trasformerebbe in un `409`: la rilettura è ciò che lo trasforma in un `200`
//!    con il pagamento già registrato. Vedi ADR-033;
//! 2. **l'addebito si rilegge sotto la RLS del chiamante prima di scrivere**,
//!    perché `app.tg_check_polymorphic_ref` verifica che la riga referenziata
//!    esista e **non guarda l'organizzazione**;
//! 3. **lo stato dell'addebito si ricalcola dai pagamenti**, e solo quando un
//!    pagamento entra o esce da `succeeded`.
//!
//! Il perimetro della fetta è in `docs/09-stato-e-roadmap.md`.

mod read;
mod shared;
mod submit;
mod transition;

pub use read::{GetPayment, ListPayments, ListPaymentsQuery};
pub use shared::SubmittedPayment;
pub use submit::{DeclarePayment, DeclarePaymentCommand, RecordPayment, RecordPaymentCommand};
pub use transition::{ConfirmPayment, RejectPayment};
