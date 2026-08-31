use chrono::{DateTime, Utc};
use propflow_domain::billing::Charge;
use propflow_domain::events::{
    ChargeStatusChanged, DomainEvent, DomainEventPayload, EventMetadata,
};
use propflow_domain::payment::Payment;
use propflow_domain::ports::{RepositoryError, UnitOfWork};
use propflow_domain::shared::ChargeId;
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult, Conflict};

/// Traduce il conflitto sull'indice di idempotenza in una condizione parlante.
///
/// Il nome del vincolo è un dettaglio dello schema e non esce mai in risposta.
/// Arrivarci significa che la rilettura per chiave **non** aveva trovato niente e
/// l'inserimento ha comunque violato `payments_idempotency_uk`: cioè la chiave
/// appartiene a un pagamento che la RLS non mostra al chiamante. È il solo `409`
/// che questa via può produrre, ed è irriducibile — distinguere «l'ho già mandato
/// io» da «qualcun altro ha usato la mia chiave» richiederebbe di leggere una riga
/// che al chiamante non spetta.
pub(super) fn map_conflict(error: RepositoryError) -> AppError {
    match &error {
        RepositoryError::Conflict { constraint } if constraint == "payments_idempotency_uk" => {
            AppError::Conflict(Conflict::PaymentIdempotencyKeyReused)
        }
        _ => AppError::from(error),
    }
}

/// Rilegge l'addebito **sotto la RLS del chiamante**, che è la parte che non si
/// può saltare.
///
/// `app.tg_check_polymorphic_ref` verifica che la riga referenziata esista
/// (`SELECT 1 FROM app.charges WHERE id = $1`) ed è `SECURITY DEFINER`: la verifica
/// non dipende dalla visibilità del chiamante e **non guarda l'organizzazione**.
/// Senza questa rilettura si potrebbe inserire un pagamento con il proprio `org_id`
/// verso l'addebito di un'altra organizzazione, e il trigger lo accetterebbe perché
/// la riga esiste davvero.
///
/// È la terza volta che questo progetto usa lo stesso rimedio, dopo
/// `ensure_owner_is_visible` dei documenti e `build_issued_charge` degli addebiti:
/// quando la `WITH CHECK` nomina la sola organizzazione, il controllo sul
/// riferimento tocca al service layer.
pub(super) async fn visible_charge(
    uow: &mut dyn UnitOfWork,
    charge_id: ChargeId,
) -> AppResult<Charge> {
    uow.charges()
        .find_by_id(charge_id)
        .await?
        .ok_or(AppError::not_found(Entity::Charge))
}

/// Ricalcola lo stato dell'addebito dai pagamenti che contano nel saldo, e lo
/// scrive **solo se è cambiato**.
///
/// **Va chiamata solo quando un pagamento entra o esce da `succeeded`.** Una
/// dichiarazione rifiutata non la tocca, perché `pending` non contava: è l'eleganza
/// del flusso in due tempi, e va detta perché altrimenti si è tentati di
/// ricalcolare a ogni transizione.
///
/// La somma si legge **dopo** che il pagamento è stato scritto, nella stessa
/// transazione: è ciò che la fa comprendere il pagamento appena entrato.
///
/// Restituisce l'eventuale evento del cambio di stato, che chi chiama accoda
/// insieme al proprio. Il riaddebito ha già stabilito la forma: due aggregati che
/// si muovono producono due eventi.
pub(super) async fn recalculate_charge_status(
    uow: &mut dyn UnitOfWork,
    context: &RequestContext,
    charge: &mut Charge,
    now: DateTime<Utc>,
) -> AppResult<Option<DomainEventPayload>> {
    let paid = uow.payments().sum_succeeded_for_charge(charge.id()).await?;

    let from_status = charge.status();
    if !charge.apply_payment(paid, now)? {
        return Ok(None);
    }

    uow.charges().update_status(charge).await?;
    Ok(Some(DomainEventPayload::ChargeStatusChanged(
        ChargeStatusChanged::new(
            charge.id(),
            charge.org_id(),
            from_status,
            charge.status(),
            authorization::effective_role(context),
        ),
    )))
}

pub(super) async fn append_all(
    uow: &mut dyn UnitOfWork,
    context: &RequestContext,
    now: DateTime<Utc>,
    payloads: Vec<DomainEventPayload>,
) -> AppResult<()> {
    for payload in payloads {
        uow.events()
            .append(&DomainEvent::new(
                payload,
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
    }
    Ok(())
}

/// Esito di una registrazione o di una dichiarazione.
///
/// `replayed` distingue il pagamento appena scritto da quello **già** registrato
/// con la stessa chiave di idempotenza, ed è ciò che al bordo HTTP diventa `200`
/// invece di `201`. Vive qui e non nell'handler perché è il service layer a sapere
/// se ha scritto: un handler che lo deducesse dovrebbe rifare la stessa domanda al
/// database.
#[derive(Debug, Clone)]
pub struct SubmittedPayment {
    pub payment: Payment,
    pub replayed: bool,
}
