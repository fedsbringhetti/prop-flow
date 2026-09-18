use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use propflow_domain::billing::{
    Consumption, InvoiceNumber, NewUtilityBill, SupplierName, SupplierVat, UtilityBill,
    UtilityBillChanges, UtilityType,
};
use propflow_domain::events::{
    DomainEvent, DomainEventPayload, EventMetadata, UtilityBillRegistered,
    UtilityBillStatusChanged, UtilityBillUpdated,
};
use propflow_domain::ports::{Clock, RepositoryError, UnitOfWork, UnitOfWorkFactory};
use propflow_domain::shared::{LeaseId, PropertyId, RoomId, UtilityBillId};
use propflow_domain::{DomainResult, Entity, RequestContext};
use rust_decimal::Decimal;

use super::read::{Placed, placed_bill};
use super::references::ensure_belong_to_property;
use crate::authorization;
use crate::error::{AppError, AppResult, Conflict};

/// Traduce il conflitto di deduplica in una condizione parlante.
///
/// Il nome del vincolo è un dettaglio dello schema e non esce mai in risposta: da
/// qui nasce `utility_bill.duplicate`, che dice al client cosa mostrare senza
/// fargli leggere la prosa.
fn map_conflict(error: RepositoryError) -> AppError {
    match &error {
        RepositoryError::Conflict { constraint } if constraint == "utility_bills_dedup_uk" => {
            AppError::Conflict(Conflict::DuplicateUtilityBill)
        }
        _ => AppError::from(error),
    }
}

#[derive(Debug, Clone)]
pub struct RegisterUtilityBillCommand {
    pub property_id: PropertyId,
    pub room_id: Option<RoomId>,
    /// Il contratto a cui la bolletta è imputata.
    ///
    /// **Vale la pena valorizzarlo**: `utility_bills_select` mostra la bolletta
    /// allo staff *oppure* a chi è parte di questo contratto, quindi una bolletta
    /// senza contratto indicato non la vede nessun inquilino — nemmeno quello che
    /// consuma l'utenza.
    pub lease_id: Option<LeaseId>,
    pub utility_type: UtilityType,
    pub supplier_name: Option<SupplierName>,
    pub supplier_vat: Option<SupplierVat>,
    pub invoice_number: Option<InvoiceNumber>,
    pub issue_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub total_amount: Decimal,
    pub taxable_amount: Option<Decimal>,
    pub vat_amount: Option<Decimal>,
    pub consumption: Option<Consumption>,
    pub notes: Option<String>,
}

/// Registra una bolletta.
///
/// `ensure_staff`: registrare una bolletta è contabilità, e l'inquilino la legge
/// ma non la scrive.
pub struct RegisterUtilityBill {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl RegisterUtilityBill {
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
        command: RegisterUtilityBillCommand,
    ) -> AppResult<Placed<UtilityBill>> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;

        let property = uow
            .properties()
            .find_by_id(command.property_id)
            .await?
            .ok_or(AppError::not_found(Entity::Property))?;

        ensure_belong_to_property(
            uow.as_mut(),
            property.id(),
            command.room_id,
            command.lease_id,
        )
        .await?;

        let bill = UtilityBill::register(
            NewUtilityBill {
                id: UtilityBillId::generate(),
                org_id,
                property_id: property.id(),
                room_id: command.room_id,
                lease_id: command.lease_id,
                utility_type: command.utility_type,
                supplier_name: command.supplier_name,
                supplier_vat: command.supplier_vat,
                invoice_number: command.invoice_number,
                issue_date: command.issue_date,
                due_date: command.due_date,
                period_start: command.period_start,
                period_end: command.period_end,
                total_amount: command.total_amount,
                taxable_amount: command.taxable_amount,
                vat_amount: command.vat_amount,
                consumption: command.consumption,
                notes: command.notes,
            },
            now,
        )?;

        uow.utility_bills()
            .insert(&bill)
            .await
            .map_err(map_conflict)?;

        uow.events()
            .append(&DomainEvent::new(
                DomainEventPayload::UtilityBillRegistered(UtilityBillRegistered::from_bill(&bill)),
                now,
                EventMetadata::from_context(context),
            ))
            .await?;
        let placed = placed_bill(uow.as_mut(), bill).await?;
        uow.commit().await?;

        Ok(placed)
    }
}

/// Correzione parziale di una bolletta.
pub type UpdateUtilityBillCommand = UtilityBillChanges;

/// Corregge una bolletta.
///
/// **Su una bolletta validata la correzione disfa la validazione**: lo stato torna
/// a `draft` e chi la valida di nuovo deve rifarlo apposta. Il caso d'uso emette
/// quindi fino a **due** eventi — l'aggiornamento e, quando la validazione salta,
/// il cambio di stato — perché altrimenti chi consuma non avrebbe modo di scoprire
/// che una validazione è stata disfatta senza che nessuno l'abbia chiesto.
pub struct UpdateUtilityBill {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl UpdateUtilityBill {
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
        id: UtilityBillId,
        changes: UpdateUtilityBillCommand,
    ) -> AppResult<Placed<UtilityBill>> {
        let org_id = authorization::ensure_staff(context)?;
        let now = self.clock.now();

        let mut uow = self.unit_of_work.begin(context).await?;
        let mut bill = uow
            .utility_bills()
            .find_by_id(id)
            .await?
            .ok_or(AppError::not_found(Entity::UtilityBill))?;

        // Stanza e contratto si riverificano **qui**, e non solo alla
        // registrazione: restano scrivibili, e lo schema continua a non
        // controllarli. Si leggono solo se il `PATCH` li nomina — altrimenti
        // sarebbero due query per nulla su ogni correzione di un importo.
        if changes.touches_property_references() {
            let room_id = changes.room_id.unwrap_or(bill.room_id());
            let lease_id = changes.lease_id.unwrap_or(bill.lease_id());
            ensure_belong_to_property(uow.as_mut(), bill.property_id(), room_id, lease_id).await?;
        }

        let from_status = bill.status();
        if !bill.apply(&changes, now)? {
            // Nessuna scrittura, nessun evento, nessuna riga di audit — e
            // soprattutto nessuna validazione disfatta: la stessa proprietà di
            // `Property::apply`.
            return placed_bill(uow.as_mut(), bill).await;
        }

        uow.utility_bills()
            .update(&bill)
            .await
            .map_err(map_conflict)?;

        let mut events = vec![DomainEventPayload::UtilityBillUpdated(UtilityBillUpdated {
            utility_bill_id: bill.id(),
            org_id,
            changed_fields: changes.changed_field_names(),
        })];
        if bill.status() != from_status {
            events.push(DomainEventPayload::UtilityBillStatusChanged(
                UtilityBillStatusChanged::new(
                    bill.id(),
                    org_id,
                    from_status,
                    bill.status(),
                    authorization::effective_role(context),
                ),
            ));
        }

        append_all(uow.as_mut(), context, now, events).await?;
        let placed = placed_bill(uow.as_mut(), bill).await?;
        uow.commit().await?;

        Ok(placed)
    }
}

/// Le transizioni della bolletta condividono la sequenza: autorizza → carica →
/// chiama il dominio → persiste → emette il cambio di stato → commit.
///
/// Vive in una funzione e non copiata tre volte per la stessa ragione delle
/// segnalazioni: ciò che distingue le transizioni è la chiamata al dominio, non
/// l'ordine dei passi, e `UtilityBillStatusChanged` lo emette sempre questa
/// funzione, così nessuna transizione può dimenticarlo.
async fn transition<T>(
    unit_of_work: &Arc<dyn UnitOfWorkFactory>,
    clock: &Arc<dyn Clock>,
    context: &RequestContext,
    id: UtilityBillId,
    apply: T,
) -> AppResult<Placed<UtilityBill>>
where
    T: FnOnce(&mut UtilityBill, DateTime<Utc>) -> DomainResult<()>,
{
    let org_id = authorization::ensure_staff(context)?;
    let now = clock.now();

    let mut uow = unit_of_work.begin(context).await?;
    let mut bill = uow
        .utility_bills()
        .find_by_id(id)
        .await?
        .ok_or(AppError::not_found(Entity::UtilityBill))?;

    let from_status = bill.status();
    apply(&mut bill, now)?;

    uow.utility_bills().update(&bill).await?;

    append_all(
        uow.as_mut(),
        context,
        now,
        vec![DomainEventPayload::UtilityBillStatusChanged(
            UtilityBillStatusChanged::new(
                bill.id(),
                org_id,
                from_status,
                bill.status(),
                authorization::effective_role(context),
            ),
        )],
    )
    .await?;
    let placed = placed_bill(uow.as_mut(), bill).await?;
    uow.commit().await?;

    Ok(placed)
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

/// Valida una bolletta: una persona ha controllato che i numeri siano quelli.
pub struct ValidateUtilityBill {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl ValidateUtilityBill {
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
        id: UtilityBillId,
    ) -> AppResult<Placed<UtilityBill>> {
        // Chi valida viene dal contesto e non dal comando: `validated_by` deve
        // dire *chi ha controllato*, e accettarlo dal corpo della richiesta
        // permetterebbe di attribuire la validazione a qualcun altro.
        let validated_by = context.user_id().ok_or(AppError::Forbidden(
            propflow_domain::Denial::AuthenticationRequired,
        ))?;
        transition(&self.unit_of_work, &self.clock, context, id, |bill, now| {
            bill.validate(validated_by, now)
        })
        .await
    }
}

/// Dichiara pagata al fornitore una bolletta.
///
/// **Non è un pagamento del modulo `app.payments`**, ed è la distinzione che
/// questa fetta non ha modellato: là c'è il denaro che entra — un addebito che
/// viene saldato, con chi paga e chi riceve — e un fornitore non è un utente del
/// sistema. Qui si registra un fatto avvenuto fuori dal prodotto, con la sua data,
/// perché l'elenco delle bollette non distingueva quelle saldate da quelle da
/// saldare — che è la sola domanda che ci si fa guardandolo.
pub struct MarkUtilityBillPaid {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl MarkUtilityBillPaid {
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
        id: UtilityBillId,
    ) -> AppResult<Placed<UtilityBill>> {
        transition(&self.unit_of_work, &self.clock, context, id, |bill, now| {
            bill.mark_paid(now)
        })
        .await
    }
}

/// Annulla una bolletta, che **sparisce**.
///
/// Il soft delete non è un extra: `utility_bills_dedup_uk` è parziale su
/// `deleted_at IS NULL`, quindi senza di esso la stessa fattura non sarebbe
/// reinseribile mai più. Lo valorizza `UtilityBill::void` insieme allo stato, così
/// le due cose non possono separarsi.
pub struct VoidUtilityBill {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl VoidUtilityBill {
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
        id: UtilityBillId,
    ) -> AppResult<Placed<UtilityBill>> {
        transition(&self.unit_of_work, &self.clock, context, id, |bill, now| {
            bill.void(now)
        })
        .await
    }
}
