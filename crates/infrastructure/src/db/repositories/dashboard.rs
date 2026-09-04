use async_trait::async_trait;
use rust_decimal::Decimal;

use propflow_domain::ports::{DashboardRepository, RepositoryResult};
use propflow_domain::reporting::OwnerDashboard;
use propflow_domain::shared::OrgId;

use super::user::try_get;
use crate::db::error::map_sqlx_error;
use crate::db::uow::PgUnitOfWork;

/// La dashboard **si legge e non si ricalcola**.
///
/// Nessun `COUNT` scritto qui: gli otto numeri escono da `app.v_owner_dashboard`
/// così come sono. Riprodurli in SQL nostro sarebbe una seconda definizione di
/// «insoluto» e di «segnalazione aperta», e le due divergerebbero alla prima
/// migration che tocca la vista — che è precisamente ciò che la `0024` ha fatto,
/// togliendo `void` e `written_off` dal residuo.
///
/// I due importi si castano a `numeric` come ovunque: `app.money_amount` è un
/// dominio, e il cast rende la decodifica indipendente dalla risoluzione degli OID
/// dei tipi definiti dall'utente.
///
/// **Il filtro per organizzazione è nella `WHERE` e nella policy insieme.** La
/// vista è `security_invoker`, quindi `organizations_select` la restringe già alle
/// organizzazioni di cui il chiamante è membro; la `WHERE` sceglie *quale* delle
/// sue. Senza la policy la `WHERE` basterebbe a sbagliare, senza la `WHERE` la
/// policy restituirebbe più righe a chi ne ha più di una.
#[async_trait]
impl DashboardRepository for PgUnitOfWork {
    async fn owner_dashboard(&mut self, org_id: OrgId) -> RepositoryResult<Option<OwnerDashboard>> {
        let row = sqlx::query(
            "SELECT org_id,
                    properties_count,
                    active_leases,
                    monthly_rent_expected::numeric AS monthly_rent_expected,
                    total_outstanding::numeric     AS total_outstanding,
                    overdue_charges,
                    open_tickets,
                    leases_expiring_90d,
                    bills_pending_review
               FROM app.v_owner_dashboard
              WHERE org_id = $1",
        )
        .bind(org_id.as_uuid())
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error)?;

        let Some(row) = row else {
            return Ok(None);
        };

        let org_id: uuid::Uuid = try_get(&row, "org_id")?;
        let monthly_rent_expected: Decimal = try_get(&row, "monthly_rent_expected")?;
        let total_outstanding: Decimal = try_get(&row, "total_outstanding")?;

        Ok(Some(OwnerDashboard {
            org_id: OrgId::from_uuid(org_id),
            properties_count: try_get(&row, "properties_count")?,
            active_leases: try_get(&row, "active_leases")?,
            monthly_rent_expected,
            total_outstanding,
            overdue_charges: try_get(&row, "overdue_charges")?,
            open_tickets: try_get(&row, "open_tickets")?,
            leases_expiring_90d: try_get(&row, "leases_expiring_90d")?,
            bills_pending_review: try_get(&row, "bills_pending_review")?,
        }))
    }
}
