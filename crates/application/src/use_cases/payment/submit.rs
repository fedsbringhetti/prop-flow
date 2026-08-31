use std::sync::Arc;

use chrono::{DateTime, Utc};
use propflow_domain::events::{DomainEventPayload, PaymentConfirmed, PaymentDeclared};
use propflow_domain::payment::{
    IdempotencyKey, NewPayment, Payable, Payment, PaymentMethod, PaymentStatus,
};
use propflow_domain::ports::{Clock, UnitOfWorkFactory};
use propflow_domain::shared::{ChargeId, OrgId, PaymentId, UserId};
use propflow_domain::{Denial, RequestContext};
use rust_decimal::Decimal;

use super::shared::{
    SubmittedPayment, append_all, map_conflict, recalculate_charge_status, visible_charge,
};
use crate::authorization;
use crate::error::{AppError, AppResult, Conflict};

#[derive(Debug, Clone)]
pub struct RecordPaymentCommand {
    pub charge_id: ChargeId,
    pub amount: Decimal,
    pub method: PaymentMethod,
    /// Quando il denaro si è mosso. `None` significa «adesso».
    ///
    /// È un `TIMESTAMPTZ` e va interpretato: l'inquilino che dice «ieri» e
    /// `is_past_due`, che usa il `CURRENT_DATE` **del database**, non parlano
    /// necessariamente dello stesso giorno. È lo stesso scarto già registrato per
    /// gli addebiti in ADR-028, e qui si vede una seconda volta.
    pub paid_at: Option<DateTime<Utc>>,
    pub reference: Option<String>,
    /// A chi attribuire l'incasso.
    ///
    /// **Non è anagrafica**: `payments_select` mostra la riga a
    /// `payer_user_id = current_user_id()`, quindi valorizzarlo è ciò che rende il
    /// pagamento visibile a chi lo ha fatto. Un incasso registrato senza
    /// attribuirlo resta visibile allo staff soltanto, il che è lecito — un bonifico
    /// da un conto che non si sa a chi ricondurre esiste — ma va scelto, non subìto.
    pub payer_user_id: Option<UserId>,
    pub idempotency_key: IdempotencyKey,
}

/// Registra un incasso già avvenuto: nasce **`succeeded`** e muove il saldo.
///
/// `ensure_staff`: è la via di chi vede il movimento sull'estratto conto. Non c'è
/// un passo intermedio perché non c'è un secondo osservatore da attendere, ed è lo
/// stesso argomento per cui un addebito non ha la bozza.
pub struct RecordPayment {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl RecordPayment {
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
        command: RecordPaymentCommand,
    ) -> AppResult<SubmittedPayment> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();

        submit(
            &self.unit_of_work,
            context,
            Submission {
                org_id,
                charge_id: command.charge_id,
                amount: command.amount,
                method: command.method,
                paid_at: command.paid_at,
                reference: command.reference,
                payer: Payer::Recording(command.payer_user_id),
                idempotency_key: command.idempotency_key,
            },
            now,
        )
        .await
    }
}

#[derive(Debug, Clone)]
pub struct DeclarePaymentCommand {
    pub charge_id: ChargeId,
    pub amount: Decimal,
    pub method: PaymentMethod,
    pub paid_at: Option<DateTime<Utc>>,
    pub reference: Option<String>,
    pub idempotency_key: IdempotencyKey,
}

/// L'inquilino dichiara di aver pagato: nasce **`pending`** e **non muove il
/// saldo**.
///
/// `ensure_member` e non `ensure_staff`, e non è una concessione: `payments_insert`
/// ammette `is_org_staff(org_id) OR payer_user_id = current_user_id()`, quindi lo
/// schema era già costruito per questo dalla `0014`. `pending` non è uno stato di
/// comodo — è ciò che rende la dichiarazione **visibile senza renderla vera**.
///
/// **Il pagante è sempre chi chiama**, e non un campo del comando: la policy chiede
/// `payer_user_id = current_user_id()`, quindi lasciarlo scegliere produrrebbe zero
/// righe inserite, cioè `auth.denied_by_policy` — il codice riservato ai nostri
/// difetti. Uno staff che voglia attribuire un incasso a qualcun altro usa
/// [`RecordPayment`], che ha il campo apposta.
///
/// **Chi dichiara non può ritirare.** `payments_update` è solo staff, e va detto
/// nell'API perché è controintuitivo: si può creare qualcosa che poi non si può
/// disfare. L'uscita esiste ed è il rifiuto, che è dello staff.
pub struct DeclarePayment {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl DeclarePayment {
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
        command: DeclarePaymentCommand,
    ) -> AppResult<SubmittedPayment> {
        let org_id = authorization::ensure_member(context)?;
        let payer_user_id = context
            .user_id()
            .ok_or(AppError::Forbidden(Denial::AuthenticationRequired))?;
        let now = self.clock.now();

        submit(
            &self.unit_of_work,
            context,
            Submission {
                org_id,
                charge_id: command.charge_id,
                amount: command.amount,
                method: command.method,
                paid_at: command.paid_at,
                reference: command.reference,
                payer: Payer::Declaring(payer_user_id),
                idempotency_key: command.idempotency_key,
            },
            now,
        )
        .await
    }
}

/// Chi ha pagato, e da quale delle due vie.
///
/// È l'**unica** differenza fra registrare e dichiarare, e per questo la sequenza
/// non si duplica: cambiano il costruttore del dominio e l'evento, non l'ordine dei
/// passi.
///
/// Un enum e non una coppia `(Option<UserId>, bool)` perché le due combinazioni
/// che restano fuori non hanno significato: una dichiarazione senza dichiarante
/// non è inseribile — `payments_insert` chiede
/// `payer_user_id = current_user_id()` a chi non è staff — e questa forma rende
/// quello stato **non rappresentabile** invece di farlo gestire a un ramo che
/// nessuno saprebbe come chiudere.
enum Payer {
    /// L'inquilino che dichiara: è sempre chi chiama, e la policy lo esige.
    Declaring(UserId),
    /// Lo staff che registra: può attribuire l'incasso a qualcuno o lasciarlo non
    /// attribuito, e in quel caso resta visibile allo staff soltanto.
    Recording(Option<UserId>),
}

impl Payer {
    const fn user_id(&self) -> Option<UserId> {
        match self {
            Self::Declaring(id) => Some(*id),
            Self::Recording(id) => *id,
        }
    }
}

struct Submission {
    org_id: OrgId,
    charge_id: ChargeId,
    amount: Decimal,
    method: PaymentMethod,
    paid_at: Option<DateTime<Utc>>,
    reference: Option<String>,
    payer: Payer,
    idempotency_key: IdempotencyKey,
}

/// La sequenza condivisa dalle due vie di ingresso.
///
/// L'ordine non è negoziabile, e ogni passo esiste per una ragione che si paga in
/// denaro se la si salta:
///
/// 1. **la rilettura per chiave viene prima dell'inserimento.** Se a generare la
///    chiave fosse il server, ogni ritentativo produrrebbe un secondo pagamento e
///    un saldo che scende **due volte**, e `audit_log` mostrerebbe due inserimenti
///    perfettamente legittimi. Vedi ADR-033;
/// 2. **la rilettura dell'addebito viene prima della scrittura**, perché il trigger
///    polimorfico non guarda l'organizzazione — vedi [`visible_charge`];
/// 3. **il ricalcolo dello stato viene dopo l'inserimento**, perché la somma deve
///    comprendere il pagamento appena entrato;
/// 4. **il ricalcolo scatta solo se il pagamento conta nel saldo.** Una
///    dichiarazione `pending` non lo tocca.
async fn submit(
    unit_of_work: &Arc<dyn UnitOfWorkFactory>,
    context: &RequestContext,
    submission: Submission,
    now: DateTime<Utc>,
) -> AppResult<SubmittedPayment> {
    let mut uow = unit_of_work.begin(context).await?;

    if let Some(existing) = uow
        .payments()
        .find_by_idempotency_key(&submission.idempotency_key)
        .await?
    {
        // La chiave è già stata usata. Se punta all'addebito richiesto è il doppio
        // invio che l'idempotenza esiste per assorbire, e si restituisce il
        // pagamento già registrato; se punta altrove il client sta riusando una
        // chiave per una richiesta diversa, e un `200` con un pagamento che non è
        // quello chiesto sarebbe una bugia.
        if existing.payable() != Payable::Charge(submission.charge_id) {
            return Err(AppError::Conflict(Conflict::PaymentIdempotencyKeyReused));
        }
        return Ok(SubmittedPayment {
            payment: existing,
            replayed: true,
        });
    }

    let mut charge = visible_charge(uow.as_mut(), submission.charge_id).await?;

    let parts = NewPayment {
        id: PaymentId::generate(),
        org_id: submission.org_id,
        payable: Payable::Charge(charge.id()),
        payer_user_id: submission.payer.user_id(),
        amount: submission.amount,
        method: submission.method,
        paid_at: submission.paid_at,
        reference: submission.reference,
        idempotency_key: submission.idempotency_key,
    };

    let payment = match submission.payer {
        Payer::Declaring(_) => Payment::declare(parts, now)?,
        Payer::Recording(_) => Payment::record(parts, now)?,
    };

    // L'addebito su cui la pretesa è caduta rifiuta il pagamento **prima** che si
    // scriva qualcosa: `apply_payment` lo direbbe comunque, ma solo dopo aver
    // inserito una riga che la transazione dovrebbe poi disfare.
    if payment.counts_towards_balance() && charge.status().is_claim_dropped() {
        charge.apply_payment(Decimal::ZERO, now)?;
    }

    uow.payments()
        .insert(&payment)
        .await
        .map_err(map_conflict)?;

    let mut events = Vec::with_capacity(2);
    events.push(match submission.payer {
        Payer::Declaring(declared_by) => {
            DomainEventPayload::PaymentDeclared(PaymentDeclared::new(&payment, declared_by))
        }
        // **Le due vie convergono su `PaymentConfirmed`**: chi consuma reagisce al
        // fatto che il denaro è diventato vero, non alla porta da cui è passato. A
        // distinguerle basta l'esistenza — o l'assenza — del `PaymentDeclared` che
        // porta lo stesso `payment_id`.
        Payer::Recording(_) => DomainEventPayload::PaymentConfirmed(PaymentConfirmed::new(
            &payment,
            authorization::effective_role(context),
        )),
    });

    if payment.status() == PaymentStatus::Succeeded
        && let Some(event) =
            recalculate_charge_status(uow.as_mut(), context, &mut charge, now).await?
    {
        events.push(event);
    }

    append_all(uow.as_mut(), context, now, events).await?;
    uow.commit().await?;

    Ok(SubmittedPayment {
        payment,
        replayed: false,
    })
}
