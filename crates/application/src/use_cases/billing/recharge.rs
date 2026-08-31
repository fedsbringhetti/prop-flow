use std::sync::Arc;

use chrono::NaiveDate;
use propflow_domain::billing::{Charge, ChargeType, UtilityBill};
use propflow_domain::events::{ChargeIssued, DomainEventPayload, UtilityBillStatusChanged};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::{LeaseId, UtilityBillId};
use propflow_domain::{DomainError, Entity, RequestContext};
use rust_decimal::Decimal;

use super::bill::append_all;
use super::charge::{NewChargeRequest, build_issued_charge};
use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct RechargeUtilityBillCommand {
    /// Il contratto a cui addebitare la quota.
    ///
    /// **Non deve coincidere con `utility_bills.lease_id`**: quello dice a chi la
    /// bolletta è *imputata* — e quindi chi la vede — mentre questo dice a chi si
    /// chiede il denaro. In un co-living con contratti per stanza sono
    /// legittimamente diversi, e la stessa bolletta si riaddebita a più contratti
    /// ripetendo l'operazione.
    pub lease_id: LeaseId,
    /// La quota da riaddebitare, che **non** è per forza il totale della bolletta.
    ///
    /// Nessuno verifica che le parti sommino al totale, e non è una dimenticanza:
    /// lo schema non ha un vincolo, un controllo applicativo sarebbe soggetto a
    /// race condition fra due riaddebiti concorrenti — il criterio di ADR-015 per
    /// cui quel genere di invariante sta nel database o non sta da nessuna parte —
    /// e il posto in cui «le parti sommano» è già espresso correttamente è il
    /// `CONSTRAINT TRIGGER … DEFERRABLE` di `expense_shares`, cioè il modulo
    /// co-living. **Riaddebitare più del totale è oggi possibile.**
    pub amount: Decimal,
    pub due_date: NaiveDate,
    pub description: Option<String>,
    pub notes: Option<String>,
}

/// Esito del riaddebito: i due aggregati che la transazione ha toccato.
#[derive(Debug)]
pub struct Recharge {
    pub bill: UtilityBill,
    pub charge: Charge,
}

/// Riaddebita una bolletta a un contratto.
///
/// È **una sotto-risorsa della bolletta** e non un campo nella creazione
/// dell'addebito: `POST /api/v1/utility-bills/{id}/recharge`. L'alternativa —
/// accettare `utility_bill_id` nel corpo di `POST /api/v1/leases/{id}/charges` —
/// nasconderebbe in una creazione un effetto su un altro aggregato, che è lo
/// stesso motivo per cui le transizioni delle segnalazioni sono sotto-risorse e
/// non un `PATCH` sullo stato. Così c'è **una via sola** perché una bolletta e un
/// addebito si leghino, ed è quella che tiene onesto lo stato della bolletta.
///
/// Tutto in **una transazione sola** e con **due eventi**, uno per aggregato: chi
/// consuma il solo `billing.charge.issued` non saprebbe che la bolletta è stata
/// usata, e chi consuma il solo `billing.utility_bill.status_changed` non saprebbe
/// a chi.
pub struct RechargeUtilityBill {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl RechargeUtilityBill {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>, clock: Arc<dyn Clock>) -> Self {
        Self {
            unit_of_work,
            clock,
        }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        bill_id: UtilityBillId,
        command: RechargeUtilityBillCommand,
    ) -> AppResult<Recharge> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut bill = uow
            .utility_bills()
            .find_by_id(bill_id)
            .await?
            .ok_or(AppError::not_found(Entity::UtilityBill))?;

        // La valuta non è dichiarabile e resta `EUR` ovunque, quindi l'importo del
        // riaddebito e quello della bolletta sono confrontabili per costruzione. Il
        // controllo che *non* si fa è la somma delle quote: vedi il commento sul
        // campo `amount` del comando.
        if command.amount.is_sign_negative() {
            return Err(AppError::Validation(DomainError::validation(
                "amount",
                "must be a non-negative amount",
            )));
        }

        let from_status = bill.status();
        // Prima la transizione della bolletta, che è quella che può rifiutare: se
        // la bolletta non è validata non ha senso costruire l'addebito.
        bill.mark_allocated(now)?;

        let charge = build_issued_charge(
            uow.as_mut(),
            org_id,
            NewChargeRequest {
                lease_id: command.lease_id,
                // Il tipo lo decide il caso d'uso e non il chiamante: un addebito
                // nato da una bolletta **è** un riaddebito di utenza, e lasciarlo
                // scegliere permetterebbe di emettere un canone che cita una
                // bolletta.
                charge_type: ChargeType::UtilityRecharge,
                description: command.description,
                // Il periodo dell'addebito è quello della bolletta: è ciò che
                // l'inquilino sta pagando, e ricopiarlo dal comando permetterebbe
                // di dichiararne uno diverso da quello fatturato.
                period_start: bill.period_start(),
                period_end: bill.period_end(),
                amount: command.amount,
                due_date: command.due_date,
                utility_bill_id: Some(bill.id()),
                notes: command.notes,
            },
            now,
        )
        .await?;

        uow.charges().insert(&charge).await?;
        uow.utility_bills().update(&bill).await?;

        append_all(
            uow.as_mut(),
            context,
            now,
            vec![
                DomainEventPayload::ChargeIssued(ChargeIssued::from_charge(&charge)),
                DomainEventPayload::UtilityBillStatusChanged(UtilityBillStatusChanged::new(
                    bill.id(),
                    org_id,
                    from_status,
                    bill.status(),
                    authorization::effective_role(context),
                )),
            ],
        )
        .await?;
        uow.commit().await?;

        Ok(Recharge { bill, charge })
    }
}
