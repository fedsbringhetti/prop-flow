use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Transaction};

use propflow_domain::RequestContext;
use propflow_domain::ports::{
    AiJobQueue, AiJobRepository, ChargeRepository, DashboardRepository, DocumentRepository,
    EventSink, ExpenseGroupRepository, ExpenseRepository, InvitationRepository,
    LeasePartyRepository, LeaseRepository, OrgMemberRepository, OrganizationRepository,
    PaymentRepository, PropertyRepository, RepositoryResult, RoomRepository, SessionRepository,
    SettlementRepository, TicketMessageRepository, TicketRepository, UnitOfWork, UnitOfWorkFactory,
    UserRepository, UtilityBillRepository, WorkerUnitOfWork, WorkerUnitOfWorkFactory,
};

use super::db_enums;
use super::error::map_sqlx_error;
use super::pool::DatabasePools;

/// Unità di lavoro su PostgreSQL: una transazione con il contesto di sicurezza
/// già impostato.
///
/// Implementa direttamente tutte le porte dei repository. Non è pigrizia: i
/// repository devono condividere la *stessa* transazione, e un tipo separato per
/// ciascuno dovrebbe prendere in prestito la transazione contemporaneamente agli
/// altri. Con un solo tipo, `properties()` restituisce `self` visto come
/// `&mut dyn PropertyRepository`, e il prestito è uno solo per volta.
pub struct PgUnitOfWork {
    pub(super) tx: Transaction<'static, Postgres>,
}

#[async_trait]
impl UnitOfWork for PgUnitOfWork {
    fn users(&mut self) -> &mut dyn UserRepository {
        self
    }
    fn organizations(&mut self) -> &mut dyn OrganizationRepository {
        self
    }
    fn members(&mut self) -> &mut dyn OrgMemberRepository {
        self
    }
    fn sessions(&mut self) -> &mut dyn SessionRepository {
        self
    }
    fn invitations(&mut self) -> &mut dyn InvitationRepository {
        self
    }
    fn properties(&mut self) -> &mut dyn PropertyRepository {
        self
    }
    fn rooms(&mut self) -> &mut dyn RoomRepository {
        self
    }
    fn leases(&mut self) -> &mut dyn LeaseRepository {
        self
    }
    fn lease_parties(&mut self) -> &mut dyn LeasePartyRepository {
        self
    }
    fn documents(&mut self) -> &mut dyn DocumentRepository {
        self
    }
    fn tickets(&mut self) -> &mut dyn TicketRepository {
        self
    }
    fn ticket_messages(&mut self) -> &mut dyn TicketMessageRepository {
        self
    }
    fn utility_bills(&mut self) -> &mut dyn UtilityBillRepository {
        self
    }
    fn charges(&mut self) -> &mut dyn ChargeRepository {
        self
    }
    fn payments(&mut self) -> &mut dyn PaymentRepository {
        self
    }
    fn expense_groups(&mut self) -> &mut dyn ExpenseGroupRepository {
        self
    }
    fn expenses(&mut self) -> &mut dyn ExpenseRepository {
        self
    }
    fn settlements(&mut self) -> &mut dyn SettlementRepository {
        self
    }
    fn ai_jobs(&mut self) -> &mut dyn AiJobRepository {
        self
    }
    fn dashboard(&mut self) -> &mut dyn DashboardRepository {
        self
    }
    fn events(&mut self) -> &mut dyn EventSink {
        self
    }

    async fn commit(self: Box<Self>) -> RepositoryResult<()> {
        self.tx.commit().await.map_err(map_sqlx_error)
    }
}

/// Fabbrica di unità di lavoro.
pub struct PgUnitOfWorkFactory {
    pools: DatabasePools,
}

impl PgUnitOfWorkFactory {
    #[must_use]
    pub const fn new(pools: DatabasePools) -> Self {
        Self { pools }
    }

    #[must_use]
    pub const fn pools(&self) -> &DatabasePools {
        &self.pools
    }

    /// Apre la transazione e installa il contesto come **primo** statement.
    ///
    /// L'ordine è la ragione d'essere di questo tipo: dopo questa chiamata ogni
    /// query della transazione vede le GUC impostate, quindi le policy RLS sono
    /// attive. Nessun altro punto del codice ha accesso a una transazione priva
    /// di contesto.
    async fn begin_with_context(
        pool: &PgPool,
        context: Option<&RequestContext>,
    ) -> RepositoryResult<Box<dyn UnitOfWork>> {
        let mut tx = pool.begin().await.map_err(map_sqlx_error)?;

        if let Some(context) = context {
            sqlx::query("SELECT app.set_request_context($1, $2, $3, $4, $5::inet, $6, $7)")
                .bind(context.user_id().map(|id| id.as_uuid()))
                .bind(context.org_id().map(|id| id.as_uuid()))
                .bind(context.platform_admin())
                .bind(context.request_id())
                .bind(context.client_ip().map(|ip| ip.to_string()))
                .bind(context.user_agent())
                .bind(context.org_role().map(db_enums::org_role_to_db))
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        }

        Ok(Box::new(PgUnitOfWork { tx }))
    }
}

#[async_trait]
impl UnitOfWorkFactory for PgUnitOfWorkFactory {
    async fn begin(&self, context: &RequestContext) -> RepositoryResult<Box<dyn UnitOfWork>> {
        Self::begin_with_context(self.pools.application(), Some(context)).await
    }

    async fn begin_privileged(&self) -> RepositoryResult<Box<dyn UnitOfWork>> {
        // Nessun contesto da impostare: il ruolo ha BYPASSRLS e le policy non si
        // applicano. Riservata ai flussi pre-autenticazione.
        Self::begin_with_context(self.pools.privileged(), None).await
    }
}

/// Unità di lavoro del worker: la stessa transazione, un'altra postura.
///
/// Avvolge [`PgUnitOfWork`] invece di essere un tipo nuovo perché i repository
/// sono gli stessi — il SQL non cambia con il ruolo che lo esegue — e ciò che
/// cambia è **come ci si arriva**: da qui si passa dal pool privilegiato, senza
/// contesto di sicurezza, e si vedono tre repository invece di diciotto.
///
/// L'avvolgimento è la parte che conta: `PgUnitOfWork` implementa anche
/// `AiJobQueue`, ma quel trait non compare fra i metodi di `UnitOfWork`, quindi
/// una transazione applicativa non ha modo di raggiungerlo. Il confine è nel
/// sistema di tipi, non in una convenzione.
pub struct PgWorkerUnitOfWork {
    inner: PgUnitOfWork,
}

#[async_trait]
impl WorkerUnitOfWork for PgWorkerUnitOfWork {
    fn ai_jobs(&mut self) -> &mut dyn AiJobQueue {
        &mut self.inner
    }
    fn documents(&mut self) -> &mut dyn DocumentRepository {
        &mut self.inner
    }
    fn utility_bills(&mut self) -> &mut dyn UtilityBillRepository {
        &mut self.inner
    }

    async fn commit(self: Box<Self>) -> RepositoryResult<()> {
        self.inner.tx.commit().await.map_err(map_sqlx_error)
    }
}

/// Fabbrica di unità di lavoro per il worker.
///
/// Tiene **un solo pool**, quello privilegiato, e non `DatabasePools`: il worker
/// non ha niente da fare con il pool applicativo, e dargli entrambi significherebbe
/// lasciare aperta la porta di usare quello sbagliato. È la stessa ragione per cui
/// i due pool sono separati in primo luogo.
pub struct PgWorkerUnitOfWorkFactory {
    pool: PgPool,
}

impl PgWorkerUnitOfWorkFactory {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WorkerUnitOfWorkFactory for PgWorkerUnitOfWorkFactory {
    async fn begin(&self) -> RepositoryResult<Box<dyn WorkerUnitOfWork>> {
        // Nessun `app.set_request_context`: non c'è un utente di cui installare
        // il contesto, perché non c'è nessun utente. Il ruolo ha `BYPASSRLS` e
        // deve vedere i job di tutte le organizzazioni — è la stessa posizione da
        // cui il dispatcher dell'outbox legge `app.domain_events`.
        let tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        Ok(Box::new(PgWorkerUnitOfWork {
            inner: PgUnitOfWork { tx },
        }))
    }
}
