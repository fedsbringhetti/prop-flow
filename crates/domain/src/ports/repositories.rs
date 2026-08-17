use async_trait::async_trait;

use crate::events::DomainEvent;
use crate::identity::{Invitation, OrgMember, Organization, RefreshSession, TokenHash, User};
use crate::lease::{Lease, LeaseParty, LeaseStatus};
use crate::property::{Property, PropertyStatus, Room};
use crate::shared::{Email, InvitationId, LeaseId, OrgId, PropertyId, RoomId, UserId};

/// Errori della persistenza espressi senza nominare la tecnologia.
///
/// Non esiste una variante `NotFound`: i metodi di lettura restituiscono
/// `Option`, e trasformare l'assenza in errore è una decisione applicativa.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RepositoryError {
    /// Un vincolo di unicità ha rifiutato la scrittura (email già usata,
    /// idempotency key ripetuta, slug occupato).
    #[error("conflitto sul vincolo `{constraint}`")]
    Conflict { constraint: String },

    /// Un'invariante applicata dal database ha rifiutato la scrittura: check,
    /// foreign key, exclusion constraint.
    #[error("violazione di integrità: {details}")]
    Integrity { details: String },

    /// Il periodo richiesto si sovrappone a una locazione già presente.
    ///
    /// Ha una variante propria e non finisce in [`Self::Integrity`] perché è
    /// l'unico modo di rilevare questa invariante: una verifica applicativa
    /// preventiva sarebbe soggetta a race condition fra due transazioni
    /// concorrenti (ADR-015), quindi il conflitto arriva necessariamente dal
    /// vincolo di esclusione — e chi chiama merita di sapere che ha chiesto un
    /// periodo occupato, non "un vincolo violato".
    #[error("periodo sovrapposto a una locazione già presente su {0}")]
    OverlappingLease(LeaseOverlap),

    /// La scrittura è stata rifiutata dalle policy di riga. Segnala un difetto:
    /// il service layer avrebbe dovuto fermare la richiesta prima.
    #[error("scrittura rifiutata dalle policy di riga")]
    RowLevelSecurity,

    #[error("errore del backend di persistenza: {details}")]
    Backend { details: String },
}

pub type RepositoryResult<T> = Result<T, RepositoryError>;

/// Oggetto della sovrapposizione. Distinguerli non è cosmetico: in un co-living
/// "l'immobile è occupato" sarebbe sbagliato, perché le altre stanze sono libere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseOverlap {
    Property,
    Room,
}

impl std::fmt::Display for LeaseOverlap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // In inglese perché finisce nel `detail` della risposta, che è pubblico.
        f.write_str(match self {
            Self::Property => "this property",
            Self::Room => "this room",
        })
    }
}

#[async_trait]
pub trait UserRepository: Send {
    async fn find_by_id(&mut self, id: UserId) -> RepositoryResult<Option<User>>;
    async fn find_by_email(&mut self, email: &Email) -> RepositoryResult<Option<User>>;
    async fn insert(&mut self, user: &User) -> RepositoryResult<()>;
    async fn update(&mut self, user: &User) -> RepositoryResult<()>;
}

#[async_trait]
pub trait OrganizationRepository: Send {
    async fn find_by_id(&mut self, id: OrgId) -> RepositoryResult<Option<Organization>>;
    async fn insert(&mut self, organization: &Organization) -> RepositoryResult<()>;
}

#[async_trait]
pub trait OrgMemberRepository: Send {
    async fn find(&mut self, org_id: OrgId, user_id: UserId)
    -> RepositoryResult<Option<OrgMember>>;
    async fn list_for_user(&mut self, user_id: UserId) -> RepositoryResult<Vec<OrgMember>>;
    async fn insert(&mut self, member: &OrgMember) -> RepositoryResult<()>;
}

#[async_trait]
pub trait SessionRepository: Send {
    async fn find_by_token_hash(
        &mut self,
        token_hash: &TokenHash,
    ) -> RepositoryResult<Option<RefreshSession>>;
    async fn insert(&mut self, session: &RefreshSession) -> RepositoryResult<()>;
    async fn update(&mut self, session: &RefreshSession) -> RepositoryResult<()>;
    /// Revoca ogni sessione attiva dell'utente. Usata al rilevamento del riuso di
    /// un refresh token: se un token è stato replicato non si sa quale delle due
    /// parti sia legittima, quindi cadono tutte.
    async fn revoke_all_for_user(&mut self, user_id: UserId) -> RepositoryResult<u64>;
}

/// Filtro di elenco degli immobili con paginazione keyset.
///
/// Keyset e non OFFSET: con OFFSET il database scarta N righe a ogni pagina, e le
/// righe inserite intanto fanno saltare o ripetere elementi.
#[derive(Debug, Clone)]
pub struct PropertyQuery {
    pub status: Option<PropertyStatus>,
    pub city: Option<String>,
    /// Ultimo identificatore della pagina precedente. Gli id sono UUIDv7, quindi
    /// ordinarli equivale a ordinarli per data di creazione.
    pub after: Option<PropertyId>,
    pub limit: u16,
}

impl PropertyQuery {
    pub const DEFAULT_LIMIT: u16 = 25;
    pub const MAX_LIMIT: u16 = 100;
}

impl Default for PropertyQuery {
    fn default() -> Self {
        Self {
            status: None,
            city: None,
            after: None,
            limit: Self::DEFAULT_LIMIT,
        }
    }
}

/// Pagina di risultati. `next_cursor` è `None` quando non c'è altro da leggere.
#[derive(Debug, Clone)]
pub struct PropertyPage {
    pub items: Vec<Property>,
    pub next_cursor: Option<PropertyId>,
}

#[async_trait]
pub trait PropertyRepository: Send {
    async fn find_by_id(&mut self, id: PropertyId) -> RepositoryResult<Option<Property>>;
    async fn list(&mut self, query: &PropertyQuery) -> RepositoryResult<PropertyPage>;
    async fn insert(&mut self, property: &Property) -> RepositoryResult<()>;
    async fn update(&mut self, property: &Property) -> RepositoryResult<()>;
    /// Soft delete: la riga resta, con `deleted_at` valorizzato e lo stato
    /// portato ad archiviato in un solo statement. Il ruolo applicativo non ha
    /// nemmeno il privilegio `DELETE`.
    async fn soft_delete(&mut self, property: &Property) -> RepositoryResult<()>;
}

/// Stanze di un immobile.
///
/// Non c'è un `list` generale né una paginazione: una stanza non ha vita propria
/// e si legge sempre attraverso l'immobile che la contiene, dove il numero di
/// righe è per costruzione piccolo.
#[async_trait]
pub trait RoomRepository: Send {
    async fn find_by_id(&mut self, id: RoomId) -> RepositoryResult<Option<Room>>;
    async fn list_for_property(&mut self, property_id: PropertyId) -> RepositoryResult<Vec<Room>>;
    async fn insert(&mut self, room: &Room) -> RepositoryResult<()>;
    async fn update(&mut self, room: &Room) -> RepositoryResult<()>;
    /// Soft delete: la riga resta con `deleted_at` valorizzato, perché i
    /// contratti per stanza continuano a riferirsi a essa.
    async fn soft_delete(&mut self, room: &Room) -> RepositoryResult<()>;
}

/// Filtro di elenco dei contratti con paginazione keyset.
#[derive(Debug, Clone)]
pub struct LeaseQuery {
    pub property_id: Option<PropertyId>,
    pub status: Option<LeaseStatus>,
    /// `Some(true)` = solo i contratti in essere oggi, `Some(false)` = solo quelli
    /// che non lo sono. Il predicato è `app.lease_in_force`, la stessa funzione
    /// usata dalle viste: due definizioni di "in essere" si contraddirebbero.
    pub in_force: Option<bool>,
    pub after: Option<LeaseId>,
    pub limit: u16,
}

impl LeaseQuery {
    pub const DEFAULT_LIMIT: u16 = 25;
    pub const MAX_LIMIT: u16 = 100;
}

impl Default for LeaseQuery {
    fn default() -> Self {
        Self {
            property_id: None,
            status: None,
            in_force: None,
            after: None,
            limit: Self::DEFAULT_LIMIT,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LeasePage {
    pub items: Vec<Lease>,
    pub next_cursor: Option<LeaseId>,
}

#[async_trait]
pub trait LeaseRepository: Send {
    async fn find_by_id(&mut self, id: LeaseId) -> RepositoryResult<Option<Lease>>;
    async fn list(&mut self, query: &LeaseQuery) -> RepositoryResult<LeasePage>;
    async fn insert(&mut self, lease: &Lease) -> RepositoryResult<()>;
    async fn update(&mut self, lease: &Lease) -> RepositoryResult<()>;
    async fn soft_delete(&mut self, lease: &Lease) -> RepositoryResult<()>;
}

/// Parti di un contratto.
///
/// Nessun `find_by_id`: una parte si raggiunge solo attraverso il proprio
/// contratto, e caricarne una da sola non è un'operazione di dominio.
#[async_trait]
pub trait LeasePartyRepository: Send {
    async fn list_for_lease(&mut self, lease_id: LeaseId) -> RepositoryResult<Vec<LeaseParty>>;
    async fn insert(&mut self, party: &LeaseParty) -> RepositoryResult<()>;
    async fn update(&mut self, party: &LeaseParty) -> RepositoryResult<()>;
}

/// Inviti a entrare in un'organizzazione.
///
/// La ricerca è per **hash del token** e non per email: chi accetta prova di
/// possedere il token, non di conoscere un indirizzo. Cercare per email
/// trasformerebbe l'invito in qualcosa di indovinabile.
#[async_trait]
pub trait InvitationRepository: Send {
    async fn find_by_id(&mut self, id: InvitationId) -> RepositoryResult<Option<Invitation>>;
    async fn find_by_token_hash(
        &mut self,
        token_hash: &TokenHash,
    ) -> RepositoryResult<Option<Invitation>>;
    async fn insert(&mut self, invitation: &Invitation) -> RepositoryResult<()>;
    async fn update(&mut self, invitation: &Invitation) -> RepositoryResult<()>;
}

/// Destinazione degli eventi di dominio: l'outbox, scritta nella stessa
/// transazione del comando.
#[async_trait]
pub trait EventSink: Send {
    async fn append(&mut self, event: &DomainEvent) -> RepositoryResult<()>;
}
