//! Porte del dominio: ciò che il dominio *richiede* dall'esterno, dichiarato in
//! termini suoi. L'infrastruttura le implementa; il dominio non la conosce.

mod clock;
mod repositories;
mod uow;

pub use clock::Clock;
pub use repositories::{
    AiJobQueue, AiJobRepository, ChargeBalancePage, ChargeQuery, ChargeRepository,
    DashboardRepository, DocumentPage, DocumentQuery, DocumentRepository, EventSink,
    ExpenseGroupPage, ExpenseGroupQuery, ExpenseGroupRepository, ExpensePage, ExpenseQuery,
    ExpenseRepository, InvitationRepository, LeaseLabels, LeaseOverlap, LeasePage,
    LeasePartyRepository, LeaseQuery, LeaseRepository, OrgMemberRepository, OrganizationRepository,
    OwnerInvitationRepository, PasswordResetCodeRepository, PaymentPage, PaymentQuery,
    PaymentRepository, PersonName, PropertyOccupancy, PropertyPage, PropertyQuery,
    PropertyRepository, RechargedAmount, RepositoryError, RepositoryResult, RoomOccupancy,
    RoomRepository, SessionRepository, SettlementPage, SettlementQuery, SettlementRepository,
    TicketLabels, TicketMessagePage, TicketMessageQuery, TicketMessageRepository, TicketPage,
    TicketQuery, TicketRepository, UserRepository, UtilityBillPage, UtilityBillQuery,
    UtilityBillRepository,
};
pub use uow::{UnitOfWork, UnitOfWorkFactory, WorkerUnitOfWork, WorkerUnitOfWorkFactory};
