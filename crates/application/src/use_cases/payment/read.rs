use std::sync::Arc;

use propflow_domain::payment::{Payment, PaymentStatus};
use propflow_domain::ports::{PaymentPage, PaymentQuery, UnitOfWorkFactory};
use propflow_domain::shared::{ChargeId, LeaseId, PaymentId};
use propflow_domain::{Entity, RequestContext};

use super::shared::visible_charge;
use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Default)]
pub struct ListPaymentsQuery {
    /// L'addebito, quando l'elenco è il suo. `None` è la collezione.
    pub charge_id: Option<ChargeId>,
    pub status: Option<PaymentStatus>,
    pub lease_id: Option<LeaseId>,
    pub after: Option<PaymentId>,
    pub limit: Option<u16>,
}

/// Elenca i pagamenti: quelli di un addebito, o quelli dell'organizzazione.
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
/// # Due forme, e la prima non è cambiata
///
/// **Con `charge_id` l'addebito si rilegge prima**, come faceva la fetta contabile:
/// senza, un `charge_id` inesistente o invisibile darebbe una pagina vuota invece
/// di un `404`, che è il modo peggiore di dire «non è tuo».
///
/// **Senza `charge_id` è la collezione**, ed è la coda che il modulo dei pagamenti
/// dal lato del proprietario richiede: *quali dichiarazioni aspettano una
/// conferma*. Qui non c'è niente da rileggere prima — non c'è una risorsa padre di
/// cui verificare la visibilità — e a restringere è la sola policy. Un elenco vuoto
/// qui **è** la risposta corretta: significa che non ci sono pagamenti, non che
/// qualcosa non si è trovato.
///
/// Il filtro `lease_id` non rilegge il contratto per la stessa ragione: è un
/// filtro, non un padre. Un contratto altrui non produce un `403` ma zero righe,
/// perché `payments_select` non mostra quelle righe comunque — e distinguere i due
/// casi richiederebbe di leggere il contratto per dire che non si può leggere.
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

        // La rilettura c'è solo quando l'addebito è nel percorso. `visible_charge`
        // traduce l'assenza in `404`, che è ciò che una sotto-risorsa deve dire; la
        // collezione non ha un padre da cui derivare quel `404`.
        let charge_id = match query.charge_id {
            Some(id) => Some(visible_charge(uow.as_mut(), id).await?.id()),
            None => None,
        };

        uow.payments()
            .list(&PaymentQuery {
                charge_id,
                status: query.status,
                lease_id: query.lease_id,
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
