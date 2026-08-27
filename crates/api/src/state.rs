use std::sync::Arc;

use propflow_application::ports::{
    ObjectStorage, OpaqueTokenFactory, PasswordHasher, RefreshTokenFactory, TokenIssuer,
};
use propflow_application::use_cases::auth::{Login, Logout, RefreshSessionUseCase, RegisterOwner};
use propflow_application::use_cases::document::{
    ArchiveDocument, DownloadDocument, ListDocuments, UploadDocument,
};
use propflow_application::use_cases::invitation::{
    AcceptInvitation, InviteTenant, RevokeInvitation,
};
use propflow_application::use_cases::lease::{
    ActivateLease, AddLeaseParty, DraftLease, GetLease, ListLeases, TerminateLease,
};
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

    pub draft_lease: Arc<DraftLease>,
    pub get_lease: Arc<GetLease>,
    pub list_leases: Arc<ListLeases>,
    pub add_lease_party: Arc<AddLeaseParty>,
    pub activate_lease: Arc<ActivateLease>,
    pub terminate_lease: Arc<TerminateLease>,

    pub upload_document: Arc<UploadDocument>,
    pub list_documents: Arc<ListDocuments>,
    pub download_document: Arc<DownloadDocument>,
    pub archive_document: Arc<ArchiveDocument>,

    pub invite_tenant: Arc<InviteTenant>,
    pub revoke_invitation: Arc<RevokeInvitation>,
    pub accept_invitation: Arc<AcceptInvitation>,

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
    pub invitation_tokens: Arc<dyn OpaqueTokenFactory>,
    pub clock: Arc<dyn propflow_domain::ports::Clock>,
    pub object_storage: Arc<dyn ObjectStorage>,
    /// Vita dell'URL di download pre-firmato. Vive qui e non dentro l'adattatore
    /// perché è una decisione applicativa — quanto a lungo vale un permesso — e non
    /// un dettaglio del fornitore.
    pub presign_ttl_seconds: u32,
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
            invitation_tokens,
            clock,
            object_storage,
            presign_ttl_seconds,
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
            archive_room: Arc::new(ArchiveRoom::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            draft_lease: Arc::new(DraftLease::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            get_lease: Arc::new(GetLease::new(Arc::clone(&unit_of_work), Arc::clone(&clock))),
            list_leases: Arc::new(ListLeases::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            add_lease_party: Arc::new(AddLeaseParty::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            activate_lease: Arc::new(ActivateLease::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            terminate_lease: Arc::new(TerminateLease::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            upload_document: Arc::new(UploadDocument::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&object_storage),
                Arc::clone(&clock),
            )),
            list_documents: Arc::new(ListDocuments::new(Arc::clone(&unit_of_work))),
            download_document: Arc::new(DownloadDocument::new(
                Arc::clone(&unit_of_work),
                object_storage,
                presign_ttl_seconds,
            )),
            archive_document: Arc::new(ArchiveDocument::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            invite_tenant: Arc::new(InviteTenant::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&invitation_tokens),
                Arc::clone(&clock),
            )),
            revoke_invitation: Arc::new(RevokeInvitation::new(
                Arc::clone(&unit_of_work),
                Arc::clone(&clock),
            )),
            accept_invitation: Arc::new(AcceptInvitation::new(
                Arc::clone(&unit_of_work),
                invitation_tokens,
                clock,
            )),
            tokens,
            unit_of_work,
            pools,
        }
    }
}
