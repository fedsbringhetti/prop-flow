use async_trait::async_trait;

use super::repositories::{
    AiJobQueue, AiJobRepository, ChargeRepository, DashboardRepository, DocumentRepository,
    EventSink, ExpenseGroupRepository, ExpenseRepository, InvitationRepository,
    LeasePartyRepository, LeaseRepository, OrgMemberRepository, OrganizationRepository,
    PaymentRepository, PropertyRepository, RepositoryResult, RoomRepository, SessionRepository,
    SettlementRepository, TicketMessageRepository, TicketRepository, UserRepository,
    UtilityBillRepository,
};
use crate::context::RequestContext;

/// Unità di lavoro: una transazione con il contesto di sicurezza già impostato.
///
/// Tutti i repository si ottengono **solo** da qui. È il punto in cui il vincolo
/// "ogni query gira sotto Row Level Security" diventa strutturale: non esiste un
/// modo di ottenere un repository senza aver prima passato un [`RequestContext`]
/// a [`UnitOfWorkFactory::begin`], quindi dimenticare il contesto non è un bug
/// possibile — è codice che non compila.
///
/// Senza `commit` esplicito il `Drop` della transazione fa rollback.
#[async_trait]
pub trait UnitOfWork: Send {
    fn users(&mut self) -> &mut dyn UserRepository;
    fn organizations(&mut self) -> &mut dyn OrganizationRepository;
    fn members(&mut self) -> &mut dyn OrgMemberRepository;
    fn sessions(&mut self) -> &mut dyn SessionRepository;
    fn invitations(&mut self) -> &mut dyn InvitationRepository;
    fn properties(&mut self) -> &mut dyn PropertyRepository;
    fn rooms(&mut self) -> &mut dyn RoomRepository;
    fn leases(&mut self) -> &mut dyn LeaseRepository;
    fn lease_parties(&mut self) -> &mut dyn LeasePartyRepository;
    fn documents(&mut self) -> &mut dyn DocumentRepository;
    fn tickets(&mut self) -> &mut dyn TicketRepository;
    fn ticket_messages(&mut self) -> &mut dyn TicketMessageRepository;
    fn utility_bills(&mut self) -> &mut dyn UtilityBillRepository;
    fn charges(&mut self) -> &mut dyn ChargeRepository;
    fn payments(&mut self) -> &mut dyn PaymentRepository;
    fn expense_groups(&mut self) -> &mut dyn ExpenseGroupRepository;
    fn expenses(&mut self) -> &mut dyn ExpenseRepository;
    fn settlements(&mut self) -> &mut dyn SettlementRepository;
    /// Solo accodamento e lettura: l'esito di un job lo scrive il worker. Vedi
    /// [`AiJobRepository`].
    fn ai_jobs(&mut self) -> &mut dyn AiJobRepository;
    /// Sola lettura, e nessuna scrittura possibile: la dashboard è una vista.
    fn dashboard(&mut self) -> &mut dyn DashboardRepository;
    fn events(&mut self) -> &mut dyn EventSink;

    async fn commit(self: Box<Self>) -> RepositoryResult<()>;
}

#[async_trait]
pub trait UnitOfWorkFactory: Send + Sync {
    /// Apre una transazione e imposta il contesto di sicurezza come primo
    /// statement, prima che qualunque query possa girare.
    async fn begin(&self, context: &RequestContext) -> RepositoryResult<Box<dyn UnitOfWork>>;

    /// Transazione privilegiata, non soggetta alle policy di riga.
    ///
    /// Serve al solo caso in cui un contesto non può esistere: la registrazione di
    /// un nuovo utente, che avviene prima dell'autenticazione e deve creare utente,
    /// organizzazione e membership quando ancora nessuna delle tre esiste.
    /// Ogni altro uso è un errore di progetto.
    async fn begin_privileged(&self) -> RepositoryResult<Box<dyn UnitOfWork>>;
}

/// Unità di lavoro del **worker**: una transazione senza contesto di sicurezza.
///
/// Esiste come tipo a sé e non come un terzo metodo di [`UnitOfWorkFactory`] per
/// tenere separate due cose che si somigliano e non sono la stessa:
///
/// - `begin_privileged()` è per i flussi in cui un contesto **non può** esistere
///   perché avvengono prima dell'autenticazione — registrazione, login, rotazione
///   del refresh token. La regola 20 di `CLAUDE.md` ne tiene piccola la superficie
///   apposta, e allargarla al ciclo dell'OCR sarebbe precisamente la scorciatoia
///   che quella regola esiste per impedire;
/// - questa è per un processo che **non serve una richiesta**. Il worker gira come
///   `propflow_worker` con `BYPASSRLS`, con il **proprio pool**, e deve vedere i
///   job di tutte le organizzazioni: non c'è un utente di cui installare il
///   contesto, perché non c'è nessun utente. È la stessa posizione da cui il
///   dispatcher dell'outbox legge `app.domain_events` dal primo giorno.
///
/// Ne segue la proprietà che conta: da qui **non** si ottengono i repository
/// dell'applicazione. Ci sono i tre che il ciclo dell'OCR tocca e nient'altro,
/// quindi la superficie senza RLS resta leggibile in venti righe invece di essere
/// «tutto, ma con un altro pool».
#[async_trait]
pub trait WorkerUnitOfWork: Send {
    fn ai_jobs(&mut self) -> &mut dyn AiJobQueue;
    /// Serve a leggere il documento in ingresso — e a **non** leggerlo quando è
    /// stato archiviato: `find_by_id` filtra `deleted_at IS NULL`, quindi un
    /// documento tolto da una persona torna `None` anche a chi ha `BYPASSRLS`.
    fn documents(&mut self) -> &mut dyn DocumentRepository;
    fn utility_bills(&mut self) -> &mut dyn UtilityBillRepository;

    async fn commit(self: Box<Self>) -> RepositoryResult<()>;
}

#[async_trait]
pub trait WorkerUnitOfWorkFactory: Send + Sync {
    /// Apre una transazione sul pool del worker. Nessun contesto da installare:
    /// il ruolo ha `BYPASSRLS` e le policy non si applicano.
    async fn begin(&self) -> RepositoryResult<Box<dyn WorkerUnitOfWork>>;
}
