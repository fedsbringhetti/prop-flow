use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::ports::{PropertyPage, PropertyQuery, UnitOfWorkFactory};
use propflow_domain::property::PropertyStatus;
use propflow_domain::shared::PropertyId;

use crate::authorization;
use crate::error::AppResult;

#[derive(Debug, Clone, Default)]
pub struct ListPropertiesQuery {
    pub status: Option<PropertyStatus>,
    pub city: Option<String>,
    pub after: Option<PropertyId>,
    pub limit: Option<u16>,
}

/// Elenca gli immobili visibili al chiamante.
pub struct ListProperties {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
}

impl ListProperties {
    #[must_use]
    pub const fn new(unit_of_work: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        context: &RequestContext,
        query: ListPropertiesQuery,
    ) -> AppResult<PropertyPage> {
        authorization::ensure_member(context)?;

        // Il limite si tronca invece di rifiutare: un client che chiede 10_000
        // righe ottiene la pagina massima, non un errore da gestire. Ma il tetto
        // resta, altrimenti un solo GET può esaurire memoria e connessione.
        let limit = query
            .limit
            .unwrap_or(PropertyQuery::DEFAULT_LIMIT)
            .clamp(1, PropertyQuery::MAX_LIMIT);

        let repository_query = PropertyQuery {
            status: query.status,
            city: query
                .city
                .map(|city| city.trim().to_owned())
                .filter(|c| !c.is_empty()),
            after: query.after,
            limit,
        };

        let mut uow = self.unit_of_work.begin(context).await?;
        let page = uow.properties().list(&repository_query).await?;
        Ok(page)
    }
}
