//! Pagamenti: il denaro che entra.
//!
//! È il modulo che fa muovere `paid_amount`, fino a qui sempre `0.00` in ogni
//! risposta perché `app.payments` era vuota — non perché il calcolo sbagliasse.
//!
//! Due vie di ingresso e un solo stato che conta:
//!
//! ```text
//! Payment:  (staff)     ──record───►  succeeded          muove il saldo
//!           (inquilino) ──declare──►  pending ──confirm──► succeeded
//!                                        └────reject────► cancelled
//! ```
//!
//! Lo schema è costruito per questo flusso e non lo si sta forzando: le tre righe
//! che lo dicono erano già nella `0014` e nella `0015` —
//! `payments_insert` ammette `is_org_staff(org_id) OR payer_user_id =
//! current_user_id()`, `payments_update` è staff soltanto, e
//! `app.v_charge_balances` conta i soli `succeeded` e `partially_refunded`.
//!
//! Lo stato dell'addebito **si ricalcola** dai pagamenti e non si dichiara: è
//! [`Charge::apply_payment`](crate::billing::Charge::apply_payment), la prima
//! scrittura che la `GRANT UPDATE (status)` della migration `0023` rende possibile.
//! Il ricalcolo scatta **solo** quando un pagamento entra o esce da `succeeded`:
//! una dichiarazione rifiutata non lo tocca, perché `pending` non contava.
//!
//! Il perimetro di questa fetta è dichiarato in `docs/09-stato-e-roadmap.md`.
//! Restano fuori i rimborsi, i provider veri (`stripe`, `gocardless`, e con loro i
//! webhook), i pagamenti di quote e regolamenti del modulo co-living, e il
//! pagamento della bolletta al fornitore — che non è modellato affatto, perché
//! `app.payable_type` non comprende `utility_bill`.

mod aggregate;
mod value_objects;

pub use aggregate::{NewPayment, Payable, Payment, PaymentMethod, PaymentParts, PaymentStatus};
pub use value_objects::IdempotencyKey;
