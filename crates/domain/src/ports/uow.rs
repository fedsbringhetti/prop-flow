use async_trait::async_trait;

use super::repositories::{
    DocumentRepository, EventSink, InvitationRepository, LeasePartyRepository, LeaseRepository,
    OrgMemberRepository, OrganizationRepository, PropertyRepository, RepositoryResult,
    RoomRepository, SessionRepository, TicketMessageRepository, TicketRepository, UserRepository,
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
