use async_trait::async_trait;

use crate::ai::{AiJob, AiTask};
use crate::billing::{
    BillStatus, Charge, ChargeBalance, ChargeStatus, ChargeType, UtilityBill, UtilityType,
};
use crate::document::{AttachmentOwner, Document};
use crate::events::DomainEvent;
use crate::expense::{
    Expense, ExpenseGroup, ExpenseGroupMember, ExpenseShare, GroupBalance, Settlement,
    SettlementStatus,
};
use crate::identity::{
    Invitation, OrgMember, Organization, OrganizationMembership, RefreshSession, TokenHash, User,
};
use crate::lease::{Lease, LeaseParty, LeaseStatus};
use crate::payment::{IdempotencyKey, Payment, PaymentStatus};
use crate::property::{Property, PropertyResident, PropertyStatus, Room};
use crate::reporting::OwnerDashboard;
use crate::shared::{
    ChargeId, DocumentId, Email, ExpenseGroupId, ExpenseId, InvitationId, LeaseId, LeasePartyId,
    OrgId, PaymentId, PropertyId, RoomId, SettlementId, TicketId, TicketMessageId, UserId,
    UtilityBillId,
};
use crate::ticket::{Ticket, TicketMessage, TicketReference, TicketStatus, TicketStatusChange};

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
    /// Le organizzazioni dell'utente **con il loro nome**.
    ///
    /// Distinta da [`OrgMemberRepository::list_for_user`], che restituisce le sole
    /// appartenenze e serve al login: là il nome non serve, qui è tutto il punto —
    /// senza, un elenco di organizzazioni è un elenco di UUID.
    ///
    /// È un join e non un ciclo di `find_by_id` sulle organizzazioni: la seconda
    /// forma sarebbe un `N+1` su una domanda che una query sola risolve, e
    /// `organizations_select` è già `is_org_member(id)`, quindi la Row Level
    /// Security restringe il join alle proprie senza che il `WHERE` debba dirlo.
    async fn list_organizations_for_user(
        &mut self,
        user_id: UserId,
    ) -> RepositoryResult<Vec<OrganizationMembership>>;
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

    /// Chi abita l'immobile, **letto da `app.property_residents`**.
    ///
    /// Non è una query con i join, ed è la decisione centrale della fetta: unire
    /// `app.lease_parties` ad `app.leases` — e unirle è obbligatorio, perché
    /// l'immobile e «in essere» stanno sul contratto — passa da `leases_select`,
    /// che è per contratto, e l'elenco si riduce al solo chiamante **senza alcun
    /// errore**. Vedi [`PropertyResident`] e la migration `0030`.
    ///
    /// Restituisce un elenco vuoto anche quando l'immobile non è visibile: il
    /// confine è dentro la funzione. Chi chiama rilegge l'immobile prima, per
    /// distinguere «non esiste» da «non ci abita nessuno».
    async fn list_residents(
        &mut self,
        property_id: PropertyId,
    ) -> RepositoryResult<Vec<PropertyResident>>;
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
/// `find_by_id` esiste perché **correggere una parte è un'operazione su una riga
/// sola**: chi sistema una quota sbagliata ha in mano l'identificatore di quella
/// parte, e farlo passare dal contratto vorrebbe dire caricarne l'elenco per
/// scartarne tutte tranne una.
#[async_trait]
pub trait LeasePartyRepository: Send {
    async fn find_by_id(&mut self, id: LeasePartyId) -> RepositoryResult<Option<LeaseParty>>;
    async fn list_for_lease(&mut self, lease_id: LeaseId) -> RepositoryResult<Vec<LeaseParty>>;
    async fn insert(&mut self, party: &LeaseParty) -> RepositoryResult<()>;
    async fn update(&mut self, party: &LeaseParty) -> RepositoryResult<()>;
    /// Cancellazione soft di una parte inserita per sbaglio.
    ///
    /// Separata da `update` perché quella ha `deleted_at IS NULL` nella `WHERE` e
    /// non tocca la colonna: sono due operazioni diverse, e scriverne una sola che
    /// fa entrambe renderebbe possibile cancellare per distrazione.
    async fn soft_delete(&mut self, party: &LeaseParty) -> RepositoryResult<()>;
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

/// Filtro di elenco dei documenti di un owner, con paginazione keyset.
///
/// Il filtro sull'owner non è opzionale: un elenco di *tutti* i documenti
/// dell'organizzazione non è un'operazione che l'API espone, e renderlo possibile
/// nella porta significherebbe scriverne l'implementazione perché "prima o poi
/// servirà". Un documento si raggiunge dall'oggetto a cui è allegato.
#[derive(Debug, Clone)]
pub struct DocumentQuery {
    pub owner: AttachmentOwner,
    pub after: Option<DocumentId>,
    pub limit: u16,
}

impl DocumentQuery {
    pub const DEFAULT_LIMIT: u16 = 25;
    pub const MAX_LIMIT: u16 = 100;

    #[must_use]
    pub const fn for_owner(owner: AttachmentOwner) -> Self {
        Self {
            owner,
            after: None,
            limit: Self::DEFAULT_LIMIT,
        }
    }
}

#[derive(Debug)]
pub struct DocumentPage {
    pub items: Vec<Document>,
    pub next_cursor: Option<DocumentId>,
}

/// Metadati dei documenti.
///
/// Nessun metodo restituisce il contenuto: quello sta dietro `ObjectStorage`, ed è
/// il confine che permette di cambiare fornitore di storage senza toccare la
/// persistenza. Nessun `update` generico: in questa fetta un documento nasce e
/// viene archiviato, non si modifica — il versionamento è V2.
#[async_trait]
pub trait DocumentRepository: Send {
    async fn find_by_id(&mut self, id: DocumentId) -> RepositoryResult<Option<Document>>;
    async fn list_by_owner(&mut self, query: &DocumentQuery) -> RepositoryResult<DocumentPage>;
    async fn insert(&mut self, document: &Document) -> RepositoryResult<()>;
    /// Soft delete: la riga resta con `deleted_at` valorizzato. **L'oggetto nello
    /// storage non viene toccato** — il ruolo applicativo non ha nemmeno il
    /// privilegio `DELETE`, e rimuovere i byte è un lavoro a parte, insieme al
    /// purge di `retention_until`.
    async fn archive(&mut self, document: &Document) -> RepositoryResult<()>;
}

/// Filtro di elenco delle segnalazioni con paginazione keyset.
#[derive(Debug, Clone)]
pub struct TicketQuery {
    pub property_id: Option<PropertyId>,
    pub status: Option<TicketStatus>,
    /// `Some(true)` = solo le segnalazioni ancora in lavorazione, `Some(false)` =
    /// solo quelle terminate.
    ///
    /// Il ramo positivo enumera i sei stati di [`TicketStatus::WORKING`] invece di
    /// scrivere `status <> 'closed'`, ed è la ragione per cui esiste: l'indice
    /// `tickets_open_ix` è **parziale** su quell'elenco esatto, e una disuguaglianza
    /// non lo userebbe. È l'elenco che la dashboard del proprietario legge.
    pub open: Option<bool>,
    pub after: Option<TicketId>,
    pub limit: u16,
}

impl TicketQuery {
    pub const DEFAULT_LIMIT: u16 = 25;
    pub const MAX_LIMIT: u16 = 100;
}

impl Default for TicketQuery {
    fn default() -> Self {
        Self {
            property_id: None,
            status: None,
            open: None,
            after: None,
            limit: Self::DEFAULT_LIMIT,
        }
    }
}

#[derive(Debug)]
pub struct TicketPage {
    pub items: Vec<Ticket>,
    pub next_cursor: Option<TicketId>,
}

/// Segnalazioni.
///
/// Nessun `soft_delete`: una segnalazione non si archivia, si **annulla** — e
/// l'annullamento è una transizione di stato, quindi passa da `update`. La colonna
/// `deleted_at` esiste sulla tabella e in questa fetta nessuno la valorizza.
#[async_trait]
pub trait TicketRepository: Send {
    async fn find_by_id(&mut self, id: TicketId) -> RepositoryResult<Option<Ticket>>;
    async fn list(&mut self, query: &TicketQuery) -> RepositoryResult<TicketPage>;

    /// Inserisce la segnalazione e **restituisce il riferimento** che il database
    /// le ha assegnato.
    ///
    /// È l'unico `insert` del progetto con un valore di ritorno, e non è una
    /// comodità: `tickets.reference` ha come `DEFAULT`
    /// `'TK-' || to_char(nextval('app.ticket_reference_seq'), 'FM000000')`, quindi
    /// il valore esiste solo dopo la scrittura. Chi chiama passa il risultato a
    /// `Ticket::attach_reference`.
    ///
    /// Generarlo nell'applicazione non è la scorciatoia che sembra: metterebbe il
    /// codice in competizione con la sequenza, e il primo `INSERT` che non lo
    /// specificasse la farebbe avanzare comunque, producendo collisioni
    /// sull'indice unico `tickets_reference_uk` a distanza di mesi.
    ///
    /// **Non lo recupera un `RETURNING`**, che sotto Row Level Security fallisce:
    /// l'implementazione rilegge la riga in uno statement separato della stessa
    /// transazione, e il perché sta nel commento del repository.
    async fn insert(&mut self, ticket: &Ticket) -> RepositoryResult<TicketReference>;

    /// Aggiorna le sole colonne che l'applicazione può scrivere.
    ///
    /// L'elenco non è discrezionale: coincide con la `GRANT UPDATE (…)` della
    /// migration `0020`, e una colonna aggiunta qui e non lì fallisce con
    /// `permission denied` nei test di integrazione, che girano come
    /// `propflow_app`.
    async fn update(&mut self, ticket: &Ticket) -> RepositoryResult<()>;

    /// Storico delle transizioni, dalla più vecchia alla più recente.
    ///
    /// Solo lettura, e non per prudenza: lo scrive il trigger
    /// `tg_ticket_status_history`, e `app.ticket_status_history` non ha una policy
    /// di `INSERT`. Un metodo di scrittura su questa porta inviterebbe a fare ciò
    /// che il database rifiuta.
    async fn list_status_history(
        &mut self,
        ticket_id: TicketId,
    ) -> RepositoryResult<Vec<TicketStatusChange>>;
}

/// Filtro di elenco dei messaggi di una segnalazione, con paginazione keyset.
///
/// Il filtro sulla segnalazione non è opzionale: un elenco di *tutti* i messaggi
/// dell'organizzazione non è un'operazione che l'API espone.
#[derive(Debug, Clone)]
pub struct TicketMessageQuery {
    pub ticket_id: TicketId,
    pub after: Option<TicketMessageId>,
    pub limit: u16,
}

impl TicketMessageQuery {
    pub const DEFAULT_LIMIT: u16 = 50;
    pub const MAX_LIMIT: u16 = 100;

    #[must_use]
    pub const fn for_ticket(ticket_id: TicketId) -> Self {
        Self {
            ticket_id,
            after: None,
            limit: Self::DEFAULT_LIMIT,
        }
    }
}

#[derive(Debug)]
pub struct TicketMessagePage {
    pub items: Vec<TicketMessage>,
    pub next_cursor: Option<TicketMessageId>,
}

/// Thread di conversazione di una segnalazione.
///
/// **Nessun `update` e nessun `delete`, nemmeno soft.** `app.ticket_messages` ha
/// due sole policy, `_select` e `_insert`: un messaggio scritto resta com'è. Non è
/// una svista da correggere di passaggio — un thread in cui le righe non si
/// riscrivono è una proprietà — ma qualunque metodo qui che prometta di
/// modificarne uno starebbe promettendo ciò che il database rifiuta.
///
/// Le note interne **non** hanno una porta a parte: `is_internal` è un campo del
/// messaggio, e la policy di SELECT lo filtra per l'inquilino. Due tabelle
/// avrebbero due thread da fondere in lettura e due punti dove sbagliare l'ordine.
#[async_trait]
pub trait TicketMessageRepository: Send {
    async fn insert(&mut self, message: &TicketMessage) -> RepositoryResult<()>;
    async fn list_for_ticket(
        &mut self,
        query: &TicketMessageQuery,
    ) -> RepositoryResult<TicketMessagePage>;
}

/// Filtro di elenco delle bollette con paginazione keyset.
#[derive(Debug, Clone)]
pub struct UtilityBillQuery {
    pub property_id: Option<PropertyId>,
    pub lease_id: Option<LeaseId>,
    pub status: Option<BillStatus>,
    pub utility_type: Option<UtilityType>,
    pub after: Option<UtilityBillId>,
    pub limit: u16,
}

impl UtilityBillQuery {
    pub const DEFAULT_LIMIT: u16 = 25;
    pub const MAX_LIMIT: u16 = 100;
}

impl Default for UtilityBillQuery {
    fn default() -> Self {
        Self {
            property_id: None,
            lease_id: None,
            status: None,
            utility_type: None,
            after: None,
            limit: Self::DEFAULT_LIMIT,
        }
    }
}

#[derive(Debug)]
pub struct UtilityBillPage {
    pub items: Vec<UtilityBill>,
    pub next_cursor: Option<UtilityBillId>,
}

/// Bollette delle utenze.
///
/// Nessun `soft_delete` separato: annullare una bolletta la **cancella** — vedi
/// [`UtilityBill::void`], che valorizza `deleted_at` insieme allo stato — e resta
/// quindi una transizione, che passa da `update`. Farne un metodo a parte
/// permetterebbe di cancellare una riga senza portarla a `void`, cioè di
/// nasconderla lasciandola dichiarare di essere valida.
#[async_trait]
pub trait UtilityBillRepository: Send {
    async fn find_by_id(&mut self, id: UtilityBillId) -> RepositoryResult<Option<UtilityBill>>;
    async fn list(&mut self, query: &UtilityBillQuery) -> RepositoryResult<UtilityBillPage>;
    async fn insert(&mut self, bill: &UtilityBill) -> RepositoryResult<()>;

    /// Aggiorna le colonne che l'applicazione può scrivere.
    ///
    /// A differenza di [`ChargeRepository::update`] l'elenco **non** è ristretto
    /// da una `GRANT` di colonna, ed è una scelta: una bolletta si digita, dodici
    /// campi alla volta, e finché non è validata correggerne uno è il lavoro
    /// previsto. La `GRANT` dovrebbe nominare quasi ogni colonna, e un elenco che
    /// contiene quasi tutto non difende niente — si pagherebbe l'onere di
    /// manutenzione senza comprare la proprietà per cui l'onere esisteva.
    /// L'immutabilità della bolletta è quindi un'invariante di **dominio**
    /// (`UtilityBill::apply` rifiuta da `allocated` in poi) e non un privilegio.
    async fn update(&mut self, bill: &UtilityBill) -> RepositoryResult<()>;
}

/// I job AI **visti da una richiesta**: si accodano e si leggono, non si toccano.
///
/// La forma non è una scelta di stile: è la migration `0031`, che concede al ruolo
/// applicativo l'`INSERT` con `status = 'queued'` nella `WITH CHECK` e **nessun
/// `UPDATE`**. Un metodo di aggiornamento qui sarebbe un metodo che la Row Level
/// Security rifiuta sempre — e lo farebbe in silenzio, con zero righe aggiornate,
/// perché la mancanza di una policy nega senza sollevare. Chi scrive l'esito di un
/// job è il worker, attraverso [`AiJobQueue`].
#[async_trait]
pub trait AiJobRepository: Send {
    /// Accoda un job. Va chiamata **nella stessa transazione** che registra il
    /// documento da leggere: un job che puntasse a un documento la cui riga non è
    /// stata committata è un job che fallirebbe più tardi, nel worker, dove
    /// l'errore non ha più niente a che vedere con la sua causa.
    async fn enqueue(&mut self, job: &AiJob) -> RepositoryResult<()>;

    /// L'ultimo job accodato su un soggetto, se ce n'è uno.
    ///
    /// «L'ultimo» e non «tutti»: la rotta di stato risponde alla domanda «a che
    /// punto è la lettura di questa bolletta», che ha una risposta sola. Lo
    /// storico dei tentativi resta in tabella e si guarderà quando il ritentativo
    /// esisterà. Usa `ai_jobs_subject_ix`, che è ordinato
    /// `(subject_type, subject_id, created_at DESC)` apposta.
    async fn find_latest_for_subject(
        &mut self,
        subject: AttachmentOwner,
    ) -> RepositoryResult<Option<AiJob>>;
}

/// La coda dei job **vista dal worker**, che è l'unico a poterla far avanzare.
///
/// Sta in un trait separato da [`AiJobRepository`] e non fra i metodi di
/// [`UnitOfWork`](super::UnitOfWork) di proposito: si ottiene solo da
/// [`WorkerUnitOfWork`], cioè da una transazione che gira come `propflow_worker`
/// con `BYPASSRLS` e **senza** contesto di sicurezza. Il tipo dice quindi da sé
/// con quale postura si sta parlando al database, che è la proprietà per cui la
/// regola 4 di `CLAUDE.md` esiste — non «un solo modo di raggiungere il
/// database», ma «nessun modo di raggiungerlo senza dichiarare chi si è».
#[async_trait]
pub trait AiJobQueue: Send {
    /// Prende in carico il job più vecchio in coda per questo compito, se c'è.
    ///
    /// Usa `FOR UPDATE SKIP LOCKED`: chi arriva secondo **salta** le righe già
    /// bloccate invece di aspettarle, che è ciò che rende il worker replicabile
    /// senza coordinamento esterno. Va usato anche con una replica sola —
    /// aggiungerlo dopo significa scoprire il difetto quando le repliche diventano
    /// due, cioè sotto carico.
    ///
    /// La riga torna **bloccata dalla transazione in corso**: chi chiama deve
    /// portarla a `running` e committare *subito*, senza chiamare il fornitore nel
    /// frattempo. Tenere la transazione aperta per i secondi di una chiamata di
    /// rete occuperebbe una connessione del pool per tutto quel tempo; chiuderla
    /// senza aver marcato la riga la rimetterebbe in coda per un altro worker.
    async fn claim_next(&mut self, task: AiTask) -> RepositoryResult<Option<AiJob>>;

    /// Scrive lo stato del job: la presa in carico e la chiusura passano da qui.
    async fn update(&mut self, job: &AiJob) -> RepositoryResult<()>;
}

/// Filtro di elenco degli addebiti con paginazione keyset.
#[derive(Debug, Clone)]
pub struct ChargeQuery {
    pub lease_id: Option<LeaseId>,
    pub charge_type: Option<ChargeType>,
    /// **Non accetta `overdue`**, e chi chiama deve averlo già rifiutato: quello
    /// stato non esiste in database, si deriva. Il bordo HTTP risponde `422`
    /// indicando `?outstanding=true`, come fa con `?status=expiring` sui
    /// contratti — restituire una pagina vuota sarebbe il modo peggiore di dirlo.
    pub status: Option<ChargeStatus>,
    /// `Some(true)` = solo gli addebiti ancora da incassare.
    ///
    /// Il ramo positivo enumera i tre stati di [`ChargeStatus::OUTSTANDING`]
    /// invece di scrivere una disuguaglianza, ed è la ragione per cui esiste:
    /// l'indice `charges_outstanding_ix` è **parziale** su quell'elenco esatto.
    /// È lo scadenzario, e ciò che la dashboard leggerà.
    pub outstanding: Option<bool>,
    pub after: Option<ChargeId>,
    pub limit: u16,
}

impl ChargeQuery {
    pub const DEFAULT_LIMIT: u16 = 25;
    pub const MAX_LIMIT: u16 = 100;
}

impl Default for ChargeQuery {
    fn default() -> Self {
        Self {
            lease_id: None,
            charge_type: None,
            status: None,
            outstanding: None,
            after: None,
            limit: Self::DEFAULT_LIMIT,
        }
    }
}

#[derive(Debug)]
pub struct ChargeBalancePage {
    pub items: Vec<ChargeBalance>,
    pub next_cursor: Option<ChargeId>,
}

/// Addebiti verso l'inquilino.
///
/// **Due letture distinte, e non è una duplicazione da accorpare.** Il saldo si
/// legge da `app.v_charge_balances`, che è `security_invoker` e applica quindi la
/// RLS del chiamante; copiarne la `LEFT JOIN LATERAL` dentro il repository sarebbe
/// una seconda definizione di «pagato» e divergerebbe alla prima modifica — il
/// giorno in cui il modulo dei pagamenti aggiungerà uno stato all'array
/// `('succeeded','partially_refunded')` deve cambiare in un posto solo.
///
/// Il prezzo è che `app.charges` viene attraversata due volte quando servono anche
/// le colonne che la vista non porta (`description`, `period_*`, `issued_at`,
/// `utility_bill_id`, `notes`). Da qui la separazione: [`Self::find_by_id`] legge
/// la sola tabella ed è ciò che usano le transizioni, mentre
/// [`Self::find_balance_by_id`] e [`Self::list_balances`] uniscono la vista e sono
/// ciò che usa il bordo HTTP. **Le scritture non pagano un join che non
/// useranno.**
///
/// Nessun `soft_delete`: un addebito non si cancella, si **annulla** — e resta
/// visibile a chi doveva pagarlo, che è precisamente il punto.
#[async_trait]
pub trait ChargeRepository: Send {
    /// Legge la sola riga di `app.charges`, senza saldo. È ciò che serve alle
    /// transizioni, che non guardano quanto è stato incassato.
    async fn find_by_id(&mut self, id: ChargeId) -> RepositoryResult<Option<Charge>>;

    /// Legge l'addebito **con il saldo**, unendo `app.v_charge_balances`.
    async fn find_balance_by_id(&mut self, id: ChargeId)
    -> RepositoryResult<Option<ChargeBalance>>;

    async fn list_balances(&mut self, query: &ChargeQuery) -> RepositoryResult<ChargeBalancePage>;

    /// Inserisce l'addebito.
    ///
    /// Chi chiama **deve** aver già verificato che il contratto sia visibile: la
    /// `WITH CHECK` di `charges_insert` nomina il solo `org_id` e non chiama
    /// `can_access_lease`, e su `app.charges` non esiste un trigger
    /// `assert_org_matches_*`. Senza quel controllo si scrive una riga che nessuno
    /// potrà rileggere né cancellare, e che non comparirà in nessun saldo.
    async fn insert(&mut self, charge: &Charge) -> RepositoryResult<()>;

    /// Aggiorna la **sola colonna `status`**.
    ///
    /// Non è una restrizione di stile: la migration `0023` revoca l'`UPDATE` su
    /// `app.charges` e lo riconcede sulla sola `status`, perché la `WITH CHECK` di
    /// `charges_update` nomina il solo `org_id` e lascia libera ogni altra colonna
    /// (ADR-021, ADR-026). Una `UPDATE` che nominasse `amount` fallirebbe a
    /// runtime con `permission denied`, ed è la difesa che rende impossibile
    /// riscrivere una pretesa già comunicata.
    async fn update_status(&mut self, charge: &Charge) -> RepositoryResult<()>;
}

/// Filtro di elenco dei pagamenti, con paginazione keyset.
///
/// # `charge_id` è **opzionale**, e non lo era
///
/// La fetta contabile l'aveva obbligatorio, con una ragione scritta: «i pagamenti
/// di un addebito sono pochi, e un filtro su un elenco che sta in una pagina non
/// compra niente». Resta vero per l'elenco **di un addebito**, e falso per la
/// domanda che il modulo dei pagamenti pone dal lato di chi amministra: *quali
/// dichiarazioni aspettano una conferma, in tutta l'organizzazione*. Senza una
/// collezione quella domanda non ha risposta, e una dichiarazione si scopre solo
/// aprendo gli addebiti a uno a uno.
///
/// La rotta annidata continua a passare l'addebito valorizzato, quindi il suo
/// comportamento non cambia: è lo stesso oggetto con un campo in più riempito.
///
/// # Nessun `org_id`, e non è una dimenticanza
///
/// A restringere all'organizzazione è `payments_select`, che è
/// `org_id = current_org_id()` più i due rami su pagante e incassante. Un filtro
/// applicativo ripeterebbe la policy e darebbe l'impressione che la query sia
/// sicura per conto proprio — è la stessa scelta di [`ExpenseGroupQuery`] e di
/// [`TicketQuery`].
///
/// Ne segue la conseguenza che vale la pena dire ad alta voce: **un inquilino che
/// chiami la collezione ottiene i propri pagamenti**, non un `403`. A decidere
/// quali righe siano sue è la policy, non un ramo sul ruolo.
#[derive(Debug, Clone)]
pub struct PaymentQuery {
    /// L'addebito, quando l'elenco è il suo. `None` è la collezione.
    pub charge_id: Option<ChargeId>,
    /// Lo stato, per esteso.
    ///
    /// **Il valore che serve è `pending`**, ed è la coda delle dichiarazioni.
    /// L'indice `payments_pending_ix` della migration `0032` è parziale su quel
    /// solo valore, quindi la query deve nominarlo: una disuguaglianza non lo
    /// userebbe. È la stessa forma di `?outstanding=true` sugli addebiti.
    pub status: Option<PaymentStatus>,
    /// Il contratto degli addebiti che i pagamenti saldano.
    ///
    /// **Non è una colonna di `app.payments`**: il legame passa da
    /// `payable_id → app.charges.lease_id`, quindi il filtro è un `EXISTS` e non
    /// un confronto. Esiste perché la domanda «chi mi deve dei soldi su questo
    /// contratto» si pone dalla scheda del contratto, dove l'addebito non è ancora
    /// scelto.
    pub lease_id: Option<LeaseId>,
    pub after: Option<PaymentId>,
    pub limit: u16,
}

impl PaymentQuery {
    pub const DEFAULT_LIMIT: u16 = 25;
    pub const MAX_LIMIT: u16 = 100;
}

impl Default for PaymentQuery {
    fn default() -> Self {
        Self {
            charge_id: None,
            status: None,
            lease_id: None,
            after: None,
            limit: Self::DEFAULT_LIMIT,
        }
    }
}

#[derive(Debug)]
pub struct PaymentPage {
    pub items: Vec<Payment>,
    pub next_cursor: Option<PaymentId>,
}

/// Pagamenti.
///
/// **Nessun `soft_delete`, e nessun filtro `deleted_at` nelle query**:
/// `app.payments` non ha quella colonna — verificato, non dedotto — e il ruolo
/// applicativo non ha `DELETE`. Un pagamento sbagliato si corregge muovendone lo
/// stato. Copiare il pattern `AND deleted_at IS NULL` da un altro repository dà un
/// errore di colonna inesistente: fastidioso e innocuo, perché fallisce subito.
#[async_trait]
pub trait PaymentRepository: Send {
    async fn find_by_id(&mut self, id: PaymentId) -> RepositoryResult<Option<Payment>>;

    /// Cerca un pagamento già registrato con la stessa chiave di idempotenza.
    ///
    /// **Va chiamata prima di [`Self::insert`], sempre.** È ciò che permette a una
    /// chiave ripetuta di ottenere `200` con il pagamento già registrato invece di
    /// inserirne un secondo: senza, un doppio invio farebbe scendere il saldo due
    /// volte, e `audit_log` mostrerebbe due inserimenti perfettamente legittimi.
    ///
    /// La ricerca gira **sotto la RLS del chiamante**, e ne segue un caso residuo:
    /// un inquilino non vede i pagamenti di cui non è il pagante, quindi una chiave
    /// usata da qualcun altro non si trova qui e l'`INSERT` che segue viola
    /// `payments_idempotency_uk`, dando un conflitto. È il solo `409` che questa
    /// via può produrre, ed è irriducibile: distinguere «l'ho già mandato io» da
    /// «qualcun altro ha usato la mia chiave» richiederebbe di leggere una riga che
    /// al chiamante non spetta. Con chiavi generate dal client — UUID o ULID — la
    /// collisione è teorica.
    async fn find_by_idempotency_key(
        &mut self,
        key: &IdempotencyKey,
    ) -> RepositoryResult<Option<Payment>>;

    /// Elenca i pagamenti che il filtro seleziona, dentro il perimetro della
    /// policy. Senza `charge_id` è la collezione dell'organizzazione.
    async fn list(&mut self, query: &PaymentQuery) -> RepositoryResult<PaymentPage>;

    /// Somma dei pagamenti che **contano nel saldo** di un addebito.
    ///
    /// I due stati sono quelli di `app.v_charge_balances`
    /// (`'succeeded','partially_refunded'`) e **non** i tre di
    /// `payments_payable_ix`, che comprende anche `authorized`: contare denaro
    /// autorizzato e non incassato farebbe scendere l'insoluto su un incasso che
    /// non è arrivato.
    ///
    /// Restituisce un [`Decimal`](rust_decimal::Decimal) e non un
    /// [`Money`](crate::shared::Money) per la stessa ragione per cui
    /// `Charge::is_past_due` prende un `Decimal`: è un totale, non l'importo di una
    /// riga, e la valuta è una sola.
    async fn sum_succeeded_for_charge(
        &mut self,
        charge_id: ChargeId,
    ) -> RepositoryResult<rust_decimal::Decimal>;

    async fn insert(&mut self, payment: &Payment) -> RepositoryResult<()>;

    /// Aggiorna le sole colonne che l'applicazione scrive dopo l'inserimento:
    /// `status`, `paid_at` e `reconciled_at`.
    ///
    /// L'elenco non è una scelta di stile: la migration `0025` revoca l'`UPDATE` su
    /// `app.payments` e lo riconcede su quelle tre, perché la `WITH CHECK` di
    /// `payments_update` nomina il solo `org_id` e lascia libera ogni altra colonna
    /// (ADR-021, ADR-026). Importo, pagante, metodo e chiave di idempotenza **non
    /// hanno motivo di cambiare** dopo l'inserimento, e una `UPDATE` che li
    /// nominasse fallirebbe a runtime con `permission denied`.
    async fn update_status(&mut self, payment: &Payment) -> RepositoryResult<()>;
}

// ---------------------------------------------------------------------------
// Spese co-living
//
// **Nessun `soft_delete` sui gruppi, e nessun filtro `deleted_at` su tre tabelle
// su quattro.** `app.expense_group_members`, `app.expense_shares` e
// `app.settlements` non hanno quella colonna — verificato, non dedotto — mentre
// `app.expense_groups` e `app.expenses` sì. Copiare il pattern
// `AND deleted_at IS NULL` da un altro repository sulle tre che non ce l'hanno dà
// un errore di colonna inesistente: fastidioso e innocuo, perché fallisce subito.
// ---------------------------------------------------------------------------

/// Filtro di elenco dei gruppi di spesa con paginazione keyset.
///
/// **Non c'è un filtro sul membro**, e non è una dimenticanza: a restringere i
/// gruppi a quelli di chi chiede è già `expense_groups_select`, che passa da
/// `can_access_expense_group`. Un filtro applicativo ripeterebbe la policy e
/// darebbe l'impressione che la query sia sicura per conto proprio.
#[derive(Debug, Clone)]
pub struct ExpenseGroupQuery {
    pub property_id: Option<PropertyId>,
    /// `Some(false)` = solo i gruppi ancora aperti. Serve l'indice
    /// `expense_groups_org_ix`, che è su `(org_id, is_archived)`.
    pub is_archived: Option<bool>,
    pub after: Option<ExpenseGroupId>,
    pub limit: u16,
}

impl ExpenseGroupQuery {
    pub const DEFAULT_LIMIT: u16 = 25;
    pub const MAX_LIMIT: u16 = 100;
}

impl Default for ExpenseGroupQuery {
    fn default() -> Self {
        Self {
            property_id: None,
            is_archived: None,
            after: None,
            limit: Self::DEFAULT_LIMIT,
        }
    }
}

#[derive(Debug)]
pub struct ExpenseGroupPage {
    pub items: Vec<ExpenseGroup>,
    pub next_cursor: Option<ExpenseGroupId>,
}

/// Gruppi di spesa e loro composizione.
///
/// Le due cose stanno in un repository solo perché una membership non ha vita
/// propria: si legge sempre attraverso il gruppo che la contiene, dove il numero
/// di righe è per costruzione piccolo. È la stessa scelta di
/// [`RoomRepository`].
#[async_trait]
pub trait ExpenseGroupRepository: Send {
    async fn find_by_id(&mut self, id: ExpenseGroupId) -> RepositoryResult<Option<ExpenseGroup>>;

    async fn list(&mut self, query: &ExpenseGroupQuery) -> RepositoryResult<ExpenseGroupPage>;

    /// Inserisce il gruppo.
    ///
    /// **Senza `RETURNING`, e stavolta il muro c'è davvero.**
    /// `expense_groups_select` chiama `can_access_expense_group(id)`, cioè
    /// interroga la riga che si sta inserendo, e la funzione è `STABLE`: vede lo
    /// snapshot di inizio statement, in cui la riga non c'è. È il muro già
    /// incontrato sulle segnalazioni, e la regola generale vale ancora — il muro
    /// c'è solo quando la policy di `SELECT` dipende dalla riga stessa. Sugli
    /// addebiti non c'era, perché `charges_select` interroga `app.leases`.
    async fn insert(&mut self, group: &ExpenseGroup) -> RepositoryResult<()>;

    /// Aggiorna le sole colonne che il dominio muove sul gruppo: `is_archived`.
    async fn update(&mut self, group: &ExpenseGroup) -> RepositoryResult<()>;

    /// I membri del gruppo.
    ///
    /// `active_only` distingue le due domande, che sono diverse: la ripartizione
    /// vuole i **membri attivi**, mentre la lettura della composizione vuole anche
    /// chi è uscito — altrimenti un client non saprebbe spiegare le quote di una
    /// spesa vecchia.
    async fn list_members(
        &mut self,
        group_id: ExpenseGroupId,
        active_only: bool,
    ) -> RepositoryResult<Vec<ExpenseGroupMember>>;

    /// La membership di una persona, **attiva o no**.
    ///
    /// Restituisce anche la riga di chi è uscito, ed è ciò che permette a `join`
    /// di distinguere un ingresso da un rientro:
    /// `expense_group_members_group_user_uk` è unico su `(group_id, user_id)` e
    /// **non è parziale**, quindi chi rientra non può avere una seconda riga.
    async fn find_member(
        &mut self,
        group_id: ExpenseGroupId,
        user_id: UserId,
    ) -> RepositoryResult<Option<ExpenseGroupMember>>;

    async fn insert_member(&mut self, member: &ExpenseGroupMember) -> RepositoryResult<()>;

    /// Aggiorna una membership: uscita, rientro, peso.
    async fn update_member(&mut self, member: &ExpenseGroupMember) -> RepositoryResult<()>;

    /// I saldi del gruppo, letti da `app.v_expense_group_balances`.
    ///
    /// **Si leggono e non si ricalcolano**, per la ragione di ADR-032: due
    /// definizioni dello stesso numero si contraddicono al primo caso limite.
    /// Dalla migration `0027` l'elenco comprende chi è uscito con un conto aperto.
    async fn list_balances(
        &mut self,
        group_id: ExpenseGroupId,
    ) -> RepositoryResult<Vec<GroupBalance>>;
}

/// Filtro di elenco delle spese di un gruppo, con paginazione keyset.
#[derive(Debug, Clone)]
pub struct ExpenseQuery {
    pub group_id: ExpenseGroupId,
    pub category: Option<crate::expense::ExpenseCategory>,
    pub after: Option<ExpenseId>,
    pub limit: u16,
}

impl ExpenseQuery {
    pub const DEFAULT_LIMIT: u16 = 25;
    pub const MAX_LIMIT: u16 = 100;
}

#[derive(Debug)]
pub struct ExpensePage {
    pub items: Vec<Expense>,
    pub next_cursor: Option<ExpenseId>,
}

/// Spese e loro quote.
#[async_trait]
pub trait ExpenseRepository: Send {
    async fn find_by_id(&mut self, id: ExpenseId) -> RepositoryResult<Option<Expense>>;

    async fn list_for_group(&mut self, query: &ExpenseQuery) -> RepositoryResult<ExpensePage>;

    /// Inserisce la spesa **e le sue quote**, nella stessa transazione.
    ///
    /// Un metodo solo e non due, e non è una comodità: il trigger
    /// `expense_shares_balanced` è `DEFERRABLE INITIALLY DEFERRED` e verifica al
    /// `COMMIT` che la somma delle quote sia esattamente l'importo. Due metodi
    /// separati renderebbero rappresentabile una spesa senza quote, che è uno
    /// stato che il database rifiuta — e lo rifiuterebbe alla fine della
    /// transazione, dove l'errore non ha più niente a che vedere con la query che
    /// lo ha causato.
    ///
    /// **L'errore del trigger esce da `commit()`, non da qui.** Chi chiama deve
    /// mapparlo là.
    async fn insert(&mut self, expense: &Expense, shares: &[ExpenseShare]) -> RepositoryResult<()>;

    /// Le quote di una spesa.
    async fn list_shares(&mut self, expense_id: ExpenseId) -> RepositoryResult<Vec<ExpenseShare>>;
}

/// Filtro di elenco dei regolamenti di un gruppo, con paginazione keyset.
#[derive(Debug, Clone)]
pub struct SettlementQuery {
    pub group_id: ExpenseGroupId,
    /// Serve l'indice parziale `settlements_pending_ix`, che è su
    /// `(to_user_id, created_at DESC) WHERE status = 'pending'`.
    pub status: Option<SettlementStatus>,
    pub after: Option<SettlementId>,
    pub limit: u16,
}

impl SettlementQuery {
    pub const DEFAULT_LIMIT: u16 = 25;
    pub const MAX_LIMIT: u16 = 100;
}

#[derive(Debug)]
pub struct SettlementPage {
    pub items: Vec<Settlement>,
    pub next_cursor: Option<SettlementId>,
}

/// Regolamenti fra coinquilini.
///
/// **Nessun `soft_delete` e nessun filtro `deleted_at`**: `app.settlements` non ha
/// quella colonna, e il ruolo applicativo non ha `DELETE`. Un regolamento
/// sbagliato si ritira muovendone lo stato.
#[async_trait]
pub trait SettlementRepository: Send {
    async fn find_by_id(&mut self, id: SettlementId) -> RepositoryResult<Option<Settlement>>;

    async fn list_for_group(&mut self, query: &SettlementQuery)
    -> RepositoryResult<SettlementPage>;

    async fn insert(&mut self, settlement: &Settlement) -> RepositoryResult<()>;

    /// Aggiorna le sole colonne che le tre transizioni scrivono: `status`,
    /// `settled_at`, `confirmed_at` e `confirmed_by`.
    ///
    /// **Su `app.settlements` non ci sono privilegi di colonna**, a differenza di
    /// `app.payments`: l'elenco qui è una scelta del repository e non un vincolo
    /// del database. Nominare `amount` non fallirebbe — passerebbe, ed è la
    /// ragione per cui l'elenco va scritto corto e lasciato corto.
    async fn update_status(&mut self, settlement: &Settlement) -> RepositoryResult<()>;
}

/// Destinazione degli eventi di dominio: l'outbox, scritta nella stessa
/// transazione del comando.
#[async_trait]
pub trait EventSink: Send {
    async fn append(&mut self, event: &DomainEvent) -> RepositoryResult<()>;
}

/// La lettura che attraversa tutti i moduli insieme.
///
/// **Una porta a sé e non un metodo su `OrganizationRepository`**: quella legge
/// l'aggregato organizzazione, questa legge una proiezione che di
/// `app.organizations` usa la sola chiave. Metterle insieme farebbe sembrare la
/// dashboard un attributo dell'organizzazione, e il giorno in cui arrivasse una
/// seconda proiezione — quella dell'inquilino — non ci sarebbe un posto ovvio dove
/// metterla.
#[async_trait]
pub trait DashboardRepository: Send {
    /// I nove numeri dell'organizzazione indicata.
    ///
    /// `None` significa che la riga **non è visibile al chiamante**: la vista è
    /// `security_invoker` e passa da `organizations_select`, cioè
    /// `app.is_org_member(id)`. Non significa «organizzazione inesistente», e le
    /// due cose sotto Row Level Security sono giustamente indistinguibili.
    async fn owner_dashboard(&mut self, org_id: OrgId) -> RepositoryResult<Option<OwnerDashboard>>;
}
