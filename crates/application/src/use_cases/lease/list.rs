use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::lease::LeaseStatus;
use propflow_domain::ports::{Clock, LeaseLabels, LeaseQuery, UnitOfWorkFactory};
use propflow_domain::shared::{LeaseId, PropertyId};

use super::LeaseView;
use crate::authorization;
use crate::error::{AppError, AppResult, UnsupportedQuery};

#[derive(Debug, Clone, Default)]
pub struct ListLeasesQuery {
    pub property_id: Option<PropertyId>,
    pub status: Option<LeaseStatus>,
    pub in_force: Option<bool>,
    pub after: Option<LeaseId>,
    pub limit: Option<u16>,
}

/// Un contratto in elenco: lo stato effettivo, e come si chiama ciò che affitta.
///
/// I nomi sono qui e non dentro `LeaseView` perché non appartengono al contratto:
/// sono ciò che serve a **distinguere le righe**, che senza mostrano tipo e date e
/// si somigliano tutte.
#[derive(Debug, Clone)]
pub struct LeaseListItem {
    pub view: LeaseView,
    pub labels: Option<LeaseLabels>,
}

/// Pagina di contratti, ognuno con lo stato effettivo già calcolato.
#[derive(Debug, Clone)]
pub struct LeaseListPage {
    pub items: Vec<LeaseListItem>,
    pub next_cursor: Option<LeaseId>,
}

/// Elenca i contratti visibili al chiamante: tutti quelli dell'organizzazione per
/// lo staff, i propri per l'inquilino.
pub struct ListLeases {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl ListLeases {
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
        query: ListLeasesQuery,
    ) -> AppResult<LeaseListPage> {
        authorization::ensure_member(context)?;

        // `expiring` ed `expired` non sono memorizzati (ADR-019): filtrarci sopra
        // restituirebbe sempre zero righe, che è il modo peggiore di dirlo. Meglio
        // un 422 che indica il filtro corretto.
        //
        // Non è una `Validation`: il valore non è malformato, è derivato. Il codice
        // distinto lo separa da "`status` non è un valore ammesso", che è un errore
        // diverso sullo stesso campo (ADR-022).
        if query.status.is_some_and(LeaseStatus::is_derived) {
            return Err(AppError::UnsupportedQuery(
                UnsupportedQuery::LeaseDerivedStatusFilter,
            ));
        }

        // Il limite si tronca invece di rifiutare: un client che chiede 10_000
        // righe ottiene la pagina massima, non un errore da gestire.
        let limit = query
            .limit
            .unwrap_or(LeaseQuery::DEFAULT_LIMIT)
            .clamp(1, LeaseQuery::MAX_LIMIT);

        let repository_query = LeaseQuery {
            property_id: query.property_id,
            status: query.status,
            in_force: query.in_force,
            after: query.after,
            limit,
        };

        let mut uow = self.unit_of_work.begin(context).await?;
        let page = uow.leases().list(&repository_query).await?;

        // **Una query per la pagina intera**, non una per riga: venticinque
        // contratti risolti uno alla volta sarebbero venticinque viaggi al
        // database per scrivere venticinque nomi.
        let ids: Vec<LeaseId> = page
            .items
            .iter()
            .map(propflow_domain::lease::Lease::id)
            .collect();
        let mut labels = uow.leases().labels(&ids).await?;

        let today = self.clock.now().date_naive();
        Ok(LeaseListPage {
            items: page
                .items
                .into_iter()
                .map(|lease| {
                    let id = lease.id();
                    let position = labels.iter().position(|entry| entry.lease_id == id);
                    LeaseListItem {
                        labels: position.map(|index| labels.swap_remove(index)),
                        view: LeaseView::new(lease, today),
                    }
                })
                .collect(),
            next_cursor: page.next_cursor,
        })
    }
}
