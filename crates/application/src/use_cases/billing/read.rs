use std::sync::Arc;

use propflow_domain::billing::{
    BillStatus, Charge, ChargeBalance, ChargeStatus, ChargeType, UtilityBill, UtilityType,
};
use propflow_domain::ports::{
    ChargeBalancePage, ChargeQuery, LeaseLabels, RechargedAmount, UnitOfWork, UnitOfWorkFactory,
    UtilityBillPage, UtilityBillQuery,
};
use propflow_domain::shared::{ChargeId, LeaseId, PropertyId, UtilityBillId};
use propflow_domain::{Entity, RequestContext};

use rust_decimal::Decimal;

use crate::authorization;
use crate::error::{AppError, AppResult, UnsupportedQuery};

/// I nomi di ciò che i contratti citati da una risposta affittano.
///
/// **Terza occorrenza dello stesso modello di lettura**, e la regolarità è la parte
/// che conviene ricordare: `LeaseLabels` è nato nella sezione 3 per l'elenco dei
/// contratti, [ADR-049](../../../../../docs/08-decisioni.md) l'ha applicato ai nomi
/// delle persone sulle segnalazioni, e qui serve perché la scheda di un addebito e
/// quella di una bolletta portavano un identificatore verso una schermata — cioè
/// mostravano «Contratti» al posto di *dove* e *di chi*.
///
/// Si risolve **attraverso il contratto** e non dalle colonne della bolletta: ciò
/// che il collegamento nomina è il contratto, e il nome di un contratto è la casa
/// più la stanza. Una bolletta senza contratto non ha niente da nominare, e il
/// campo resta assente come prima.
#[derive(Debug, Clone, Default)]
pub struct Places(Vec<LeaseLabels>);

impl Places {
    #[must_use]
    pub fn of(&self, lease_id: Option<LeaseId>) -> Option<&LeaseLabels> {
        let lease_id = lease_id?;
        self.0.iter().find(|labels| labels.lease_id == lease_id)
    }
}

/// Quanto di ciascuna bolletta citata è **già stato riaddebitato**.
///
/// Serve a dire quanto resta: riaddebitare una bolletta d'immobile a tre contratti
/// sono tre sottrazioni a mente, e a mente si sbagliano. **Non è un vincolo** —
/// riaddebitare più del totale resta possibile ed è scritto come deliberato — è un
/// numero che aiuta.
///
/// Zero è la risposta giusta per una bolletta mai riaddebitata: la query raggruppa,
/// quindi una bolletta senza addebiti semplicemente non ha una riga.
#[derive(Debug, Clone, Default)]
pub struct Recharged(Vec<RechargedAmount>);

impl Recharged {
    #[must_use]
    pub fn of(&self, utility_bill_id: UtilityBillId) -> Decimal {
        self.0
            .iter()
            .find(|row| row.utility_bill_id == utility_bill_id)
            .map_or(Decimal::ZERO, |row| row.amount)
    }
}

/// Ciò che un caso d'uso restituisce, **con i nomi dei luoghi che cita**.
///
/// Vale per ogni risposta e non per le sole letture, ed è la lezione della sezione
/// 5 applicata qui: se le scritture non li risolvessero, la stessa risposta
/// porterebbe i campi a volte sì e a volte no, e un client non potrebbe distinguere
/// «immobile archiviato» da «non risolto» — mostrerebbe l'etichetta neutra subito
/// dopo un'azione riuscita.
#[derive(Debug)]
pub struct Placed<T> {
    pub value: T,
    pub places: Places,
    /// Vuoto su ciò che non cita una bolletta, e non è un campo di comodo.
    ///
    /// Un involucro per aggregato sarebbe sembrato più pulito e non regge il terzo
    /// caso: il riaddebito restituisce **una bolletta e un addebito insieme**,
    /// quindi cita entrambi i modelli e li vuole tutti e due. Con due involucri
    /// distinti quel caso avrebbe avuto bisogno di un terzo, che li contiene: un
    /// tipo in più per non ammettere che i modelli di lettura di questo modulo sono
    /// due e non uno per aggregato.
    pub recharged: Recharged,
}

/// `Placed<T>` si legge come la `T` che avvolge.
///
/// È la forma di `axum::Json<T>` e `axum::Extension<T>`: un decoratore che
/// aggiunge contesto e non lo interpone. Senza, ogni chiamante che vuole
/// l'aggregato scriverebbe `.value`, e il rumore finirebbe per far preferire il
/// tipo nudo — cioè per rimettere i nomi solo sulle letture, che è il difetto da
/// cui si è partiti. `places` resta invece **esplicito**, perché chi lo usa lo sta
/// usando apposta.
impl<T> std::ops::Deref for Placed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

/// I nomi dei contratti citati, in **una query** e non una per riga.
pub(super) async fn places_of(
    uow: &mut dyn UnitOfWork,
    lease_ids: impl Iterator<Item = Option<LeaseId>>,
) -> AppResult<Places> {
    let mut wanted: Vec<LeaseId> = Vec::new();
    for id in lease_ids.flatten() {
        if !wanted.contains(&id) {
            wanted.push(id);
        }
    }
    Ok(Places(uow.leases().labels(&wanted).await?))
}

/// Quanto è già stato riaddebitato delle bollette citate, in **una query**.
pub(super) async fn recharged_of(
    uow: &mut dyn UnitOfWork,
    bill_ids: &[UtilityBillId],
) -> AppResult<Recharged> {
    Ok(Recharged(uow.utility_bills().recharged(bill_ids).await?))
}

/// Il punto unico in cui una risposta con un addebito si costruisce.
pub(super) async fn placed_charge(
    uow: &mut dyn UnitOfWork,
    charge: Charge,
) -> AppResult<Placed<Charge>> {
    let places = places_of(uow, std::iter::once(Some(charge.lease_id()))).await?;
    Ok(Placed {
        value: charge,
        places,
        recharged: Recharged::default(),
    })
}

/// Il punto unico in cui una risposta con una bolletta si costruisce.
pub(super) async fn placed_bill(
    uow: &mut dyn UnitOfWork,
    bill: UtilityBill,
) -> AppResult<Placed<UtilityBill>> {
    let places = places_of(uow, std::iter::once(bill.lease_id())).await?;
    let recharged = recharged_of(uow, &[bill.id()]).await?;
    Ok(Placed {
        value: bill,
        places,
        recharged,
    })
}

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
    ) -> AppResult<Placed<UtilityBillPage>> {
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
        let places = places_of(uow.as_mut(), page.items.iter().map(UtilityBill::lease_id)).await?;
        let ids: Vec<UtilityBillId> = page.items.iter().map(UtilityBill::id).collect();
        let recharged = recharged_of(uow.as_mut(), &ids).await?;
        Ok(Placed {
            value: page,
            places,
            recharged,
        })
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
    ) -> AppResult<Placed<UtilityBill>> {
        authorization::ensure_member(context)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        let bill = uow
            .utility_bills()
            .find_by_id(id)
            .await?
            .ok_or(AppError::not_found(Entity::UtilityBill))?;
        placed_bill(uow.as_mut(), bill).await
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
    ) -> AppResult<Placed<ChargeBalancePage>> {
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
        let places = places_of(
            uow.as_mut(),
            page.items
                .iter()
                .map(|balance| Some(balance.charge.lease_id())),
        )
        .await?;
        Ok(Placed {
            value: page,
            places,
            recharged: Recharged::default(),
        })
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
    ) -> AppResult<Placed<ChargeBalance>> {
        authorization::ensure_member(context)?;

        let mut uow = self.unit_of_work.begin(context).await?;
        let balance = uow
            .charges()
            .find_balance_by_id(id)
            .await?
            .ok_or(AppError::not_found(Entity::Charge))?;
        let places = places_of(
            uow.as_mut(),
            std::iter::once(Some(balance.charge.lease_id())),
        )
        .await?;
        Ok(Placed {
            value: balance,
            places,
            recharged: Recharged::default(),
        })
    }
}
