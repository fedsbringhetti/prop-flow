use std::sync::Arc;

use chrono::{DateTime, Utc};
use propflow_domain::events::{
    DomainEventPayload, SettlementCancelled, SettlementConfirmed, SettlementDeclared,
    SettlementRejected,
};
use propflow_domain::expense::{NewSettlement, Settlement, SettlementStatus};
use propflow_domain::payment::PaymentMethod;
use propflow_domain::ports::{Clock, SettlementPage, SettlementQuery, UnitOfWorkFactory};
use propflow_domain::shared::{ExpenseGroupId, SettlementId, UserId};
use propflow_domain::{Denial, Entity, RequestContext};
use rust_decimal::Decimal;

use super::shared::{append_all, open_group, visible_group};
use crate::authorization;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct DeclareSettlementCommand {
    pub group_id: ExpenseGroupId,
    /// A chi si è pagato.
    pub to_user_id: UserId,
    pub amount: Decimal,
    pub method: PaymentMethod,
    /// Quando il denaro si è mosso. `None` significa «adesso», e la conferma lo
    /// risolve.
    pub settled_at: Option<DateTime<Utc>>,
    pub note: Option<String>,
}

/// Chi paga dichiara di aver saldato: nasce **`pending`** e **non muove i saldi**.
///
/// **Il pagante è sempre chi chiama**, e non un campo del comando: la policy
/// `settlements_insert` chiede `from_user_id = app.current_user_id()` a chi non è
/// staff, quindi lasciarlo scegliere produrrebbe zero righe inserite, cioè
/// `auth.denied_by_policy` — il codice riservato ai nostri difetti. È la stessa
/// forma di `DeclarePayment`.
///
/// **Non si verifica che il saldo dovuto sia quello dichiarato**, e non è una
/// dimenticanza: si può saldare in parte, si può saldare in anticipo, e un
/// controllo sul saldo obbligherebbe a decidere che cosa fare di un versamento in
/// eccesso — che è la stessa domanda che i pagamenti hanno risolto lasciando
/// passare il sovrapagamento. La vista somma i regolamenti confermati e basta.
///
/// **Non si verifica nemmeno che il destinatario sia un membro del gruppo.** Lo
/// schema non ha un vincolo, e la ragione per cui qui non si aggiunge il controllo
/// che invece la spesa fa è che il caso legittimo esiste: si salda con chi è
/// **uscito** dal gruppo, ed è precisamente ciò che la migration `0027` esiste per
/// rendere possibile — il debitore uscito resta nei saldi finché non paga. Un
/// controllo sui membri attivi renderebbe irraggiungibile la chiusura di quel
/// conto.
pub struct DeclareSettlement {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl DeclareSettlement {
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
        command: DeclareSettlementCommand,
    ) -> AppResult<Settlement> {
        let org_id = authorization::ensure_member(context)?;
        let from_user_id = context
            .user_id()
            .ok_or(AppError::Forbidden(Denial::AuthenticationRequired))?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let group = open_group(uow.as_mut(), command.group_id).await?;

        let settlement = Settlement::declare(
            NewSettlement {
                id: SettlementId::generate(),
                org_id,
                group_id: group.id(),
                from_user_id,
                to_user_id: command.to_user_id,
                amount: command.amount,
                method: command.method,
                settled_at: command.settled_at,
                note: command.note,
            },
            now,
        )?;

        uow.settlements().insert(&settlement).await?;
        append_all(
            uow.as_mut(),
            context,
            now,
            vec![DomainEventPayload::SettlementDeclared(
                SettlementDeclared::new(&settlement),
            )],
        )
        .await?;
        uow.commit().await?;

        Ok(settlement)
    }
}

/// Chi riceve conferma: `pending` → `confirmed`, e da qui i saldi si muovono.
///
/// **Conferma il creditore, non lo staff**, ed è la differenza che conta con i
/// pagamenti: là lo staff vedeva il movimento sull'estratto conto, qui l'unico che
/// sa se il bonifico è arrivato è chi doveva riceverlo. Lo schema lo dice già —
/// `settlements.confirmed_by` esiste, e la `USING` di `settlements_update` nomina
/// `to_user_id`.
///
/// Lo staff può comunque, perché la `USING` lo ammette e perché un
/// amministratore che sistema un conto bloccato è un caso reale. Il ruolo con cui
/// si è agito resta in `audit_log`, e `confirmed_by` registra **chi**.
///
/// **Non ricalcola niente**, a differenza di `ConfirmPayment`: il saldo del gruppo
/// non è memorizzato da nessuna parte, si legge da
/// `app.v_expense_group_balances`. È la conseguenza pratica della decisione 7 —
/// `is_settled` non si scrive — e vale la pena dirla, perché chi arriva dal modulo
/// dei pagamenti si aspetta un `recalculate_*` che qui non serve e non deve
/// esistere.
pub struct ConfirmSettlement {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl ConfirmSettlement {
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
        id: SettlementId,
    ) -> AppResult<Settlement> {
        let caller = authorized_caller(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut settlement = find_settlement(uow.as_mut(), id).await?;
        ensure_creditor(context, &settlement, caller)?;

        settlement.confirm(caller, now)?;
        uow.settlements().update_status(&settlement).await?;

        append_all(
            uow.as_mut(),
            context,
            now,
            vec![DomainEventPayload::SettlementConfirmed(
                SettlementConfirmed::new(&settlement),
            )],
        )
        .await?;
        uow.commit().await?;

        Ok(settlement)
    }
}

/// Chi riceve dice che il denaro non è arrivato: `pending` → `rejected`.
///
/// L'uscita negativa nasce **insieme** a `pending` e non dopo, per la ragione di
/// sempre: uno stato in cui si entra e da cui non si esce è peggio di nessuno
/// stato.
pub struct RejectSettlement {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl RejectSettlement {
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
        id: SettlementId,
    ) -> AppResult<Settlement> {
        let caller = authorized_caller(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut settlement = find_settlement(uow.as_mut(), id).await?;
        ensure_creditor(context, &settlement, caller)?;

        settlement.reject(now)?;
        uow.settlements().update_status(&settlement).await?;

        append_all(
            uow.as_mut(),
            context,
            now,
            vec![DomainEventPayload::SettlementRejected(
                SettlementRejected::new(&settlement),
            )],
        )
        .await?;
        uow.commit().await?;

        Ok(settlement)
    }
}

/// Chi ha dichiarato ritira: `pending` → `cancelled`.
///
/// # Questa operazione esiste perché lo schema la ammette, non per simmetria
///
/// Il piano lasciava aperta la domanda «lo stato `cancelled` si scrive o no», da
/// decidere eseguendo. La risposta è **sì**, e la ragione è una sola: la `USING` di
/// `settlements_update` nomina `from_user_id`, mentre `payments_update` è staff
/// soltanto. Sui pagamenti «chi dichiara non può ritirare» **non era una scelta di
/// prodotto**, era una conseguenza della policy, documentata come controintuitiva.
/// Qui la policy dice l'opposto, e importare il vincolo dei pagamenti
/// significherebbe copiarne la forma senza copiarne la ragione — che è esattamente
/// il genere di conclusione che si eredita senza riverificarla.
///
/// Il fatto sostanziale è che **fra pari il ritiro è lecito, verso il proprietario
/// no**: chi ha sbagliato destinatario o importo non deve dover chiedere al
/// creditore di rifiutare una dichiarazione che sa già essere sbagliata.
///
/// Ritira **chi ha dichiarato**, non chi riceve: quello è il rifiuto, ed è un
/// fatto diverso — «non l'ho mandato» contro «non è arrivato».
pub struct CancelSettlement {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl CancelSettlement {
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
        id: SettlementId,
    ) -> AppResult<Settlement> {
        let caller = authorized_caller(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut settlement = find_settlement(uow.as_mut(), id).await?;

        if settlement.from_user_id() != caller && !context.is_staff() {
            return Err(AppError::Forbidden(Denial::SettlementNotDeclaredByYou));
        }

        settlement.cancel(now)?;
        uow.settlements().update_status(&settlement).await?;

        append_all(
            uow.as_mut(),
            context,
            now,
            vec![DomainEventPayload::SettlementCancelled(
                SettlementCancelled::new(&settlement),
            )],
        )
        .await?;
        uow.commit().await?;

        Ok(settlement)
    }
}

#[derive(Debug, Clone)]
pub struct ListSettlementsQuery {
    pub group_id: ExpenseGroupId,
    pub status: Option<SettlementStatus>,
    pub after: Option<SettlementId>,
    pub limit: Option<u16>,
}

/// Elenca i regolamenti di un gruppo.
///
/// **Serve al creditore per trovare che cosa confermare**, ed è per questo che
/// esiste: senza, l'unico modo di raggiungere una dichiarazione pendente sarebbe
/// conoscerne l'identificatore, cioè averlo ricevuto per un canale che non c'è.
/// Il filtro sullo stato è servito dall'indice parziale `settlements_pending_ix`.
///
/// Il gruppo si rilegge prima, per la ragione di sempre: una pagina vuota è il
/// modo peggiore di dire «non è tuo».
pub struct ListSettlements {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl ListSettlements {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        query: ListSettlementsQuery,
    ) -> AppResult<SettlementPage> {
        authorization::ensure_member(context)?;

        let limit = query
            .limit
            .unwrap_or(SettlementQuery::DEFAULT_LIMIT)
            .clamp(1, SettlementQuery::MAX_LIMIT);

        let mut uow = self.unit_of_work.begin(context).await?;
        let group = visible_group(uow.as_mut(), query.group_id).await?;

        uow.settlements()
            .list_for_group(&SettlementQuery {
                group_id: group.id(),
                status: query.status,
                after: query.after,
                limit,
            })
            .await
            .map_err(AppError::from)
    }
}

fn authorized_caller(context: &RequestContext) -> AppResult<UserId> {
    authorization::ensure_member(context)?;
    context
        .user_id()
        .ok_or(AppError::Forbidden(Denial::AuthenticationRequired))
}

async fn find_settlement(
    uow: &mut dyn propflow_domain::ports::UnitOfWork,
    id: SettlementId,
) -> AppResult<Settlement> {
    uow.settlements()
        .find_by_id(id)
        .await?
        .ok_or(AppError::not_found(Entity::Settlement))
}

/// Confermare e rifiutare spettano a chi **riceve**.
///
/// Un `403` con un codice proprio e non un `404`: la riga il chiamante la vede —
/// `settlements_select` la mostra a tutto il gruppo — e nascondergliela adesso
/// sarebbe incoerente. Il codice `settlement.not_addressed_to_you` gli dice che il
/// rimedio è chiedere all'altro, non cercarsi un permesso.
fn ensure_creditor(
    context: &RequestContext,
    settlement: &Settlement,
    caller: UserId,
) -> AppResult<()> {
    if settlement.to_user_id() == caller || context.is_staff() {
        return Ok(());
    }
    Err(AppError::Forbidden(Denial::SettlementNotAddressedToYou))
}
