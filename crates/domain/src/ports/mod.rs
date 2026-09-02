//! Porte del dominio: ciò che il dominio *richiede* dall'esterno, dichiarato in
//! termini suoi. L'infrastruttura le implementa; il dominio non la conosce.

mod clock;
mod repositories;
mod uow;

pub use clock::Clock;
pub use repositories::{
    ChargeBalancePage, ChargeQuery, ChargeRepository, DocumentPage, DocumentQuery,
    DocumentRepository, EventSink, ExpenseGroupPage, ExpenseGroupQuery, ExpenseGroupRepository,
    ExpensePage, ExpenseQuery, ExpenseRepository, InvitationRepository, LeaseOverlap, LeasePage,
    LeasePartyRepository, LeaseQuery, LeaseRepository, OrgMemberRepository, OrganizationRepository,
    PaymentPage, PaymentQuery, PaymentRepository, PropertyPage, PropertyQuery, PropertyRepository,
    RepositoryError, RepositoryResult, RoomRepository, SessionRepository, SettlementPage,
    SettlementQuery, SettlementRepository, TicketMessagePage, TicketMessageQuery,
    TicketMessageRepository, TicketPage, TicketQuery, TicketRepository, UserRepository,
    UtilityBillPage, UtilityBillQuery, UtilityBillRepository,
};
pub use uow::{UnitOfWork, UnitOfWorkFactory};
