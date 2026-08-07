# 02 — Architettura

## Il principio

Il dominio non conosce il mondo esterno. Dichiara ciò che gli serve — *porte* — e
qualcun altro lo fornisce. Ne segue che PostgreSQL, Axum, Argon2 e il provider AI
sono dettagli sostituibili, e che le regole di business si possono testare senza
avviare nulla.

Questo vale in molti progetti come intenzione. Qui è **verificato dal
compilatore**, perché i livelli sono crate separati e un crate non può usare ciò
che non ha fra le dipendenze.

## Struttura

```
crates/domain           propflow-domain          entità, value object, eventi, porte di persistenza
crates/application     propflow-application     use case, autorizzazione, porte tecniche
crates/infrastructure  propflow-infrastructure  SQLx, Argon2id, EdDSA, telemetria
crates/api        [bin] propflow-api             Axum, DTO, OpenAPI, modello di errore HTTP
crates/worker     [bin] propflow-worker          dispatcher dell'outbox
crates/migrator   [bin] propflow-migrator        migration dello schema
```

Direzione delle dipendenze, unica ammessa:

```
api ────────► application ────────► domain
 │                                    ▲
 └──► infrastructure ─────────────────┘   (implementa le porte)
```

`api` dipende da `infrastructure` **solo in `main.rs`**, dove costruisce le
implementazioni concrete e le inietta come `Arc<dyn Port>`. Gli handler vedono
soltanto tipi di `application`.

Il controllo è in `ops/ci/check-domain-purity.sh`, eseguito dalla pipeline: se
`propflow-domain` acquisisce `sqlx`, `axum`, `tokio` o un client HTTP, la build
fallisce.

### Cosa può dipendere da cosa

| Crate | Dipendenze ammesse |
|---|---|
| `domain` | `uuid`, `chrono`, `rust_decimal`, `thiserror`, `async-trait`, `serde` (solo derive) |
| `application` | `domain` + le stesse più `tracing` |
| `infrastructure` | `domain`, `application`, e qualunque libreria tecnica |
| `api` | `domain`, `application`, `infrastructure` (solo `main.rs`), Axum e affini |
| `worker`, `migrator` | `infrastructure` e librerie tecniche |

`serde` è ammesso nel dominio perché gli eventi sono contratti verso l'esterno e
hanno bisogno di serializzarsi. `serde_json` no: nel dominio non esiste JSON
dinamico.

## Il pezzo che tiene tutto: Unit of Work e contesto RLS

Ogni riga del database è protetta da Row Level Security, che si attiva solo se la
transazione ha impostato il contesto di sicurezza. Se una query gira senza
contesto, non vede nulla — o, peggio, se qualcuno aggiungesse una via alternativa
al database, vedrebbe tutto.

La soluzione non è una convenzione ma un tipo:

```rust
#[async_trait]
pub trait UnitOfWorkFactory: Send + Sync {
    async fn begin(&self, context: &RequestContext) -> RepositoryResult<Box<dyn UnitOfWork>>;
    async fn begin_privileged(&self) -> RepositoryResult<Box<dyn UnitOfWork>>;
}

#[async_trait]
pub trait UnitOfWork: Send {
    fn users(&mut self) -> &mut dyn UserRepository;
    fn properties(&mut self) -> &mut dyn PropertyRepository;
    fn events(&mut self) -> &mut dyn EventSink;
    // …
    async fn commit(self: Box<Self>) -> RepositoryResult<()>;
}
```

I repository **si ottengono solo da qui**, e `begin` esegue
`app.set_request_context(...)` come primo statement della transazione. Non esiste
un modo di avere un repository senza aver prima passato un `RequestContext`:
dimenticare il contesto non è un bug possibile, è codice che non compila.

Senza `commit` esplicito, il `Drop` della transazione fa rollback.

### Perché un solo tipo implementa tutte le porte

`PgUnitOfWork` implementa `UserRepository`, `PropertyRepository`, `EventSink` e le
altre. Non è pigrizia: i repository devono condividere la **stessa** transazione,
e tipi separati dovrebbero prenderla in prestito contemporaneamente, cosa che il
borrow checker rifiuta. Con un solo tipo, `properties()` restituisce `self` visto
come `&mut dyn PropertyRepository`, e il prestito resta uno per volta.

## I due pool

`DatabasePools` tiene due pool distinti, con ruoli PostgreSQL diversi:

| Pool | Ruolo | Proprietà |
|---|---|---|
| `application` | `propflow_app` | soggetto a RLS, nessun privilegio `DELETE` |
| `privileged` | `propflow_worker` | `BYPASSRLS` |

`begin()` usa il primo, `begin_privileged()` il secondo. Il secondo esiste per un
solo motivo: registrazione, login e rotazione del refresh token avvengono **prima**
dell'autenticazione, quando non esiste un contesto da cui la RLS possa dedurre una
visibilità — la policy su `users` mostra una riga solo a chi è già identificato, e
prima di autenticare non si può leggere l'utente da autenticare.

Tenerli separati rende un accesso privilegiato visibile nel codice come
`begin_privileged()`, invece di nasconderlo in una connessione che ha sempre tutti
i permessi. **Ogni altro uso del pool privilegiato è un errore di progetto.**

## Autorizzazione a due livelli

`application::authorization` offre `ensure_authenticated`, `ensure_member`,
`ensure_staff`. Duplicano quello che la RLS già impone, di proposito:

- la **RLS è l'autorità**: non è aggirabile nemmeno da una query scritta a mano;
- il **service layer serve alla semantica**: la RLS da sola risponderebbe "zero
  righe" dove il chiamante merita un `403`, e un `404` al posto di un `403`
  nasconde un problema di permessi facendolo sembrare un bug.

Il ruolo presente nell'access token serve a rispondere `403` senza interrogare il
database. Non è autorevole: la RLS rilegge sempre la membership, quindi un token
con un ruolo gonfiato non ottiene righe in più.

## Eventi di dominio e outbox

Un evento è una **decisione del dominio**, non un effetto collaterale di una
scrittura: per questo non viene generato da un trigger. Lo produce lo use case e
lo scrive nella tabella `domain_events` **nella stessa transazione** del comando.

Ne segue una proprietà utile: non può esistere un evento senza la modifica che
descrive, né la modifica senza l'evento. È ciò che rende superfluo un two-phase
commit verso il broker.

`propflow-worker` legge la coda con `FOR UPDATE SKIP LOCKED` — più repliche
consumano la stessa coda senza coordinamento esterno — e pubblica su un trait
`EventBus` che ha una sola implementazione, `LoggingEventBus`. Il broker non è
nominato da nessuna parte: sostituirlo è aggiungere un file.

Gli eventi **non trasportano dati personali**: `UserRegistered` non contiene
l'email, `PropertyUpdated` porta i nomi dei campi modificati e non i valori. Un
evento pubblicato su un broker è un dato che viaggia e si conserva.

## Forma di uno use case

```rust
pub struct CreateProperty {
    unit_of_work: Arc<dyn UnitOfWorkFactory>,
    clock: Arc<dyn Clock>,
}

impl CreateProperty {
    pub async fn execute(
        &self,
        context: &RequestContext,
        command: CreatePropertyCommand,
    ) -> AppResult<Property> {
        let org_id = authorization::ensure_staff(context)?;   // 1. autorizza
        let property = Property::create(/* … */)?;            // 2. costruisci con le invarianti
        let mut uow = self.unit_of_work.begin(context).await?; // 3. transazione con contesto
        uow.properties().insert(&property).await?;            // 4. persisti
        uow.events().append(&event).await?;                   // 5. evento nella stessa tx
        uow.commit().await?;                                  // 6. commit
        Ok(property)
    }
}
```

Sempre questa sequenza. `Clock` è iniettato perché chiamare `Utc::now()` nel
dominio renderebbe i test dipendenti dal tempo reale: scadenze, lockout e
rotazioni si verificano solo potendo decidere "adesso".

## Dominio ricco, non anemico

Gli aggregati hanno campi privati, accessori e metodi che applicano le invarianti.
Due esempi:

- `Property::apply(changes, now)` rifiuta le modifiche su un immobile archiviato e
  restituisce `false` se non c'era nulla da cambiare, così un PATCH vuoto non
  produce scrittura, evento e riga di audit per nulla;
- `RefreshSession::ensure_usable(now)` distingue "revocata" da "già ruotata":
  presentare un token già ruotato è un `Invariant`, cioè il segnale che qualcuno
  ne ha una copia, e chi chiama deve revocare l'intera catena.

Ogni aggregato ha un costruttore `from_persistence` che **non** riesegue le
invarianti: un aggregato già salvato deve poter essere riletto anche se le regole
sono cambiate da allora.

## Aggiungere una funzionalità

L'ordine è sempre dall'interno verso l'esterno. Ricette dettagliate in
[06 — Sviluppo](06-sviluppo.md).

1. **Dominio** — entità e invarianti, con test unitari puri. Se serve un nuovo
   evento, una variante in `DomainEventPayload`: il compilatore trova i punti da
   aggiornare.
2. **Porte** — se serve una capacità nuova, un metodo su un trait esistente o un
   trait nuovo in `domain::ports` (persistenza) o `application::ports` (servizi).
3. **Use case** — orchestrazione, autorizzazione, transazione, evento.
4. **Infrastruttura** — implementazione delle porte. Query come letterali statici.
5. **Bordo HTTP** — DTO, handler sottile, annotazione `utoipa`, registrazione in
   `openapi.rs` e nel router.
6. **Test** — unitari nel dominio, di integrazione per le query e la RLS, di
   contratto per l'HTTP.
