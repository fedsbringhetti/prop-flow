use std::sync::Arc;

use propflow_application::ports::{PasswordHasher, RefreshTokenFactory, TokenIssuer};
use propflow_application::use_cases::auth::{Login, Logout, RefreshSessionUseCase, RegisterOwner};
use propflow_application::use_cases::property::{
    ArchiveProperty, CreateProperty, GetProperty, ListProperties, UpdateProperty,
};
use propflow_application::use_cases::room::{ArchiveRoom, CreateRoom, ListRooms, UpdateRoom};
use propflow_domain::ports::UnitOfWorkFactory;
use propflow_infrastructure::DatabasePools;

/// Contenitore delle dipendenze passato agli handler.
///
/// Gli handler vedono solo use case e porte: nessun `PgPool`, nessun tipo di
/// SQLx. È il confine che impedisce a un handler di scrivere una query, e quindi
/// di aggirare il service layer e la Row Level Security.
#[derive(Clone)]
pub struct AppState {
    pub register_owner: Arc<RegisterOwner>,
    pub login: Arc<Login>,
    pub refresh_session: Arc<RefreshSessionUseCase>,
    pub logout: Arc<Logout>,

    pub create_property: Arc<CreateProperty>,
    pub get_property: Arc<GetProperty>,
    pub list_properties: Arc<ListProperties>,
    pub update_property: Arc<UpdateProperty>,
    pub archive_property: Arc<ArchiveProperty>,

    pub create_room: Arc<CreateRoom>,
    pub list_rooms: Arc<ListRooms>,
    pub update_room: Arc<UpdateRoom>,
    pub archive_room: Arc<ArchiveRoom>,

    /// Serve al middleware di autenticazione per verificare gli access token.
    pub tokens: Arc<dyn TokenIssuer>,
    /// Serve al middleware per risolvere il ruolo quando il client cambia
    /// organizzazione con l'header `X-Org-Id`.
    pub unit_of_work: Arc<dyn UnitOfWorkFactory>,
    /// Usato solo dall'endpoint di prontezza.
    pub pools: DatabasePools,
}

/// Componenti concrete costruite in `main`, prima di essere composte negli use case.
pub struct Dependencies {
    pub unit_of_work: Arc<dyn UnitOfWorkFactory>,
    pub hasher: Arc<dyn PasswordHasher>,
    pub tokens: Arc<dyn TokenIssuer>,
    pub refresh_tokens: Arc<dyn RefreshTokenFactory>,
    pub clock: Arc<dyn propflow_domain::ports::Clock>,
    pub pools: DatabasePools,
}

impl AppState {
    #[must_use]
    pub fn new(dependencies: Dependencies) -> Self {
        let Dependencies {
            unit_of_work,
            hasher,
            tokens,
            refresh_tokens,
            clock,
            pools,
        } = dependencies;

        Self {
            register_owner: Arc::new(RegisterOwner::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&hasher),
                Arc::clone(&clock),
            )),
            login: Arc::new(Login::new(
                Arc::clone(&unit_of_work),
                hasher,
                Arc::clone(&tokens),
                Arc::clone(&refresh_tokens),
                Arc::clone(&clock),
            )),
            refresh_session: Arc::new(RefreshSessionUseCase::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&tokens),
                Arc::clone(&refresh_tokens),
                Arc::clone(&clock),
            )),
            logout: Arc::new(Logout::new(
                Arc::clone(&unit_of_work),
                refresh_tokens,
                Arc::clone(&clock),
            )),
            create_property: Arc::new(CreateProperty::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            get_property: Arc::new(GetProperty::new(Arc::clone(&unit_of_work))),
            list_properties: Arc::new(ListProperties::new(Arc::clone(&unit_of_work))),
            update_property: Arc::new(UpdateProperty::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            archive_property: Arc::new(ArchiveProperty::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            create_room: Arc::new(CreateRoom::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            list_rooms: Arc::new(ListRooms::new(Arc::clone(&unit_of_work))),
            update_room: Arc::new(UpdateRoom::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            archive_room: Arc::new(ArchiveRoom::new(Arc::clone(&unit_of_work), clock)),
            tokens,
            unit_of_work,
            pools,
        }
    }
}
