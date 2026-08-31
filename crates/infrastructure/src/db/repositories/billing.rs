use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::billing::{
    Charge, ChargeBalance, ChargeParts, Consumption, ConsumptionUnit, InvoiceNumber, SupplierName,
    SupplierVat, UtilityBill, UtilityBillParts,
};
use propflow_domain::ports::{
    ChargeBalancePage, ChargeQuery, ChargeRepository, RepositoryError, RepositoryResult,
    UtilityBillPage, UtilityBillQuery, UtilityBillRepository,
};
use propflow_domain::shared::{
    ChargeId, Currency, LeaseId, Money, OrgId, PropertyId, RoomId, UserId, UtilityBillId,
};

use super::user::try_get;
use crate::db::db_enums;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

/// I tre stati dell'insoluto, **come letterale SQL**.
///
/// Stessa ragione di `working_statuses!` sulle segnalazioni, e non è una
/// duplicazione da eliminare legando la query a `ChargeStatus::OUTSTANDING` con un
/// parametro: l'indice `charges_outstanding_ix` è parziale su esattamente questo
/// elenco, e il planner può usare un indice parziale solo se dimostra che il
/// predicato della query implica quello dell'indice — cosa che con un array
/// **passato come parametro** non può fare, perché al momento del piano non ne
/// conosce il contenuto.
///
/// L'elenco contiene `overdue`, che l'applicazione non scrive mai: l'indice copre
/// più di quanto serva, resta corretto, e va saputo leggendolo.
///
/// Il prezzo è che l'elenco esiste in due posti; il test
/// `the_sql_literal_and_the_domain_constant_agree` li tiene insieme.
macro_rules! outstanding_statuses {
    () => {
        "'issued','partially_paid','overdue'"
    };
}

/// Colonne lette di una bolletta.
///
/// **Non ci sono `ai_job_id`, `ocr_confidence`, `ocr_extracted`, `document_id` né
/// `metadata`.** L'OCR è fuori dal perimetro di questa fetta
/// (`docs/09-stato-e-roadmap.md`), il dominio non lo modella, e leggere una colonna
/// che nessuno sa interpretare significa doverla mappare su un tipo inventato per
/// l'occasione. `document_id` resta fuori anche in scrittura: la scansione si
/// allega con il riferimento polimorfico dei documenti, e popolare entrambi
/// darebbe due risposte alla domanda «qual è il documento di questa bolletta».
macro_rules! select_bill {
    () => {
        "
    SELECT id,
           org_id,
           property_id,
           room_id,
           lease_id,
           utility_type::text   AS utility_type,
           status::text         AS status,
           supplier_name,
           supplier_vat,
           invoice_number,
           issue_date,
           due_date,
           period_start,
           period_end,
           total_amount::numeric   AS total_amount,
           taxable_amount::numeric AS taxable_amount,
           vat_amount::numeric     AS vat_amount,
           currency::text       AS currency,
           consumption::numeric AS consumption,
           consumption_unit,
           validated_by,
           validated_at,
           notes,
           created_at,
           updated_at,
           deleted_at
      FROM app.utility_bills
"
    };
}

/// Colonne lette di un addebito, dalla **sola tabella**.
///
/// È ciò che usano le transizioni, che non guardano quanto è stato incassato e non
/// devono pagare il join con la vista dei saldi.
macro_rules! select_charge {
    () => {
        "
    SELECT id,
           org_id,
           lease_id,
           charge_type::text AS charge_type,
           status::text      AS status,
           description,
           period_start,
           period_end,
           amount::numeric   AS amount,
           currency::text    AS currency,
           due_date,
           issued_at,
           utility_bill_id,
           notes,
           created_at,
           updated_at,
           deleted_at
      FROM app.charges
"
    };
}

/// Colonne lette di un addebito **con il saldo**.
///
/// Il saldo viene da `app.v_charge_balances` e non da un calcolo qui: la vista è
/// `security_invoker`, quindi applica la RLS del chiamante, e la definizione di
/// «pagato» resta **una sola** — il giorno in cui il modulo dei pagamenti
/// aggiungerà uno stato all'array `('succeeded','partially_refunded')` cambierà in
/// un posto e non in due.
///
/// Il `JOIN` attraversa `app.charges` una seconda volta perché la vista non porta
/// `description`, `period_*`, `issued_at`, `utility_bill_id` e `notes`. È il prezzo
/// dichiarato di non duplicare la `LEFT JOIN LATERAL`, e lo pagano solo le letture
/// che il saldo lo vogliono davvero.
///
/// **In questa fetta `paid_amount` è sempre `0.00`**: non ci sono righe in
/// `app.payments`. Non è un difetto della vista.
macro_rules! select_charge_balance {
    () => {
        "
    SELECT c.id,
           c.org_id,
           c.lease_id,
           c.charge_type::text AS charge_type,
           c.status::text      AS status,
           c.description,
           c.period_start,
           c.period_end,
           c.amount::numeric   AS amount,
           c.currency::text    AS currency,
           c.due_date,
           c.issued_at,
           c.utility_bill_id,
           c.notes,
           c.created_at,
           c.updated_at,
           c.deleted_at,
           b.paid_amount::numeric        AS paid_amount,
           b.outstanding_amount::numeric AS outstanding_amount,
           b.is_past_due
      FROM app.charges c
      JOIN app.v_charge_balances b ON b.charge_id = c.id
"
    };
}

#[async_trait]
impl UtilityBillRepository for PgUnitOfWork {
    async fn find_by_id(&mut self, id: UtilityBillId) -> RepositoryResult<Option<UtilityBill>> {
        let row = sqlx::query(concat!(
            select_bill!(),
            " WHERE id = $1 AND deleted_at IS NULL"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(map_bill).transpose()
    }

    async fn list(&mut self, query: &UtilityBillQuery) -> RepositoryResult<UtilityBillPage> {
        // Il cursore keyset sfrutta l'ordinamento degli UUIDv7: ordinare per id
        // equivale a ordinare per data di creazione, senza una colonna in più. Si
        // chiede una riga in più del limite per sapere se esiste una pagina
        // successiva senza un secondo `COUNT`.
        let fetch = i64::from(query.limit) + 1;
        let rows = sqlx::query(concat!(
            select_bill!(),
            " WHERE deleted_at IS NULL
                AND ($1::uuid IS NULL OR property_id = $1)
                AND ($2::uuid IS NULL OR lease_id = $2)
                AND ($3::text IS NULL OR status = $3::app.bill_status)
                AND ($4::text IS NULL OR utility_type = $4::app.utility_type)
                AND ($5::uuid IS NULL OR id > $5)
              ORDER BY id
              LIMIT $6"
        ))
        .bind(query.property_id.map(PropertyId::as_uuid))
        .bind(query.lease_id.map(LeaseId::as_uuid))
        .bind(query.status.map(db_enums::bill_status_to_db))
        .bind(query.utility_type.map(db_enums::utility_type_to_db))
        .bind(query.after.map(UtilityBillId::as_uuid))
        .bind(fetch)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        let has_more = rows.len() > usize::from(query.limit);
        let items = rows
            .into_iter()
            .take(usize::from(query.limit))
            .map(map_bill)
            .collect::<RepositoryResult<Vec<UtilityBill>>>()?;
        let next_cursor = if has_more {
            items.last().map(UtilityBill::id)
        } else {
            None
        };

        Ok(UtilityBillPage { items, next_cursor })
    }

    async fn insert(&mut self, bill: &UtilityBill) -> RepositoryResult<()> {
        // `currency` non compare: resta al default `EUR`, e non nominarla è il modo
        // di non dichiarare un valore che le viste dei saldi non saprebbero reggere
        // — sommano `amount` e `paid_amount` senza confrontare la valuta.
        //
        // Nessun `RETURNING`: gli identificatori nascono nel dominio con
        // `generate()` (UUIDv7) e nessuna colonna è assegnata dal database. Il
        // `DEFAULT gen_random_uuid()` resta sulla tabella per le righe che nessuno
        // di noi scrive, e sarebbe un UUIDv4 — che manderebbe il cursore keyset in
        // ordine casuale.
        sqlx::query(
            "INSERT INTO app.utility_bills (
                 id, org_id, property_id, room_id, lease_id,
                 utility_type, status,
                 supplier_name, supplier_vat, invoice_number,
                 issue_date, due_date, period_start, period_end,
                 total_amount, taxable_amount, vat_amount,
                 consumption, consumption_unit,
                 validated_by, validated_at,
                 notes, created_at, updated_at
             ) VALUES (
                 $1, $2, $3, $4, $5,
                 $6::app.utility_type, $7::app.bill_status,
                 $8, $9, $10,
                 $11, $12, $13, $14,
                 $15::app.money_amount, $16::app.money_amount, $17::app.money_amount,
                 $18, $19,
                 $20, $21,
                 $22, $23, $24
             )",
        )
        .bind(bill.id().as_uuid())
        .bind(bill.org_id().as_uuid())
        .bind(bill.property_id().as_uuid())
        .bind(bill.room_id().map(RoomId::as_uuid))
        .bind(bill.lease_id().map(LeaseId::as_uuid))
        .bind(db_enums::utility_type_to_db(bill.utility_type()))
        .bind(db_enums::bill_status_to_db(bill.status()))
        .bind(bill.supplier_name().map(SupplierName::as_str))
        .bind(bill.supplier_vat().map(SupplierVat::as_str))
        .bind(bill.invoice_number().map(InvoiceNumber::as_str))
        .bind(bill.issue_date())
        .bind(bill.due_date())
        .bind(bill.period_start())
        .bind(bill.period_end())
        .bind(bill.total_amount().amount())
        .bind(bill.taxable_amount().map(|money| money.amount()))
        .bind(bill.vat_amount().map(|money| money.amount()))
        .bind(bill.consumption().map(Consumption::value))
        .bind(
            bill.consumption()
                .map(|consumption| consumption.unit().as_str()),
        )
        .bind(bill.validated_by().map(UserId::as_uuid))
        .bind(bill.validated_at())
        .bind(bill.notes())
        .bind(bill.created_at())
        .bind(bill.updated_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn update(&mut self, bill: &UtilityBill) -> RepositoryResult<()> {
        // **Nessuna `GRANT` di colonna limita questo elenco**, a differenza degli
        // addebiti: una bolletta si digita, e finché non è validata correggerne un
        // campo è il lavoro previsto. L'immutabilità da `allocated` in poi è
        // un'invariante di dominio (`UtilityBill::apply`), non un privilegio — vedi
        // il commento della migration `0023`.
        //
        // `deleted_at` **è** fra le colonne scritte: annullare una bolletta la
        // cancella, e le due cose viaggiano insieme perché
        // `utility_bills_dedup_uk` è parziale su `deleted_at IS NULL` — senza il
        // soft delete la stessa fattura non sarebbe reinseribile mai più.
        //
        // `property_id` non c'è: è l'ancoraggio strutturale della riga, e una
        // bolletta inserita sull'immobile sbagliato si annulla e si reinserisce.
        // `currency` non c'è per la ragione detta nell'`insert`.
        //
        // `updated_at` non compare perché la scrive il trigger `set_updated_at`.
        //
        // Il `WHERE` **non** filtra `deleted_at IS NULL`: dovrebbe, per la regola
        // generale, ma qui la stessa `UPDATE` che annulla la bolletta valorizza
        // quella colonna, e il filtro escluderebbe la riga solo negli aggiornamenti
        // *successivi* — che il dominio già rifiuta, perché da `void` non parte
        // nessuna transizione.
        let result = sqlx::query(
            "UPDATE app.utility_bills
                SET room_id          = $2,
                    lease_id         = $3,
                    utility_type     = $4::app.utility_type,
                    status           = $5::app.bill_status,
                    supplier_name    = $6,
                    supplier_vat     = $7,
                    invoice_number   = $8,
                    issue_date       = $9,
                    due_date         = $10,
                    period_start     = $11,
                    period_end       = $12,
                    total_amount     = $13::app.money_amount,
                    taxable_amount   = $14::app.money_amount,
                    vat_amount       = $15::app.money_amount,
                    consumption      = $16,
                    consumption_unit = $17,
                    validated_by     = $18,
                    validated_at     = $19,
                    notes            = $20,
                    deleted_at       = $21
              WHERE id = $1",
        )
        .bind(bill.id().as_uuid())
        .bind(bill.room_id().map(RoomId::as_uuid))
        .bind(bill.lease_id().map(LeaseId::as_uuid))
        .bind(db_enums::utility_type_to_db(bill.utility_type()))
        .bind(db_enums::bill_status_to_db(bill.status()))
        .bind(bill.supplier_name().map(SupplierName::as_str))
        .bind(bill.supplier_vat().map(SupplierVat::as_str))
        .bind(bill.invoice_number().map(InvoiceNumber::as_str))
        .bind(bill.issue_date())
        .bind(bill.due_date())
        .bind(bill.period_start())
        .bind(bill.period_end())
        .bind(bill.total_amount().amount())
        .bind(bill.taxable_amount().map(|money| money.amount()))
        .bind(bill.vat_amount().map(|money| money.amount()))
        .bind(bill.consumption().map(Consumption::value))
        .bind(
            bill.consumption()
                .map(|consumption| consumption.unit().as_str()),
        )
        .bind(bill.validated_by().map(UserId::as_uuid))
        .bind(bill.validated_at())
        .bind(bill.notes())
        .bind(bill.deleted_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        // Zero righe con RLS attiva significa «non aggiornabile da questo
        // contesto»: la `USING` di `utility_bills_update` chiede staff. Il service
        // layer verifica lo stesso criterio prima di arrivare qui, quindi se scatta
        // è un difetto nostro.
        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }
}

#[async_trait]
impl ChargeRepository for PgUnitOfWork {
    async fn find_by_id(&mut self, id: ChargeId) -> RepositoryResult<Option<Charge>> {
        let row = sqlx::query(concat!(
            select_charge!(),
            " WHERE id = $1 AND deleted_at IS NULL"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(|row| map_charge(&row)).transpose()
    }

    async fn find_balance_by_id(
        &mut self,
        id: ChargeId,
    ) -> RepositoryResult<Option<ChargeBalance>> {
        let row = sqlx::query(concat!(
            select_charge_balance!(),
            " WHERE c.id = $1 AND c.deleted_at IS NULL"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(map_charge_balance).transpose()
    }

    async fn list_balances(&mut self, query: &ChargeQuery) -> RepositoryResult<ChargeBalancePage> {
        let fetch = i64::from(query.limit) + 1;
        let rows = sqlx::query(concat!(
            select_charge_balance!(),
            " WHERE c.deleted_at IS NULL
                AND ($1::uuid IS NULL OR c.lease_id = $1)
                AND ($2::text IS NULL OR c.charge_type = $2::app.charge_type)
                AND ($3::text IS NULL OR c.status = $3::app.charge_status)
                AND ($4::bool IS NULL
                     OR (c.status = ANY (ARRAY[",
            outstanding_statuses!(),
            "]::app.charge_status[])) = $4)
                AND ($5::uuid IS NULL OR c.id > $5)
              ORDER BY c.id
              LIMIT $6"
        ))
        .bind(query.lease_id.map(LeaseId::as_uuid))
        .bind(query.charge_type.map(db_enums::charge_type_to_db))
        .bind(query.status.map(db_enums::charge_status_to_db))
        .bind(query.outstanding)
        .bind(query.after.map(ChargeId::as_uuid))
        .bind(fetch)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        let has_more = rows.len() > usize::from(query.limit);
        let items = rows
            .into_iter()
            .take(usize::from(query.limit))
            .map(map_charge_balance)
            .collect::<RepositoryResult<Vec<ChargeBalance>>>()?;
        let next_cursor = if has_more {
            items.last().map(|balance| balance.charge.id())
        } else {
            None
        };

        Ok(ChargeBalancePage { items, next_cursor })
    }

    async fn insert(&mut self, charge: &Charge) -> RepositoryResult<()> {
        // `generation_key` e `document_id` restano nulle: la generazione ricorrente
        // dei canoni richiede uno scheduler (ADR-019) e gli allegati passano dal
        // riferimento polimorfico dei documenti. `currency` resta al default.
        //
        // **Qui un `INSERT … RETURNING` funzionerebbe**, a differenza delle
        // segnalazioni, e sapere perché conta più del fatto: `tickets_select` chiama
        // `can_access_ticket(id)`, che interroga *la riga che si sta inserendo*, e
        // la funzione è `STABLE` — vede lo snapshot di inizio statement, in cui la
        // riga non c'è. `charges_select` chiama invece `can_access_lease(lease_id)`,
        // che interroga `app.leases`, una tabella che questa `INSERT` non tocca. La
        // regola generale: **il muro c'è solo quando la policy di `SELECT` dipende
        // dalla riga stessa.** Non ce ne serviamo comunque, perché nessuna colonna
        // è assegnata dal database.
        sqlx::query(
            "INSERT INTO app.charges (
                 id, org_id, lease_id, charge_type, status,
                 description, period_start, period_end,
                 amount, due_date, issued_at, utility_bill_id,
                 notes, created_at, updated_at
             ) VALUES (
                 $1, $2, $3, $4::app.charge_type, $5::app.charge_status,
                 $6, $7, $8,
                 $9::app.money_amount, $10, $11, $12,
                 $13, $14, $15
             )",
        )
        .bind(charge.id().as_uuid())
        .bind(charge.org_id().as_uuid())
        .bind(charge.lease_id().as_uuid())
        .bind(db_enums::charge_type_to_db(charge.charge_type()))
        .bind(db_enums::charge_status_to_db(charge.status()))
        .bind(charge.description())
        .bind(charge.period_start())
        .bind(charge.period_end())
        .bind(charge.amount().amount())
        .bind(charge.due_date())
        .bind(charge.issued_at())
        .bind(charge.utility_bill_id().map(UtilityBillId::as_uuid))
        .bind(charge.notes())
        .bind(charge.created_at())
        .bind(charge.updated_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn update_status(&mut self, charge: &Charge) -> RepositoryResult<()> {
        // **Una colonna sola, e non è una scelta di stile.** La migration `0023` ha
        // revocato l'`UPDATE` su `app.charges` e lo riconcede sulla sola `status`:
        // nominare qui `amount`, `due_date` o qualunque altra colonna farebbe
        // fallire la query con `permission denied for table charges` — nei test di
        // integrazione, che girano come `propflow_app`.
        //
        // È la difesa che rende impossibile riscrivere una pretesa già comunicata a
        // una persona. Un addebito sbagliato si annulla e si riemette.
        //
        // `updated_at` non compare perché la scrive il trigger `set_updated_at`.
        let result = sqlx::query(
            "UPDATE app.charges
                SET status = $2::app.charge_status
              WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(charge.id().as_uuid())
        .bind(db_enums::charge_status_to_db(charge.status()))
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }
}

fn invalid(field: &str, error: propflow_domain::DomainError) -> RepositoryError {
    RepositoryError::Backend {
        details: format!("{field} non valido in database: {error}"),
    }
}

fn map_bill(row: PgRow) -> RepositoryResult<UtilityBill> {
    let id: Uuid = try_get(&row, "id")?;
    let org_id: Uuid = try_get(&row, "org_id")?;
    let property_id: Uuid = try_get(&row, "property_id")?;
    let room_id: Option<Uuid> = try_get(&row, "room_id")?;
    let lease_id: Option<Uuid> = try_get(&row, "lease_id")?;
    let utility_type: String = try_get(&row, "utility_type")?;
    let status: String = try_get(&row, "status")?;
    let currency_code: String = try_get(&row, "currency")?;
    let total_amount: Decimal = try_get(&row, "total_amount")?;
    let taxable_amount: Option<Decimal> = try_get(&row, "taxable_amount")?;
    let vat_amount: Option<Decimal> = try_get(&row, "vat_amount")?;
    let consumption: Option<Decimal> = try_get(&row, "consumption")?;
    let consumption_unit: Option<String> = try_get(&row, "consumption_unit")?;
    let validated_by: Option<Uuid> = try_get(&row, "validated_by")?;
    let supplier_name: Option<String> = try_get(&row, "supplier_name")?;
    let supplier_vat: Option<String> = try_get(&row, "supplier_vat")?;
    let invoice_number: Option<String> = try_get(&row, "invoice_number")?;

    let currency = Currency::parse(&currency_code).map_err(|error| invalid("currency", error))?;
    let money = |field: &'static str, amount: Option<Decimal>| {
        amount
            .map(|value| Money::new(value, currency))
            .transpose()
            .map_err(|error| invalid(field, error))
    };

    // Consumo e unità sono due colonne indipendenti a schema e un value object
    // solo nel dominio: una riga con il numero e senza l'unità — che lo schema
    // accetta — si legge come «nessun consumo», perché un numero senza unità non
    // dice niente. Scartare in silenzio è corretto qui e non altrove: non è un
    // valore sconosciuto di un enum, è un dato incompleto scritto da qualcun altro.
    let consumption = match (consumption, consumption_unit) {
        (Some(value), Some(unit)) => Some(Consumption::from_persistence(
            value,
            ConsumptionUnit::from_persistence(unit),
        )),
        _ => None,
    };

    Ok(UtilityBill::from_persistence(UtilityBillParts {
        id: UtilityBillId::from_uuid(id),
        org_id: OrgId::from_uuid(org_id),
        property_id: PropertyId::from_uuid(property_id),
        room_id: room_id.map(RoomId::from_uuid),
        lease_id: lease_id.map(LeaseId::from_uuid),
        utility_type: db_enums::utility_type_from_db(&utility_type)?,
        status: db_enums::bill_status_from_db(&status)?,
        supplier_name: supplier_name.map(SupplierName::from_persistence),
        supplier_vat: supplier_vat.map(SupplierVat::from_persistence),
        invoice_number: invoice_number.map(InvoiceNumber::from_persistence),
        issue_date: try_get::<Option<NaiveDate>>(&row, "issue_date")?,
        due_date: try_get::<Option<NaiveDate>>(&row, "due_date")?,
        period_start: try_get::<Option<NaiveDate>>(&row, "period_start")?,
        period_end: try_get::<Option<NaiveDate>>(&row, "period_end")?,
        total_amount: Money::new(total_amount, currency)
            .map_err(|error| invalid("total_amount", error))?,
        taxable_amount: money("taxable_amount", taxable_amount)?,
        vat_amount: money("vat_amount", vat_amount)?,
        currency,
        consumption,
        validated_by: validated_by.map(UserId::from_uuid),
        validated_at: try_get::<Option<DateTime<Utc>>>(&row, "validated_at")?,
        notes: try_get(&row, "notes")?,
        created_at: try_get(&row, "created_at")?,
        updated_at: try_get(&row, "updated_at")?,
        deleted_at: try_get(&row, "deleted_at")?,
    }))
}

fn map_charge(row: &PgRow) -> RepositoryResult<Charge> {
    let id: Uuid = try_get(row, "id")?;
    let org_id: Uuid = try_get(row, "org_id")?;
    let lease_id: Uuid = try_get(row, "lease_id")?;
    let charge_type: String = try_get(row, "charge_type")?;
    let status: String = try_get(row, "status")?;
    let currency_code: String = try_get(row, "currency")?;
    let amount: Decimal = try_get(row, "amount")?;
    let utility_bill_id: Option<Uuid> = try_get(row, "utility_bill_id")?;

    let currency = Currency::parse(&currency_code).map_err(|error| invalid("currency", error))?;

    Ok(Charge::from_persistence(ChargeParts {
        id: ChargeId::from_uuid(id),
        org_id: OrgId::from_uuid(org_id),
        lease_id: LeaseId::from_uuid(lease_id),
        charge_type: db_enums::charge_type_from_db(&charge_type)?,
        status: db_enums::charge_status_from_db(&status)?,
        description: try_get(row, "description")?,
        period_start: try_get::<Option<NaiveDate>>(row, "period_start")?,
        period_end: try_get::<Option<NaiveDate>>(row, "period_end")?,
        amount: Money::new(amount, currency).map_err(|error| invalid("amount", error))?,
        currency,
        due_date: try_get(row, "due_date")?,
        issued_at: try_get::<Option<DateTime<Utc>>>(row, "issued_at")?,
        utility_bill_id: utility_bill_id.map(UtilityBillId::from_uuid),
        notes: try_get(row, "notes")?,
        created_at: try_get(row, "created_at")?,
        updated_at: try_get(row, "updated_at")?,
        deleted_at: try_get(row, "deleted_at")?,
    }))
}

fn map_charge_balance(row: PgRow) -> RepositoryResult<ChargeBalance> {
    let charge = map_charge(&row)?;
    let currency = charge.currency();
    let paid_amount: Decimal = try_get(&row, "paid_amount")?;
    let outstanding_amount: Decimal = try_get(&row, "outstanding_amount")?;

    Ok(ChargeBalance {
        paid_amount: Money::new(paid_amount, currency)
            .map_err(|error| invalid("paid_amount", error))?,
        outstanding_amount: Money::new(outstanding_amount, currency)
            .map_err(|error| invalid("outstanding_amount", error))?,
        is_past_due: try_get(&row, "is_past_due")?,
        charge,
    })
}

#[cfg(test)]
mod tests {
    use propflow_domain::billing::ChargeStatus;

    use crate::db::db_enums;

    /// L'elenco dei tre stati vive in due posti — la costante di dominio e il
    /// letterale SQL che rende utilizzabile `charges_outstanding_ix` — e questo
    /// test è ciò che li tiene insieme. Senza, aggiungere uno stato dell'insoluto
    /// al dominio farebbe smettere lo scadenzario di comprenderlo, in silenzio.
    #[test]
    fn the_sql_literal_and_the_domain_constant_agree() {
        let from_domain = ChargeStatus::OUTSTANDING
            .iter()
            .map(|status| format!("'{}'", db_enums::charge_status_to_db(*status)))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(from_domain, outstanding_statuses!());
    }
}
