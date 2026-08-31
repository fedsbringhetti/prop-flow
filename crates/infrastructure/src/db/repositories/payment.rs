use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::payment::{IdempotencyKey, Payable, Payment, PaymentParts};
use propflow_domain::ports::{
    PaymentPage, PaymentQuery, PaymentRepository, RepositoryError, RepositoryResult,
};
use propflow_domain::shared::{ChargeId, Currency, Money, OrgId, PaymentId, UserId};

use super::user::try_get;
use crate::db::db_enums;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

/// I due stati che **contano nel saldo**, come letterale SQL.
///
/// Stessa ragione di `outstanding_statuses!` sugli addebiti e di
/// `working_statuses!` sulle segnalazioni: un array passato come parametro non
/// permetterebbe al planner di dimostrare che il predicato della query implica
/// quello dell'indice parziale.
///
/// **L'elenco è quello della vista, non quello dell'indice**, ed è la trappola più
/// cara di questo repository. `payments_payable_ix` è parziale su
/// `('authorized','succeeded','partially_refunded')`, mentre
/// `app.v_charge_balances` conta `('succeeded','partially_refunded')`. Copiare
/// l'elenco dall'indice — che è ciò che viene naturale, perché è lì che si guarda
/// per capire quale indice servirà la query — conterebbe anche gli `authorized`,
/// cioè pagamenti autorizzati e **non incassati**, facendo scendere il saldo su
/// denaro che non è arrivato. Il predicato più stretto resta comunque servito
/// dall'indice, quindi avere ragione non costa niente in prestazioni.
///
/// Il prezzo è che l'elenco esiste in due posti; il test
/// `the_sql_literal_and_the_domain_constant_agree` li tiene insieme.
macro_rules! balance_statuses {
    () => {
        "'succeeded','partially_refunded'"
    };
}

/// Colonne lette di un pagamento.
///
/// **Non c'è `deleted_at`**, e non è un'omissione: `app.payments` non ha quella
/// colonna. Nessuna query di questo file la filtra, ed è la prima cosa da sapere
/// prima di copiarne una da un altro repository.
///
/// **Non ci sono `provider`, `provider_payment_id`, `provider_payload`,
/// `failure_code`, `failure_message`, `payee_user_id` né `metadata`.** I provider
/// veri sono fuori dal perimetro della fetta (`docs/09-stato-e-roadmap.md`), il
/// dominio non li modella, e leggere una colonna che nessuno sa interpretare
/// significa doverla mappare su un tipo inventato per l'occasione.
macro_rules! select_payment {
    () => {
        "
    SELECT id,
           org_id,
           payable_type::text AS payable_type,
           payable_id,
           payer_user_id,
           amount::numeric    AS amount,
           currency::text     AS currency,
           method::text       AS method,
           status::text       AS status,
           idempotency_key,
           reference,
           paid_at,
           reconciled_at,
           refunded_amount::numeric AS refunded_amount,
           created_at,
           updated_at
      FROM app.payments
"
    };
}

#[async_trait]
impl PaymentRepository for PgUnitOfWork {
    async fn find_by_id(&mut self, id: PaymentId) -> RepositoryResult<Option<Payment>> {
        let row = sqlx::query(concat!(select_payment!(), " WHERE id = $1"))
            .bind(id.as_uuid())
            .fetch_optional(&mut *self.tx)
            .await
            .map_err(map_sqlx_error)?;
        row.as_ref().map(map_payment).transpose()
    }

    async fn find_by_idempotency_key(
        &mut self,
        key: &IdempotencyKey,
    ) -> RepositoryResult<Option<Payment>> {
        // `org_id` non compare nel `WHERE` e l'indice unico è su
        // `(org_id, idempotency_key)`: a restringere all'organizzazione è la RLS,
        // che `payments_select` applica con `org_id = app.current_org_id()`.
        // Ripeterlo qui non aggiungerebbe sicurezza — la policy non è aggirabile —
        // e darebbe l'impressione che la query sia sicura per conto proprio.
        let row = sqlx::query(concat!(select_payment!(), " WHERE idempotency_key = $1"))
            .bind(key.as_str())
            .fetch_optional(&mut *self.tx)
            .await
            .map_err(map_sqlx_error)?;
        row.as_ref().map(map_payment).transpose()
    }

    async fn list_for_charge(&mut self, query: &PaymentQuery) -> RepositoryResult<PaymentPage> {
        // Il cursore keyset sfrutta l'ordinamento degli UUIDv7: ordinare per id
        // equivale a ordinare per data di creazione, senza una colonna in più. Si
        // chiede una riga in più del limite per sapere se esiste una pagina
        // successiva senza un secondo `COUNT`.
        let fetch = i64::from(query.limit) + 1;
        let rows = sqlx::query(concat!(
            select_payment!(),
            " WHERE payable_type = 'charge'::app.payable_type
                AND payable_id = $1
                AND ($2::uuid IS NULL OR id > $2)
              ORDER BY id
              LIMIT $3"
        ))
        .bind(query.charge_id.as_uuid())
        .bind(query.after.map(PaymentId::as_uuid))
        .bind(fetch)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        let has_more = rows.len() > usize::from(query.limit);
        let items = rows
            .iter()
            .take(usize::from(query.limit))
            .map(map_payment)
            .collect::<RepositoryResult<Vec<Payment>>>()?;
        let next_cursor = if has_more {
            items.last().map(Payment::id)
        } else {
            None
        };

        Ok(PaymentPage { items, next_cursor })
    }

    async fn sum_succeeded_for_charge(&mut self, charge_id: ChargeId) -> RepositoryResult<Decimal> {
        // `amount - refunded_amount`, esattamente come la `LEFT JOIN LATERAL` di
        // `app.v_charge_balances`: è la **stessa** definizione di «pagato», e
        // divergerne farebbe dire allo stato memorizzato una cosa e al saldo
        // mostrato accanto un'altra. `refunded_amount` vale sempre zero in questa
        // fetta, e sottrarlo comunque è ciò che rende la formula la stessa il
        // giorno in cui non varrà più zero.
        //
        // `COALESCE` perché `SUM` di zero righe è `NULL`, non `0`.
        let total: Decimal = sqlx::query_scalar(concat!(
            "SELECT COALESCE(SUM(amount - refunded_amount), 0)::numeric
               FROM app.payments
              WHERE payable_type = 'charge'::app.payable_type
                AND payable_id = $1
                AND status = ANY (ARRAY[",
            balance_statuses!(),
            "]::app.payment_status[])"
        ))
        .bind(charge_id.as_uuid())
        .fetch_one(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(total)
    }

    async fn insert(&mut self, payment: &Payment) -> RepositoryResult<()> {
        // `provider`, `provider_payment_id`, `provider_payload`, `payee_user_id`,
        // `failure_code`, `failure_message`, `refunded_amount` e `metadata` restano
        // ai loro default: i provider veri sono fuori perimetro, e con `manual` un
        // fallimento presso il provider non ha origine.
        //
        // Nessun `RETURNING`: nessuna colonna è assegnata dal database, e
        // `payments_select` dipende dalla riga che si sta inserendo
        // (`payer_user_id = current_user_id()`), quindi varrebbe il muro che si è
        // visto sulle segnalazioni — la policy di `SELECT` si applica anche alla
        // riga nuova di un `INSERT … RETURNING`.
        sqlx::query(
            "INSERT INTO app.payments (
                 id, org_id, payable_type, payable_id, payer_user_id,
                 amount, method, status, idempotency_key,
                 reference, paid_at, reconciled_at,
                 created_at, updated_at
             ) VALUES (
                 $1, $2, $3::app.payable_type, $4, $5,
                 $6::app.money_amount, $7::app.payment_method, $8::app.payment_status, $9,
                 $10, $11, $12,
                 $13, $14
             )",
        )
        .bind(payment.id().as_uuid())
        .bind(payment.org_id().as_uuid())
        .bind(db_enums::payable_type_to_db(payment.payable()))
        .bind(payable_id(payment.payable()))
        .bind(payment.payer_user_id().map(UserId::as_uuid))
        .bind(payment.amount().amount())
        .bind(db_enums::payment_method_to_db(payment.method()))
        .bind(db_enums::payment_status_to_db(payment.status()))
        .bind(payment.idempotency_key().as_str())
        .bind(payment.reference())
        .bind(payment.paid_at())
        .bind(payment.reconciled_at())
        .bind(payment.created_at())
        .bind(payment.updated_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn update_status(&mut self, payment: &Payment) -> RepositoryResult<()> {
        // **Tre colonne, e non è una scelta di stile.** La migration `0025` ha
        // revocato l'`UPDATE` su `app.payments` e lo riconcede su `status`,
        // `paid_at` e `reconciled_at`: nominare qui `amount`, `payer_user_id` o
        // `idempotency_key` farebbe fallire la query con
        // `permission denied for table payments` — nei test di integrazione, che
        // girano come `propflow_app`.
        //
        // `paid_at` è fra le tre proprio perché il rifiuto la **azzera**:
        // `payments_succeeded_ck` obbliga ad averla quando lo stato è `succeeded` e
        // non obbliga a toglierla muovendosi altrove, quindi l'azzeramento tocca al
        // dominio e la colonna deve restare scrivibile.
        //
        // `updated_at` non compare perché la scrive il trigger `set_updated_at`.
        let result = sqlx::query(
            "UPDATE app.payments
                SET status = $2::app.payment_status,
                    paid_at = $3,
                    reconciled_at = $4
              WHERE id = $1",
        )
        .bind(payment.id().as_uuid())
        .bind(db_enums::payment_status_to_db(payment.status()))
        .bind(payment.paid_at())
        .bind(payment.reconciled_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }
}

const fn payable_id(payable: Payable) -> Uuid {
    match payable {
        Payable::Charge(id) => id.as_uuid(),
    }
}

fn invalid(field: &str, error: propflow_domain::DomainError) -> RepositoryError {
    RepositoryError::Backend {
        details: format!("{field} non valido in database: {error}"),
    }
}

fn map_payment(row: &PgRow) -> RepositoryResult<Payment> {
    let id: Uuid = try_get(row, "id")?;
    let org_id: Uuid = try_get(row, "org_id")?;
    let payable_type: String = try_get(row, "payable_type")?;
    let payable_id: Uuid = try_get(row, "payable_id")?;
    let payer_user_id: Option<Uuid> = try_get(row, "payer_user_id")?;
    let currency_code: String = try_get(row, "currency")?;
    let amount: Decimal = try_get(row, "amount")?;
    let refunded_amount: Decimal = try_get(row, "refunded_amount")?;
    let method: String = try_get(row, "method")?;
    let status: String = try_get(row, "status")?;
    let idempotency_key: String = try_get(row, "idempotency_key")?;

    let currency = Currency::parse(&currency_code).map_err(|error| invalid("currency", error))?;

    Ok(Payment::from_persistence(PaymentParts {
        id: PaymentId::from_uuid(id),
        org_id: OrgId::from_uuid(org_id),
        payable: db_enums::payable_from_db(&payable_type, ChargeId::from_uuid(payable_id))?,
        payer_user_id: payer_user_id.map(UserId::from_uuid),
        amount: Money::new(amount, currency).map_err(|error| invalid("amount", error))?,
        currency,
        method: db_enums::payment_method_from_db(&method)?,
        status: db_enums::payment_status_from_db(&status)?,
        idempotency_key: IdempotencyKey::from_persistence(idempotency_key),
        reference: try_get(row, "reference")?,
        paid_at: try_get::<Option<DateTime<Utc>>>(row, "paid_at")?,
        reconciled_at: try_get::<Option<DateTime<Utc>>>(row, "reconciled_at")?,
        refunded_amount: Money::new(refunded_amount, currency)
            .map_err(|error| invalid("refunded_amount", error))?,
        created_at: try_get(row, "created_at")?,
        updated_at: try_get(row, "updated_at")?,
    }))
}

#[cfg(test)]
mod tests {
    use propflow_domain::payment::PaymentStatus;

    use crate::db::db_enums;

    /// L'elenco dei due stati vive in due posti — la costante di dominio e il
    /// letterale SQL — e questo test è ciò che li tiene insieme.
    ///
    /// Conta più del suo omologo sugli addebiti: qui l'elenco sbagliato non
    /// rallenta una query, fa **scendere un saldo** su denaro che non è arrivato.
    #[test]
    fn the_sql_literal_and_the_domain_constant_agree() {
        let from_domain = PaymentStatus::COUNTS_TOWARDS_BALANCE
            .iter()
            .map(|status| format!("'{}'", db_enums::payment_status_to_db(*status)))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(from_domain, balance_statuses!());
    }

    /// E l'elenco **non** è quello di `payments_payable_ix`, che comprende anche
    /// `authorized`. Il test lo fissa perché la somiglianza è l'intera trappola.
    #[test]
    fn the_literal_is_not_the_one_of_the_partial_index() {
        assert!(
            !balance_statuses!().contains("authorized"),
            "contare gli `authorized` farebbe scendere il saldo su denaro autorizzato e non incassato"
        );
    }
}
