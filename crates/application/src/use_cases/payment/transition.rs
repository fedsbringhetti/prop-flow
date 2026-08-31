use std::sync::Arc;

use propflow_domain::events::{DomainEventPayload, PaymentConfirmed, PaymentRejected};
use propflow_domain::payment::Payment;
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::PaymentId;
use propflow_domain::{Entity, RequestContext};

use super::shared::{append_all, recalculate_charge_status, visible_charge};
use crate::authorization;
use crate::error::{AppError, AppResult};

/// Conferma una dichiarazione: `pending` → `succeeded`, e da qui il saldo si muove.
///
/// `ensure_staff`, e non è una scelta applicativa che si possa allentare:
/// `payments_update` è staff soltanto, quindi una rotta che lasciasse confermare a
/// qualcun altro otterrebbe zero righe aggiornate, cioè `auth.denied_by_policy` —
/// il codice riservato ai nostri difetti.
pub struct ConfirmPayment {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl ConfirmPayment {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>, clock: Arc<dyn Clock>) -> Self {
        Self {
            unit_of_work,
            clock,
        }
    }

    /// La sequenza è quella di [`submit`](super::submit), con una differenza che
    /// conta: **il ricalcolo dello stato dell'addebito avviene qui e non alla
    /// dichiarazione**, perché è adesso che il pagamento entra in `succeeded`. Una
    /// dichiarazione `pending` non aveva mosso niente, ed è precisamente ciò che il
    /// flusso in due tempi compra.
    pub async fn execute(&self, context: &RequestContext, id: PaymentId) -> AppResult<Payment> {
        authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut payment = uow
            .payments()
            .find_by_id(id)
            .await?
            .ok_or(AppError::not_found(Entity::Payment))?;

        // L'addebito si rilegge **prima** di scrivere: qui serve per applicarci il
        // pagamento, non per verificare l'organizzazione — quella l'ha già provata
        // la RLS trovando la riga del pagamento. Un `404` a questo punto significa
        // che l'addebito è stato reso invisibile fra la dichiarazione e la
        // conferma, ed è più onesto di una conferma che non aggiorna niente.
        let mut charge = visible_charge(uow.as_mut(), payment.payable().charge_id()).await?;

        payment.confirm(now)?;
        uow.payments().update_status(&payment).await?;

        let mut events = vec![DomainEventPayload::PaymentConfirmed(PaymentConfirmed::new(
            &payment,
            authorization::effective_role(context),
        ))];
        if let Some(event) =
            recalculate_charge_status(uow.as_mut(), context, &mut charge, now).await?
        {
            events.push(event);
        }

        append_all(uow.as_mut(), context, now, events).await?;
        uow.commit().await?;

        Ok(payment)
    }
}

/// Rifiuta una dichiarazione: `pending` → `cancelled`.
///
/// **Non ricalcola lo stato dell'addebito, e non è una svista.** Il ricalcolo
/// scatta solo quando un pagamento entra o esce da `succeeded`, e una dichiarazione
/// rifiutata non contava: `app.v_charge_balances` somma i soli `succeeded` e
/// `partially_refunded`. Ricalcolare qui produrrebbe una `UPDATE` che riscrive lo
/// stesso stato, un evento per un cambiamento che non c'è stato e una riga di audit
/// per nulla.
///
/// **Non rilegge l'addebito** per la stessa ragione: non c'è niente da scriverci
/// sopra, e l'organizzazione l'ha già provata la RLS trovando la riga del
/// pagamento. Il `charge_id` che l'evento porta viene dall'aggregato, non da una
/// query.
pub struct RejectPayment {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl RejectPayment {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>, clock: Arc<dyn Clock>) -> Self {
        Self {
            unit_of_work,
            clock,
        }
    }

    pub async fn execute(&self, context: &RequestContext, id: PaymentId) -> AppResult<Payment> {
        authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut payment = uow
            .payments()
            .find_by_id(id)
            .await?
            .ok_or(AppError::not_found(Entity::Payment))?;

        payment.reject(now)?;
        uow.payments().update_status(&payment).await?;

        append_all(
            uow.as_mut(),
            context,
            now,
            vec![DomainEventPayload::PaymentRejected(PaymentRejected::new(
                &payment,
                authorization::effective_role(context),
            ))],
        )
        .await?;
        uow.commit().await?;

        Ok(payment)
    }
}
