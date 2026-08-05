use async_trait::async_trait;
use sqlx::{PgPool, Postgres, Transaction};

use propflow_domain::RequestContext;
use propflow_domain::ports::{
    EventSink, OrgMemberRepository, OrganizationRepository, PropertyRepository, RepositoryResult,
    RoomRepository, SessionRepository, UnitOfWork, UnitOfWorkFactory, UserRepository,
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
    fn properties(&mut self) -> &mut dyn PropertyRepository {
        self
    }
    fn rooms(&mut self) -> &mut dyn RoomRepository {
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
