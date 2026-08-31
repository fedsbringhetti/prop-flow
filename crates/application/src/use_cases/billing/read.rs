use std::sync::Arc;

use propflow_domain::billing::{
    BillStatus, ChargeBalance, ChargeStatus, ChargeType, UtilityBill, UtilityType,
};
use propflow_domain::ports::{
    ChargeBalancePage, ChargeQuery, UnitOfWorkFactory, UtilityBillPage, UtilityBillQuery,
};
use propflow_domain::shared::{ChargeId, LeaseId, PropertyId, UtilityBillId};
use propflow_domain::{Entity, RequestContext};

use crate::authorization;
use crate::error::{AppError, AppResult, UnsupportedQuery};

#[derive(Debug, Clone, Default)]
pub struct ListUtilityBillsQuery {
    pub property_id: Option<PropertyId>,
    pub lease_id: Option<LeaseId>,
    pub status: Option<BillStatus>,
    pub utility_type: Option<UtilityType>,
    pub after: Option<UtilityBillId>,
    pub limit: Option<u16>,
}

/// Elenca le bollette.
///
/// `ensure_member` e non `ensure_staff`: la policy `utility_bills_select` mostra
/// all'inquilino la bolletta imputata al suo contratto, e *quali* siano sue lo
/// decide lei, non un ramo applicativo sul ruolo. È la stessa scelta delle
/// segnalazioni e dei contratti.
///
/// **Nessuno stato è rifiutato dal filtro**, a differenza degli addebiti:
/// `pending_review`, `paid` e `disputed` esistono in database e danno
/// legittimamente zero righe perché l'applicazione non li scrive. `overdue` sugli
/// addebiti è diverso — lì la colonna non porta *mai* quel valore, per nessuna
/// riga, e un filtro che desse sempre zero è un errore del chiamante.
pub struct ListUtilityBills {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl ListUtilityBills {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        query: ListUtilityBillsQuery,
    ) -> AppResult<UtilityBillPage> {
        authorization::ensure_member(context)?;

        // Il limite si tronca invece di essere rifiutato: un client che chiede
        // 10 000 righe ottiene la pagina massima, non un errore da gestire.
        let limit = query
            .limit
            .unwrap_or(UtilityBillQuery::DEFAULT_LIMIT)
            .clamp(1, UtilityBillQuery::MAX_LIMIT);

        let mut uow = self.unit_of_work.begin(context).await?;
        let page = uow
            .utility_bills()
            .list(&UtilityBillQuery {
                property_id: query.property_id,
                lease_id: query.lease_id,
                status: query.status,
                utility_type: query.utility_type,
                after: query.after,
                limit,
            })
            .await?;
        Ok(page)
    }
}

/// Legge una bolletta.
pub struct GetUtilityBill {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl GetUtilityBill {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        id: UtilityBillId,
    ) -> AppResult<UtilityBill> {
        authorization::ensure_member(context)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        uow.utility_bills()
            .find_by_id(id)
            .await?
            .ok_or(AppError::not_found(Entity::UtilityBill))
    }
}

#[derive(Debug, Clone, Default)]
pub struct ListChargesQuery {
    pub lease_id: Option<LeaseId>,
    pub charge_type: Option<ChargeType>,
    pub status: Option<ChargeStatus>,
    /// `Some(true)` = solo gli addebiti ancora da incassare, che è lo scadenzario.
    pub outstanding: Option<bool>,
    pub after: Option<ChargeId>,
    pub limit: Option<u16>,
}

/// Elenca gli addebiti **con il saldo**, letto da `app.v_charge_balances`.
///
/// `ensure_member`: l'inquilino vede i propri, e a deciderlo è
/// `charges_select` attraverso `can_access_lease`.
///
/// **`?status=overdue` è rifiutato con un `422`.** Quello stato è derivato dalla
/// scadenza e nessuna riga lo porta memorizzato, quindi il filtro darebbe *sempre*
/// zero risultati: restituire una pagina vuota sarebbe il modo peggiore di dire
/// che in database quel valore non c'è. Il rimedio è `?outstanding=true`, ed è ciò
/// che il codice `charge.derived_status_filter` dice al client. È la stessa forma
/// di `?status=expiring` sui contratti.
pub struct ListCharges {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl ListCharges {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        query: ListChargesQuery,
    ) -> AppResult<ChargeBalancePage> {
        authorization::ensure_member(context)?;

        if query.status.is_some_and(ChargeStatus::is_derived) {
            return Err(AppError::UnsupportedQuery(
                UnsupportedQuery::ChargeDerivedStatusFilter,
            ));
        }

        let limit = query
            .limit
            .unwrap_or(ChargeQuery::DEFAULT_LIMIT)
            .clamp(1, ChargeQuery::MAX_LIMIT);

        let mut uow = self.unit_of_work.begin(context).await?;
        let page = uow
            .charges()
            .list_balances(&ChargeQuery {
                lease_id: query.lease_id,
                charge_type: query.charge_type,
                status: query.status,
                outstanding: query.outstanding,
                after: query.after,
                limit,
            })
            .await?;
        Ok(page)
    }
}

/// Legge un addebito con il suo saldo.
pub struct GetCharge {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl GetCharge {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        id: ChargeId,
    ) -> AppResult<ChargeBalance> {
        authorization::ensure_member(context)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        uow.charges()
            .find_balance_by_id(id)
            .await?
            .ok_or(AppError::not_found(Entity::Charge))
    }
}
