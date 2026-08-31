use async_trait::async_trait;

use crate::billing::{
    BillStatus, Charge, ChargeBalance, ChargeStatus, ChargeType, UtilityBill, UtilityType,
};
use crate::document::{AttachmentOwner, Document};
use crate::events::DomainEvent;
use crate::identity::{Invitation, OrgMember, Organization, RefreshSession, TokenHash, User};
use crate::lease::{Lease, LeaseParty, LeaseStatus};
use crate::property::{Property, PropertyStatus, Room};
use crate::shared::{
    ChargeId, DocumentId, Email, InvitationId, LeaseId, OrgId, PropertyId, RoomId, TicketId,
    TicketMessageId, UserId, UtilityBillId,
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

/// Destinazione degli eventi di dominio: l'outbox, scritta nella stessa
/// transazione del comando.
#[async_trait]
pub trait EventSink: Send {
    async fn append(&mut self, event: &DomainEvent) -> RepositoryResult<()>;
}
