use std::sync::Arc;

use propflow_domain::RequestContext;
use propflow_domain::ports::{PropertyQuery, UnitOfWorkFactory};
use propflow_domain::property::{Property, PropertyStatus};
use propflow_domain::shared::PropertyId;

use super::PropertyView;
use crate::authorization;
use crate::error::{AppError, AppResult, UnsupportedQuery};

/// Pagina di immobili, ognuno con lo stato che discende dai contratti.
#[derive(Debug, Clone)]
pub struct ListPropertiesPage {
    pub items: Vec<PropertyView>,
    pub next_cursor: Option<PropertyId>,
}

#[derive(Debug, Clone, Default)]
pub struct ListPropertiesQuery {
    pub status: Option<PropertyStatus>,
    pub occupied: Option<bool>,
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
    ) -> AppResult<ListPropertiesPage> {
        authorization::ensure_member(context)?;

        // `rented` e `partially_rented` non sono memorizzati: filtrarci sopra
        // restituirebbe sempre zero righe, che è il modo peggiore di dirlo. È la
        // terza occorrenza della stessa forma — contratti, addebiti, immobili — e
        // il messaggio indica il filtro che funziona.
        if query.status.is_some_and(PropertyStatus::is_derived) {
            return Err(AppError::UnsupportedQuery(
                UnsupportedQuery::PropertyDerivedStatusFilter,
            ));
        }

        // Il limite si tronca invece di rifiutare: un client che chiede 10_000
        // righe ottiene la pagina massima, non un errore da gestire. Ma il tetto
        // resta, altrimenti un solo GET può esaurire memoria e connessione.
        let limit = query
            .limit
            .unwrap_or(PropertyQuery::DEFAULT_LIMIT)
            .clamp(1, PropertyQuery::MAX_LIMIT);

        let repository_query = PropertyQuery {
            status: query.status,
            occupied: query.occupied,
            city: query
                .city
                .map(|city| city.trim().to_owned())
                .filter(|c| !c.is_empty()),
            after: query.after,
            limit,
        };

        let mut uow = self.unit_of_work.begin(context).await?;
        let page = uow.properties().list(&repository_query).await?;

        // Una query per la pagina, non una per riga.
        let ids: Vec<PropertyId> = page.items.iter().map(Property::id).collect();
        let occupancy = uow.properties().occupancy(&ids).await?;

        Ok(ListPropertiesPage {
            items: page
                .items
                .into_iter()
                .map(|property| {
                    let derived = occupancy
                        .iter()
                        .find(|entry| entry.property_id == property.id())
                        .and_then(|entry| entry.status);
                    PropertyView {
                        property,
                        occupancy_status: derived,
                    }
                })
                .collect(),
            next_cursor: page.next_cursor,
        })
    }
}
