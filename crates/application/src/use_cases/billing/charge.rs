use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use propflow_domain::billing::{Charge, ChargeType, NewCharge};
use propflow_domain::events::{ChargeIssued, ChargeStatusChanged, DomainEventPayload};
use propflow_domain::ports::{Clock, UnitOfWork, UnitOfWorkFactory};
use propflow_domain::shared::{ChargeId, LeaseId};
use propflow_domain::{DomainResult, Entity, RequestContext};
use rust_decimal::Decimal;

use super::bill::append_all;
use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct IssueChargeCommand {
    pub lease_id: LeaseId,
    pub charge_type: ChargeType,
    pub description: Option<String>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub amount: Decimal,
    pub due_date: NaiveDate,
    pub notes: Option<String>,
}

/// Emette un addebito su un contratto.
///
/// **Il comando non accetta `utility_bill_id`**, e non è una dimenticanza: c'è una
/// via sola perché una bolletta e un addebito si leghino, ed è
/// `RechargeUtilityBill`. Accettarlo qui nasconderebbe in una creazione un effetto
/// su un altro aggregato — la bolletta che dovrebbe passare ad `allocated` e non
/// lo farebbe — lasciando una bolletta che dichiara di non essere stata usata
/// mentre un addebito la cita.
pub struct IssueCharge {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl IssueCharge {
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
        command: IssueChargeCommand,
    ) -> AppResult<Charge> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;

        let charge = build_issued_charge(
            uow.as_mut(),
            org_id,
            NewChargeRequest {
                lease_id: command.lease_id,
                charge_type: command.charge_type,
                description: command.description,
                period_start: command.period_start,
                period_end: command.period_end,
                amount: command.amount,
                due_date: command.due_date,
                utility_bill_id: None,
                notes: command.notes,
            },
            now,
        )
        .await?;

        uow.charges().insert(&charge).await?;

        append_all(
            uow.as_mut(),
            context,
            now,
            vec![DomainEventPayload::ChargeIssued(ChargeIssued::from_charge(
                &charge,
            ))],
        )
        .await?;
        uow.commit().await?;

        Ok(charge)
    }
}

/// Dati con cui costruire un addebito, condivisi fra l'emissione diretta e il
/// riaddebito di una bolletta.
pub(super) struct NewChargeRequest {
    pub lease_id: LeaseId,
    pub charge_type: ChargeType,
    pub description: Option<String>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub amount: Decimal,
    pub due_date: NaiveDate,
    pub utility_bill_id: Option<propflow_domain::shared::UtilityBillId>,
    pub notes: Option<String>,
}

/// Costruisce un addebito **dopo aver riletto il contratto**, che è la parte che
/// non si può saltare.
///
/// La `WITH CHECK` di `charges_insert` è
/// `org_id = app.current_org_id() AND app.is_org_staff(org_id)`: nomina
/// l'organizzazione e **non** il contratto. Su `app.charges` non c'è nemmeno un
/// trigger `assert_org_matches_*`, che invece esiste su `rooms`, `leases`,
/// `tickets` e `utility_bills`, e la foreign key verso `app.leases` non guarda
/// l'organizzazione.
///
/// Ne segue che uno staff può inserire un addebito con il **proprio** `org_id` e
/// il `lease_id` di un contratto di un'altra organizzazione, se ne conosce l'UUID.
/// La riga entra e poi **è invisibile a tutti**: `charges_select` chiede
/// `can_access_lease(lease_id)`, falsa per chi l'ha scritta (non è staff
/// dell'altra organizzazione) e falsa per l'altra organizzazione (l'`org_id` non è
/// il suo). Non si può rileggere, non si può cancellare — il ruolo applicativo non
/// ha `DELETE` — e non comparirà mai in nessun saldo.
///
/// La rilettura qui è ciò che lo impedisce: sotto la RLS del chiamante, il
/// contratto di un'altra organizzazione semplicemente non si trova. È lo stesso
/// `ensure_owner_is_visible` dei documenti, e per la stessa ragione.
pub(super) async fn build_issued_charge(
    uow: &mut dyn UnitOfWork,
    org_id: propflow_domain::shared::OrgId,
    request: NewChargeRequest,
    now: DateTime<Utc>,
) -> AppResult<Charge> {
    let lease = uow
        .leases()
        .find_by_id(request.lease_id)
        .await?
        .ok_or(AppError::not_found(Entity::Lease))?;

    let charge = Charge::issue(
        NewCharge {
            id: ChargeId::generate(),
            org_id,
            lease_id: lease.id(),
            charge_type: request.charge_type,
            description: request.description,
            period_start: request.period_start,
            period_end: request.period_end,
            amount: request.amount,
            due_date: request.due_date,
            utility_bill_id: request.utility_bill_id,
            notes: request.notes,
        },
        now,
    )?;

    Ok(charge)
}

/// Le due transizioni dell'addebito condividono la sequenza, come quelle della
/// bolletta e delle segnalazioni.
///
/// La scrittura passa da `update_status`, che nomina la **sola** colonna concessa:
/// la migration `0023` ha revocato l'`UPDATE` su `app.charges` e lo riconcede su
/// `status`, quindi una query che nominasse `amount` fallirebbe con
/// `permission denied` — che è precisamente la difesa voluta.
async fn transition<T>(
    unit_of_work: &Arc<dyn UnitOfWorkFactory>,
    clock: &Arc<dyn Clock>,
    context: &RequestContext,
    id: ChargeId,
    apply: T,
) -> AppResult<Charge>
where
    T: FnOnce(&mut Charge, DateTime<Utc>) -> DomainResult<()>,
{
    let org_id = authorization::ensure_staff(context)?;
    let now = clock.now();

    let mut uow = unit_of_work.begin(context).await?;
    // `find_by_id` e non `find_balance_by_id`: una transizione non guarda quanto
    // è stato incassato, e non deve pagare il join con la vista.
    let mut charge = uow
        .charges()
        .find_by_id(id)
        .await?
        .ok_or(AppError::not_found(Entity::Charge))?;

    let from_status = charge.status();
    apply(&mut charge, now)?;

    uow.charges().update_status(&charge).await?;

    append_all(
        uow.as_mut(),
        context,
        now,
        vec![DomainEventPayload::ChargeStatusChanged(
            ChargeStatusChanged::new(
                charge.id(),
                org_id,
                from_status,
                charge.status(),
                authorization::effective_role(context),
            ),
        )],
    )
    .await?;
    uow.commit().await?;

    Ok(charge)
}

/// Annulla un addebito: la richiesta è caduta.
///
/// L'addebito **resta visibile** all'inquilino, marcato `void`. È l'asimmetria con
/// la bolletta, che annullata sparisce: qualcuno si è visto chiedere quella somma
/// e deve poter vedere che la richiesta non c'è più.
pub struct VoidCharge {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl VoidCharge {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>, clock: Arc<dyn Clock>) -> Self {
        Self {
            unit_of_work,
            clock,
        }
    }

    pub async fn execute(&self, context: &RequestContext, id: ChargeId) -> AppResult<Charge> {
        transition(
            &self.unit_of_work,
            &self.clock,
            context,
            id,
            |charge, now| charge.void(now),
        )
        .await
    }
}

/// Rinuncia all'incasso di un addebito.
///
/// Diverso dall'annullamento, e la distinzione conta nei report: `void` dice «non
/// doveva essere chiesto», `written_off` dice «era dovuto e ci rinunciamo».
/// Entrambi escono dall'insoluto, ma solo il secondo è una perdita.
pub struct WriteOffCharge {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl WriteOffCharge {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>, clock: Arc<dyn Clock>) -> Self {
        Self {
            unit_of_work,
            clock,
        }
    }

    pub async fn execute(&self, context: &RequestContext, id: ChargeId) -> AppResult<Charge> {
        transition(
            &self.unit_of_work,
            &self.clock,
            context,
            id,
            |charge, now| charge.write_off(now),
        )
        .await
    }
}
