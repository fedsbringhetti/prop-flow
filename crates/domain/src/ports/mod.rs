//! Porte del dominio: ciò che il dominio *richiede* dall'esterno, dichiarato in
//! termini suoi. L'infrastruttura le implementa; il dominio non la conosce.

mod clock;
mod repositories;
mod uow;

pub use clock::Clock;
pub use repositories::{
    ChargeBalancePage, ChargeQuery, ChargeRepository, DocumentPage, DocumentQuery,
    DocumentRepository, EventSink, InvitationRepository, LeaseOverlap, LeasePage,
    LeasePartyRepository, LeaseQuery, LeaseRepository, OrgMemberRepository, OrganizationRepository,
    PaymentPage, PaymentQuery, PaymentRepository, PropertyPage, PropertyQuery, PropertyRepository,
    RepositoryError, RepositoryResult, RoomRepository, SessionRepository, TicketMessagePage,
    TicketMessageQuery, TicketMessageRepository, TicketPage, TicketQuery, TicketRepository,
    UserRepository, UtilityBillPage, UtilityBillQuery, UtilityBillRepository,
};
pub use uow::{UnitOfWork, UnitOfWorkFactory};
