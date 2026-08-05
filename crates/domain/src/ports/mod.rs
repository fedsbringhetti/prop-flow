//! Porte del dominio: ciò che il dominio *richiede* dall'esterno, dichiarato in
//! termini suoi. L'infrastruttura le implementa; il dominio non la conosce.

mod clock;
mod repositories;
mod uow;

pub use clock::Clock;
pub use repositories::{
    EventSink, InvitationRepository, LeaseOverlap, LeasePage, LeasePartyRepository, LeaseQuery,
    LeaseRepository, OrgMemberRepository, OrganizationRepository, PropertyPage, PropertyQuery,
    PropertyRepository, RepositoryError, RepositoryResult, RoomRepository, SessionRepository,
    UserRepository,
};
pub use uow::{UnitOfWork, UnitOfWorkFactory};
