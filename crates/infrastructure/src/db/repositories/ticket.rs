use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use propflow_domain::ports::{
    RepositoryError, RepositoryResult, TicketMessagePage, TicketMessageQuery,
    TicketMessageRepository, TicketPage, TicketQuery, TicketRepository,
};
use propflow_domain::shared::{
    Currency, LeaseId, Money, OrgId, PropertyId, RoomId, TicketId, TicketMessageId, UserId,
};
use propflow_domain::ticket::{
    MessageBody, NewTicketMessage, Ticket, TicketDescription, TicketMessage, TicketParts,
    TicketReference, TicketStatusChange, TicketTitle,
};

use super::user::try_get;
use crate::db::db_enums;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

/// I sei stati della coda di lavorazione, **come letterale SQL**.
///
/// Non è una duplicazione da eliminare legando la query a `TicketStatus::WORKING`
/// con un parametro: l'indice `tickets_open_ix` è parziale su esattamente questo
/// elenco, e il planner può usare un indice parziale solo se dimostra che il
/// predicato della query implica quello dell'indice — cosa che con un array
/// **passato come parametro** non può fare, perché al momento del piano non ne
/// conosce il contenuto. Il letterale è ciò che rende l'indice utilizzabile.
///
/// Il prezzo è che l'elenco esiste in due posti; il test
/// `the_sql_literal_and_the_domain_constant_agree` li tiene insieme.
macro_rules! working_statuses {
    () => {
        "'open','triaged','assigned','in_progress','waiting_tenant','waiting_owner'"
    };
}

/// Colonne lette.
///
/// **Non ci sono `ai_job_id`, `ai_suggested_category`, `ai_suggested_priority`,
/// `ai_confidence`, `ai_analysis`, `sla_due_at`, `scheduled_at`,
/// `resolution_notes`, `metadata` né `search_vector`.** Non è una dimenticanza:
/// triage AI, SLA, programmazione e ricerca full-text sono fuori dal perimetro di
/// questa fetta (`docs/09-stato-e-roadmap.md`), il dominio non li modella, e
/// leggere una colonna che nessuno sa interpretare significa doverla mappare su un
/// tipo inventato per l'occasione.
///
/// `search_vector` in particolare è `GENERATED ALWAYS`: nominarla in una scrittura
/// fa rifiutare la query da PostgreSQL. Tenerla fuori anche in lettura toglie la
/// tentazione di aggiungerla «già che ci siamo» a un elenco di campi.
macro_rules! select_ticket {
    () => {
        "
    SELECT id,
           org_id,
           property_id,
           room_id,
           lease_id,
           reference,
           title,
           description,
           category::text       AS category,
           priority::text       AS priority,
           status::text         AS status,
           reported_by,
           reported_by_kind::text AS reported_by_kind,
           assigned_to,
           assigned_at,
           estimated_cost::numeric AS estimated_cost,
           actual_cost::numeric    AS actual_cost,
           currency::text       AS currency,
           resolved_at,
           closed_at,
           created_at,
           updated_at,
           deleted_at
      FROM app.tickets
"
    };
}

#[async_trait]
impl TicketRepository for PgUnitOfWork {
    async fn find_by_id(&mut self, id: TicketId) -> RepositoryResult<Option<Ticket>> {
        let row = sqlx::query(concat!(
            select_ticket!(),
            " WHERE id = $1 AND deleted_at IS NULL"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        row.map(map_ticket).transpose()
    }

    async fn list(&mut self, query: &TicketQuery) -> RepositoryResult<TicketPage> {
        // Il cursore keyset sfrutta l'ordinamento degli UUIDv7: ordinare per id
        // equivale a ordinarli per data di creazione, senza una colonna in più. Si
        // chiede una riga in più del limite per sapere se esiste una pagina
        // successiva senza un secondo `COUNT`.
        let fetch = i64::from(query.limit) + 1;
        let rows = sqlx::query(concat!(
            select_ticket!(),
            " WHERE deleted_at IS NULL
                AND ($1::uuid IS NULL OR property_id = $1)
                AND ($2::text IS NULL OR status = $2::app.ticket_status)
                AND ($3::bool IS NULL
                     OR (status = ANY (ARRAY[",
            working_statuses!(),
            "]::app.ticket_status[])) = $3)
                AND ($4::uuid IS NULL OR id > $4)
              ORDER BY id
              LIMIT $5"
        ))
        .bind(query.property_id.map(PropertyId::as_uuid))
        .bind(query.status.map(db_enums::ticket_status_to_db))
        .bind(query.open)
        .bind(query.after.map(TicketId::as_uuid))
        .bind(fetch)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        let has_more = rows.len() > usize::from(query.limit);
        let items = rows
            .into_iter()
            .take(usize::from(query.limit))
            .map(map_ticket)
            .collect::<RepositoryResult<Vec<Ticket>>>()?;
        let next_cursor = if has_more {
            items.last().map(Ticket::id)
        } else {
            None
        };

        Ok(TicketPage { items, next_cursor })
    }

    async fn insert(&mut self, ticket: &Ticket) -> RepositoryResult<TicketReference> {
        // `reference` **non** compare fra le colonne inserite: la riempie il
        // `DEFAULT` della colonna attingendo a `app.ticket_reference_seq`.
        // Specificarla qui metterebbe il codice in competizione con la sequenza,
        // che avanzerebbe comunque al primo insert che non la nomina, producendo
        // collisioni su `tickets_reference_uk` a distanza di mesi.
        //
        // **E non si recupera con `RETURNING`.** Sotto Row Level Security un
        // `INSERT … RETURNING` applica alla riga restituita anche la policy di
        // `SELECT`, e `tickets_select` chiama `app.can_access_ticket(id)`, che è
        // `STABLE`: vede lo snapshot preso all'inizio dello **statement**, cioè
        // prima che la riga esistesse. La funzione risponde `false`, e PostgreSQL
        // rifiuta l'inserimento con
        // `new row violates row-level security policy for table "tickets"` — un
        // messaggio che parla della `WITH CHECK` e manda a cercare nel posto
        // sbagliato. Lo stesso `INSERT` senza `RETURNING` passa.
        //
        // La rilettura in uno **statement separato** della stessa transazione
        // funziona invece: lì `can_access_ticket` prende uno snapshot nuovo, che
        // la riga appena scritta la comprende. Costa un round-trip, ed è l'unica
        // via che la policy lascia aperta.
        //
        // Non compaiono nemmeno `currency` (resta al default `EUR`) né le colonne
        // fuori perimetro: non nominarle è il modo di non dichiarare un valore che
        // questa fetta non sa gestire.
        sqlx::query(
            "INSERT INTO app.tickets (
                 id, org_id, property_id, room_id, lease_id,
                 title, description, category, priority, status,
                 reported_by, reported_by_kind, created_at, updated_at
             ) VALUES (
                 $1, $2, $3, $4, $5,
                 $6, $7, $8::app.ticket_category, $9::app.ticket_priority,
                 $10::app.ticket_status,
                 $11, $12::app.ticket_actor, $13, $14
             )",
        )
        .bind(ticket.id().as_uuid())
        .bind(ticket.org_id().as_uuid())
        .bind(ticket.property_id().as_uuid())
        .bind(ticket.room_id().map(RoomId::as_uuid))
        .bind(ticket.lease_id().map(LeaseId::as_uuid))
        .bind(ticket.title().as_str())
        .bind(ticket.description().as_str())
        .bind(db_enums::ticket_category_to_db(ticket.category()))
        .bind(db_enums::ticket_priority_to_db(ticket.priority()))
        .bind(db_enums::ticket_status_to_db(ticket.status()))
        .bind(ticket.reported_by().map(UserId::as_uuid))
        .bind(db_enums::ticket_actor_to_db(ticket.reported_by_kind()))
        .bind(ticket.created_at())
        .bind(ticket.updated_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        let row = sqlx::query("SELECT reference FROM app.tickets WHERE id = $1")
            .bind(ticket.id().as_uuid())
            .fetch_one(&mut *self.tx)
            .await
            .map_err(map_sqlx_error)?;

        let reference: String = try_get(&row, "reference")?;
        Ok(TicketReference::from_persistence(reference))
    }

    async fn update(&mut self, ticket: &Ticket) -> RepositoryResult<()> {
        // **Questo elenco e la `GRANT UPDATE (…)` della migration `0020` sono la
        // stessa lista.** Aggiungere una colonna qui senza aggiungerla là fa
        // fallire la scrittura con `permission denied`, e fallisce nei test di
        // integrazione, che girano come `propflow_app`.
        //
        // `property_id`, `room_id`, `lease_id`, `title`, `description`,
        // `reported_by` e `reference` non ci sono e non possono esserci: il ruolo
        // applicativo non ha più il privilegio di scriverle. Una segnalazione
        // aperta sull'immobile sbagliato si annulla e si riapre.
        //
        // `updated_at` non compare perché la scrive il trigger `set_updated_at`.
        let result = sqlx::query(
            "UPDATE app.tickets
                SET status         = $2::app.ticket_status,
                    category       = $3::app.ticket_category,
                    priority       = $4::app.ticket_priority,
                    assigned_to    = $5,
                    assigned_at    = $6,
                    estimated_cost = $7::app.money_amount,
                    actual_cost    = $8::app.money_amount,
                    resolved_at    = $9,
                    closed_at      = $10
              WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(ticket.id().as_uuid())
        .bind(db_enums::ticket_status_to_db(ticket.status()))
        .bind(db_enums::ticket_category_to_db(ticket.category()))
        .bind(db_enums::ticket_priority_to_db(ticket.priority()))
        .bind(ticket.assigned_to().map(UserId::as_uuid))
        .bind(ticket.assigned_at())
        .bind(ticket.estimated_cost().map(|money| money.amount()))
        .bind(ticket.actual_cost().map(|money| money.amount()))
        .bind(ticket.resolved_at())
        .bind(ticket.closed_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        // Zero righe con RLS attiva significa «non aggiornabile da questo
        // contesto»: la `USING` di `tickets_update` chiede staff, chi ha segnalato
        // o chi è assegnato. Il service layer verifica lo stesso criterio prima di
        // arrivare qui, quindi se scatta è un difetto nostro.
        if result.rows_affected() == 0 {
            return Err(RepositoryError::RowLevelSecurity);
        }
        Ok(())
    }

    async fn list_status_history(
        &mut self,
        ticket_id: TicketId,
    ) -> RepositoryResult<Vec<TicketStatusChange>> {
        // Nessuna paginazione: lo storico è corto e limitato per costruzione — una
        // riga per transizione — e serve intero al dettaglio della segnalazione.
        // `created_at` e non `id`: la tabella ha `id` UUIDv4 (`gen_random_uuid()`),
        // quindi ordinarla per id darebbe un ordine casuale.
        let rows = sqlx::query(
            "SELECT ticket_id,
                    from_status::text AS from_status,
                    to_status::text   AS to_status,
                    changed_by,
                    reason,
                    created_at
               FROM app.ticket_status_history
              WHERE ticket_id = $1
              ORDER BY created_at, to_status",
        )
        .bind(ticket_id.as_uuid())
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        rows.into_iter().map(map_status_change).collect()
    }
}

#[async_trait]
impl TicketMessageRepository for PgUnitOfWork {
    async fn insert(&mut self, message: &TicketMessage) -> RepositoryResult<()> {
        // `ai_generated`, `ai_job_id` e `metadata` restano ai default: il triage AI
        // è il modulo successivo.
        sqlx::query(
            "INSERT INTO app.ticket_messages (
                 id, org_id, ticket_id, author_id, author_kind, body, is_internal, created_at
             ) VALUES (
                 $1, $2, $3, $4, $5::app.ticket_actor, $6, $7, $8
             )",
        )
        .bind(message.id().as_uuid())
        .bind(message.org_id().as_uuid())
        .bind(message.ticket_id().as_uuid())
        .bind(message.author_id().map(UserId::as_uuid))
        .bind(db_enums::ticket_actor_to_db(message.author_kind()))
        .bind(message.body().as_str())
        .bind(message.is_internal())
        .bind(message.created_at())
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn list_for_ticket(
        &mut self,
        query: &TicketMessageQuery,
    ) -> RepositoryResult<TicketMessagePage> {
        // Le note interne **non** si filtrano qui: lo fa `ticket_messages_select`,
        // che ha `NOT is_internal OR app.is_org_staff(org_id)`. Ripetere la
        // condizione nel `WHERE` sarebbe una seconda definizione della stessa
        // regola, e le due divergerebbero alla prima modifica di una delle due.
        //
        // `ORDER BY id` e non `created_at`: gli id sono UUIDv7, quindi l'ordine
        // coincide con quello di creazione, e il cursore resta un solo UUID come
        // in tutti gli altri elenchi dell'API. Il prezzo è che
        // `ticket_messages_thread_ix` serve l'uguaglianza su `ticket_id` ma non
        // l'ordinamento, che resta un sort su un insieme piccolo: un thread è
        // limitato da quanto due persone si scrivono.
        let fetch = i64::from(query.limit) + 1;
        let rows = sqlx::query(
            "SELECT id,
                    org_id,
                    ticket_id,
                    author_id,
                    author_kind::text AS author_kind,
                    body,
                    is_internal,
                    created_at
               FROM app.ticket_messages
              WHERE ticket_id = $1
                AND deleted_at IS NULL
                AND ($2::uuid IS NULL OR id > $2)
              ORDER BY id
              LIMIT $3",
        )
        .bind(query.ticket_id.as_uuid())
        .bind(query.after.map(TicketMessageId::as_uuid))
        .bind(fetch)
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        let has_more = rows.len() > usize::from(query.limit);
        let items = rows
            .into_iter()
            .take(usize::from(query.limit))
            .map(map_message)
            .collect::<RepositoryResult<Vec<TicketMessage>>>()?;
        let next_cursor = if has_more {
            items.last().map(TicketMessage::id)
        } else {
            None
        };

        Ok(TicketMessagePage { items, next_cursor })
    }
}

fn map_ticket(row: PgRow) -> RepositoryResult<Ticket> {
    let invalid = |field: &str, error: propflow_domain::DomainError| RepositoryError::Backend {
        details: format!("{field} non valido in database: {error}"),
    };

    let id: Uuid = try_get(&row, "id")?;
    let org_id: Uuid = try_get(&row, "org_id")?;
    let property_id: Uuid = try_get(&row, "property_id")?;
    let room_id: Option<Uuid> = try_get(&row, "room_id")?;
    let lease_id: Option<Uuid> = try_get(&row, "lease_id")?;
    let category: String = try_get(&row, "category")?;
    let priority: String = try_get(&row, "priority")?;
    let status: String = try_get(&row, "status")?;
    let reported_by: Option<Uuid> = try_get(&row, "reported_by")?;
    let reported_by_kind: String = try_get(&row, "reported_by_kind")?;
    let assigned_to: Option<Uuid> = try_get(&row, "assigned_to")?;
    let currency_code: String = try_get(&row, "currency")?;
    let estimated_cost: Option<Decimal> = try_get(&row, "estimated_cost")?;
    let actual_cost: Option<Decimal> = try_get(&row, "actual_cost")?;
    let created_at: DateTime<Utc> = try_get(&row, "created_at")?;
    let updated_at: DateTime<Utc> = try_get(&row, "updated_at")?;
    let archived_at: Option<DateTime<Utc>> = try_get(&row, "deleted_at")?;

    let currency = Currency::parse(&currency_code).map_err(|error| invalid("currency", error))?;
    let money = |field: &'static str, amount: Option<Decimal>| {
        amount
            .map(|value| Money::new(value, currency))
            .transpose()
            .map_err(|error| invalid(field, error))
    };

    Ok(Ticket::from_persistence(TicketParts {
        id: TicketId::from_uuid(id),
        org_id: OrgId::from_uuid(org_id),
        property_id: PropertyId::from_uuid(property_id),
        room_id: room_id.map(RoomId::from_uuid),
        lease_id: lease_id.map(LeaseId::from_uuid),
        reference: TicketReference::from_persistence(try_get(&row, "reference")?),
        title: TicketTitle::from_persistence(try_get(&row, "title")?),
        description: TicketDescription::from_persistence(try_get(&row, "description")?),
        category: db_enums::ticket_category_from_db(&category)?,
        priority: db_enums::ticket_priority_from_db(&priority)?,
        status: db_enums::ticket_status_from_db(&status)?,
        reported_by: reported_by.map(UserId::from_uuid),
        reported_by_kind: db_enums::ticket_actor_from_db(&reported_by_kind)?,
        assigned_to: assigned_to.map(UserId::from_uuid),
        assigned_at: try_get(&row, "assigned_at")?,
        estimated_cost: money("estimated_cost", estimated_cost)?,
        actual_cost: money("actual_cost", actual_cost)?,
        currency,
        resolved_at: try_get(&row, "resolved_at")?,
        closed_at: try_get(&row, "closed_at")?,
        created_at,
        updated_at,
        archived_at,
    }))
}

fn map_message(row: PgRow) -> RepositoryResult<TicketMessage> {
    let id: Uuid = try_get(&row, "id")?;
    let org_id: Uuid = try_get(&row, "org_id")?;
    let ticket_id: Uuid = try_get(&row, "ticket_id")?;
    let author_id: Option<Uuid> = try_get(&row, "author_id")?;
    let author_kind: String = try_get(&row, "author_kind")?;

    Ok(TicketMessage::from_persistence(NewTicketMessage {
        id: TicketMessageId::from_uuid(id),
        org_id: OrgId::from_uuid(org_id),
        ticket_id: TicketId::from_uuid(ticket_id),
        author_id: author_id.map(UserId::from_uuid),
        author_kind: db_enums::ticket_actor_from_db(&author_kind)?,
        body: MessageBody::from_persistence(try_get(&row, "body")?),
        is_internal: try_get(&row, "is_internal")?,
        now: try_get(&row, "created_at")?,
    }))
}

fn map_status_change(row: PgRow) -> RepositoryResult<TicketStatusChange> {
    let ticket_id: Uuid = try_get(&row, "ticket_id")?;
    let from_status: Option<String> = try_get(&row, "from_status")?;
    let to_status: String = try_get(&row, "to_status")?;
    let changed_by: Option<Uuid> = try_get(&row, "changed_by")?;

    Ok(TicketStatusChange::from_persistence(
        TicketId::from_uuid(ticket_id),
        from_status
            .as_deref()
            .map(db_enums::ticket_status_from_db)
            .transpose()?,
        db_enums::ticket_status_from_db(&to_status)?,
        changed_by.map(UserId::from_uuid),
        try_get(&row, "reason")?,
        try_get(&row, "created_at")?,
    ))
}

#[cfg(test)]
mod tests {
    use propflow_domain::ticket::TicketStatus;

    use crate::db::db_enums;

    /// L'elenco dei sei stati vive in due posti — la costante di dominio e il
    /// letterale SQL che rende utilizzabile `tickets_open_ix` — e questo test è ciò
    /// che li tiene insieme. Senza, aggiungere uno stato di lavorazione al dominio
    /// farebbe smettere la coda di comprenderlo, in silenzio.
    #[test]
    fn the_sql_literal_and_the_domain_constant_agree() {
        let from_domain = TicketStatus::WORKING
            .iter()
            .map(|status| format!("'{}'", db_enums::ticket_status_to_db(*status)))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(from_domain, working_statuses!());
    }
}
