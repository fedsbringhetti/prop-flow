use std::sync::Arc;

use propflow_domain::payment::Payment;
use propflow_domain::ports::{PaymentPage, PaymentQuery, UnitOfWorkFactory};
use propflow_domain::shared::{ChargeId, PaymentId};
use propflow_domain::{Entity, RequestContext};

use super::shared::visible_charge;
use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct ListPaymentsQuery {
    pub charge_id: ChargeId,
    pub after: Option<PaymentId>,
    pub limit: Option<u16>,
}

/// Elenca i pagamenti di un addebito.
///
/// `ensure_member`, e *quali* righe siano di chi le chiede lo decide
/// `payments_select` — staff, oppure chi ha pagato, oppure chi ha incassato — non
/// un ramo applicativo sul ruolo. È la stessa scelta delle segnalazioni, dei
/// contratti e della contabilità.
///
/// **Ne segue una conseguenza da dire ad alta voce**: un inquilino vede i propri
/// pagamenti e **non** quelli che lo staff ha registrato senza attribuirglieli.
/// Non è un difetto dell'elenco, è `payer_user_id` lasciato vuoto — vedi il campo
/// omonimo di
/// [`RecordPaymentCommand`](super::RecordPaymentCommand). Il totale incassato
/// resta comunque visibile a tutti nel `paid_amount` del saldo, che viene dalla
/// vista e non da questo elenco.
///
/// L'addebito si rilegge prima: senza, un `charge_id` inesistente o invisibile
/// darebbe una pagina vuota invece di un `404`, che è il modo peggiore di dire «non
/// è tuo».
pub struct ListPayments {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl ListPayments {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        query: ListPaymentsQuery,
    ) -> AppResult<PaymentPage> {
        authorization::ensure_member(context)?;

        // Il limite si tronca invece di essere rifiutato: un client che chiede
        // 10 000 righe ottiene la pagina massima, non un errore da gestire.
        let limit = query
            .limit
            .unwrap_or(PaymentQuery::DEFAULT_LIMIT)
            .clamp(1, PaymentQuery::MAX_LIMIT);

        let mut uow = self.unit_of_work.begin(context).await?;
        let charge = visible_charge(uow.as_mut(), query.charge_id).await?;

        uow.payments()
            .list_for_charge(&PaymentQuery {
                charge_id: charge.id(),
                after: query.after,
                limit,
            })
            .await
            .map_err(AppError::from)
    }
}

/// Legge un pagamento.
pub struct GetPayment {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl GetPayment {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(&self, context: &RequestContext, id: PaymentId) -> AppResult<Payment> {
        authorization::ensure_member(context)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        uow.payments()
            .find_by_id(id)
            .await?
            .ok_or(AppError::not_found(Entity::Payment))
    }
}
