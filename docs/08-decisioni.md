# 08 — Registro delle decisioni

Ogni voce riporta il contesto, la decisione e le conseguenze. Diverse decisioni
nascono da un difetto osservato: quando è così, il difetto è descritto, perché è
l'unica cosa che impedisce di ripeterlo.

**Prima di cambiare una scelta documentata qui, leggi perché è stata presa.**

---

<a id="adr-001"></a>

## ADR-001 — Organizzazioni come confine di isolamento

**Contesto.** L'MVP serve piccoli proprietari privati. La V3 prevede condomini,
multi-proprietario e SaaS multi-tenant. La parola "tenant" è ambigua: in PropTech è
l'inquilino, in un SaaS è il confine di isolamento.

**Decisione.** Tabella `organizations` con `org_id NOT NULL` su ogni tabella di
business. Ogni proprietario riceve un'organizzazione personale all'onboarding. Il
termine usato è sempre "organization"; "tenant" resta l'inquilino.

**Conseguenze.** La RLS è un predicato su `org_id`, gli indici hanno `org_id` in
testa, e la V3 non richiede una migrazione strutturale con backfill su ogni tabella.
Costo oggi: una join in più in qualche query, e un concetto che nell'MVP ha sempre
cardinalità uno.

**Alternativa scartata.** Isolamento derivato dai percorsi di ownership
(`property.owner_id`): più snello ora, ma la V3 avrebbe richiesto di aggiungere e
popolare `org_id` su venti tabelle già in produzione.

---

<a id="adr-002"></a>

## ADR-002 — Importi come `NUMERIC(14,2)`

**Decisione.** Dominio `app.money_amount` = `NUMERIC(14,2)` più `currency CHAR(3)`,
mappato su `rust_decimal::Decimal`. Sul filo HTTP gli importi sono **stringhe**.

**Motivo.** Aritmetica esatta nel database, viste e aggregati leggibili, e il
valore che entra è quello che esce. Un float binario non rappresenta 0.10, e in
contabilità la differenza si accumula.

**Conseguenze.** `Money::new` rifiuta più di due decimali: senza il controllo un
importo a quattro decimali verrebbe arrotondato in silenzio dal database. `Money`
non implementa `PartialOrd`, perché confrontare 10 EUR con 10 USD non è
un'operazione di dominio.

---

<a id="adr-003"></a>

## ADR-003 — Soft delete

**Decisione.** `deleted_at` sulle entità di business. Il privilegio `DELETE` **non**
è concesso al ruolo applicativo.

**Motivo.** Contratti, addebiti e audit continuano a riferirsi alle entità
cancellate e devono restare leggibili. Revocare il privilegio rende il soft delete
un vincolo e non una convenzione: un `DELETE` fallisce a runtime, non passa
inosservato in code review.

**Conseguenze.** Ogni query deve filtrare `deleted_at IS NULL` — vedi
[ADR-008](#adr-008), che spiega perché il filtro non può stare nelle policy. Indici
parziali sul vivo e unique parziali, così email e slug si liberano dopo la
cancellazione.

---

<a id="adr-004"></a>

## ADR-004 — Workspace multi-crate

**Contesto.** Il vincolo "il dominio non dipende dall'infrastruttura" e "ogni modulo
estraibile come microservizio".

**Decisione.** Sei crate con dipendenze dirette, non un crate unico con moduli.

**Motivo.** In un crate unico la separazione è una convenzione: nulla impedisce a
`domain` di scrivere `use sqlx::...`, e la violazione va cercata a mano. Fra crate
è il compilatore a impedirlo, e `ops/ci/check-domain-purity.sh` verifica anche
l'albero delle dipendenze transitive.

**Conseguenze.** Più `Cargo.toml` da mantenere (mitigato da
`[workspace.dependencies]`), e un lockfile che dipende dai crate presenti — cosa che
rende scomodo dividere il workspace fra più branch.

---

<a id="adr-005"></a>

## ADR-005 — Il contesto RLS non è aggirabile per costruzione

**Contesto.** La RLS protegge solo se la transazione ha impostato il contesto. Una
convenzione "ricordati di chiamare `set_request_context`" fallisce la prima volta
che qualcuno ha fretta.

**Decisione.** I repository si ottengono **solo** da
`UnitOfWorkFactory::begin(context)`, che installa il contesto come primo statement.
`AppState` non espone il pool.

**Conseguenze.** Dimenticare il contesto non è un bug possibile. `PgUnitOfWork`
implementa tutte le porte dei repository, perché devono condividere la stessa
transazione e tipi separati la prenderebbero in prestito contemporaneamente.

**Nota.** Le policy non si fidano comunque delle variabili di sessione: rileggono la
membership dal database. Un contesto falsificato non concede accesso, ed è coperto
da un test.

---

<a id="adr-006"></a>

## ADR-006 — Query verificate a runtime, non con le macro `query!`

**Contesto.** SQLx offre `query!`, che verifica il SQL a compile time contro un
database reale o una cache `.sqlx`.

**Decisione.** Query costruite a runtime, come **letterali statici** composti con
`concat!`, e verificate dai test di integrazione.

**Motivo.** Le macro richiedono un database raggiungibile in fase di build o una
cache da rigenerare a ogni modifica, e la aggiungono come dipendenza obbligatoria
della compilazione. Con i test di integrazione lo stesso errore viene catturato,
solo più tardi — e la CI compila senza database.

**Conseguenze.** Nessun `cargo sqlx prepare --check`. In compenso la copertura delle
query dipende dai test, che vanno scritti per ogni query nuova. La conversione alle
macro resta meccanica se in futuro si preferisse il controllo anticipato.

**Nota su sqlx 0.9.** Rifiuta le stringhe SQL non statiche a meno di dichiararle
sicure con `AssertSqlSafe`. La risposta corretta a quel vincolo è non avere SQL
dinamico, non aggirarlo: i filtri opzionali usano `($1 IS NULL OR colonna = $1)`.

---

<a id="adr-007"></a>

## ADR-007 — Enum letti con cast a `text`

**Decisione.** Gli enum PostgreSQL si leggono con `colonna::text` e si scrivono con
`$1::app.tipo_enum`, con la conversione centralizzata nella macro `string_enum!`.
Non si usa `#[derive(sqlx::Type)]`.

**Motivo.** La risoluzione degli OID dei tipi definiti dall'utente dipende dal
`search_path` e dalla cache della connessione; il cast testuale ha lo stesso
comportamento su qualunque connessione, e rende visibile nel SQL quale tipo si sta
usando. Stesso trattamento per i domini (`::numeric`, `::text`).

**Conseguenze.** Una tabella di conversione da mantenere. In cambio un valore
sconosciuto in lettura dà **errore** invece di ricadere su un default plausibile:
un enum esteso nel database e non nel codice deve essere rumoroso.

---

<a id="adr-008"></a>

## ADR-008 — Le policy RLS non filtrano il soft delete

**Difetto osservato.** L'archiviazione di un immobile rispondeva `403`. Il messaggio
di PostgreSQL era `new row violates row-level security policy for table
"properties"`, malgrado il `WITH CHECK` della policy di UPDATE non menzionasse
`deleted_at`.

**Causa.** PostgreSQL applica le policy di **SELECT anche alla riga nuova** di un
`UPDATE`: non si può scrivere una riga che poi non si potrebbe rileggere. Con
`deleted_at IS NULL` nella policy di SELECT, valorizzare `deleted_at` produce una
riga invisibile a chi la scrive. Riprodotto in isolamento: rimuovendo la condizione
dalla sola policy di SELECT, lo stesso `UPDATE` riesce.

**Causa radice.** Le policy mescolavano due responsabilità: l'isolamento dei dati
(*di chi è questa riga?*) e un filtro di business (*è ancora attiva?*). La prima è
una proprietà di sicurezza e appartiene alla RLS. La seconda, messa nella RLS, entra
in conflitto con l'operazione che la produce.

**Decisione.** Le policy di SELECT esprimono solo l'isolamento
(`migrations/0016_soft_delete_rls_fix.sql`). Il filtro sul soft delete sta nel
`WHERE` delle query e nelle viste. Le policy di UPDATE mantengono
`deleted_at IS NULL` nella `USING`, dove si applica alla riga **esistente** e
impedisce di modificare un'entità archiviata senza creare contraddizioni.

**Conseguenze.** Una query che dimenticasse il filtro vedrebbe anche le righe
archiviate. Non è una falla di isolamento — quelle righe appartengono comunque
all'organizzazione del chiamante e restano soggette al filtro per ruolo — ma è una
comodità che si perde. Il controllo 9 delle invarianti impedisce la regressione.

---

<a id="adr-009"></a>

## ADR-009 — Il migratore fissa `search_path=public`

**Difetto osservato.** Alla seconda esecuzione su un database già migrato, il
migratore riapplicava tutte le migration e falliva con
`type "money_amount" already exists`. Si sarebbe manifestato al primo redeploy in
produzione.

**Causa.** Il ruolo `propflow_migrator` ha `search_path = app, public`. SQLx crea la
propria tabella di bookkeeping con `CREATE TABLE IF NOT EXISTS _sqlx_migrations`,
senza qualificare lo schema. Alla **prima** esecuzione lo schema `app` non esiste
ancora — lo crea la migration 0001 — quindi il nome si risolve su `public` e la
tabella nasce lì. Alla **seconda** `app` esiste, il nome si risolve su `app`, la
tabella non c'è e ne viene creata una nuova, vuota: il migratore conclude che nessuna
migration è stata applicata.

**Decisione.** Il migratore fissa `search_path=public` sulla connessione, senza
affidarsi al default del ruolo. Tutte le migration qualificano gli oggetti con
`app.`, quindi il contenuto non ne è influenzato.

**Conseguenze.** Idempotenza verificata su database vuoto e già migrato, e un passo
di CI dedicato. Il controllo 10 delle invarianti segnala una `_sqlx_migrations`
fuori da `public`, che è il sintomo.

---

<a id="adr-010"></a>

## ADR-010 — Migration lette dal filesystem

**Difetto osservato.** Aggiunta la migration 0016, il binario continuava ad
applicarne 15. La macro `sqlx::migrate!` incorpora i file a compile time e non
invalida la cache di build quando ne compare uno nuovo.

**Decisione.** `Migrator::new(path)` a runtime, con la cartella configurabile
(`PROPFLOW__DATABASE__MIGRATIONS_DIR`) e copiata nell'immagine.

**Conseguenze.** L'immagine deve contenere `migrations/`: non è più un singolo
binario autosufficiente. In cambio quella classe di errore — silenziosa e con
effetti in produzione — diventa impossibile.

---

<a id="adr-011"></a>

## ADR-011 — Due pool di connessione

**Decisione.** `application` come `propflow_app` (soggetto a RLS, senza `DELETE`) e
`privileged` come `propflow_worker` (`BYPASSRLS`). `begin_privileged()` è ammesso
solo per registrazione, login e rotazione del refresh token.

**Motivo.** Quei tre flussi avvengono prima dell'autenticazione, quando non esiste
un contesto da cui la RLS possa dedurre una visibilità: la policy su `users` mostra
una riga solo a chi è già identificato, e prima di autenticare non si può leggere
l'utente da autenticare.

**Conseguenze.** Un accesso privilegiato è **visibile nel codice** come
`begin_privileged()`, invece di essere nascosto in una connessione che ha sempre
tutti i permessi. Due pool da dimensionare.

---

<a id="adr-012"></a>

## ADR-012 — EdDSA per gli access token, token opachi per il refresh

**Decisione.** Access token JWT firmati Ed25519, 15 minuti. Refresh token opachi da
256 bit, 30 giorni, hash SHA-256 in database, rotazione a uso singolo con
rilevamento del riuso.

**Motivo.** Con EdDSA un servizio può verificare i token con la sola chiave
pubblica; con HS256 ogni verificatore sarebbe anche un potenziale emittente — e il
vincolo dice che ogni modulo deve poter diventare un microservizio. Un refresh token
deve poter essere revocato, e un JWT autoconsistente resta valido fino alla scadenza
qualunque cosa accada: la fonte di verità è la riga in `user_sessions`.

**Conseguenze.** Servono chiavi da gestire e ruotare (manca un `kid`: ruotare
invalida i token in circolazione). `jsonwebtoken` 11 obbliga a scegliere un provider
crittografico: `rust_crypto`, così non serve un toolchain C nell'immagine.

---

<a id="adr-013"></a>

## ADR-013 — `Patch<T> = Option<Option<T>>`

**Decisione.** Gli aggiornamenti parziali distinguono campo assente, campo azzerato
e campo impostato.

**Motivo.** Con un `Option<T>` semplice le prime due cose sono indistinguibili, e un
PATCH non può cancellare un valore: "non inviato" finisce per significare "metti a
null", o viceversa un campo non si può più svuotare.

**Conseguenze.** Un deserializzatore dedicato al bordo e un tipo annidato nel
dominio. `PropertyChanges::changed_field_names()` alimenta l'evento
`PropertyUpdated` con i nomi dei campi toccati, e `apply` restituisce se qualcosa è
cambiato davvero, così un PATCH vuoto non produce scrittura, evento e riga di audit.

---

<a id="adr-014"></a>

## ADR-014 — Outbox transazionale, eventi prodotti dal dominio

**Decisione.** Gli eventi li scrive lo use case in `domain_events`, nella stessa
transazione del comando. Nessun trigger li genera. Il broker è dietro un trait con
una sola implementazione che scrive nei log.

**Motivo.** Un evento è una decisione del dominio, non un effetto collaterale di una
scrittura: dedurlo da un trigger sposterebbe logica di business nel database.
Scriverlo nella stessa transazione garantisce che non esista un evento senza la
modifica che descrive né la modifica senza l'evento — che è ciò che rende superfluo
un two-phase commit verso il broker.

**Conseguenze.** Serve un dispatcher che faccia polling; usa
`FOR UPDATE SKIP LOCKED`, quindi più repliche consumano la stessa coda senza
coordinamento. Gli eventi non trasportano dati personali: viaggiano e si conservano.

---

<a id="adr-015"></a>

## ADR-015 — Invarianti nel database, workflow nell'applicazione

**Criterio.** Nel database va una regola se può essere violata da due transazioni
concorrenti, o se la sua violazione corrompe i dati. Tutto il resto è workflow e
sta nel service layer.

**Nel database.** Nessuna doppia locazione sovrapposta (`EXCLUDE USING gist`: una
verifica applicativa sarebbe soggetta a race condition); somma delle quote pari
all'importo speso (constraint trigger differito); coerenza di `org_id` fra figlio e
padre; esistenza dei riferimenti polimorfici; deduplica delle bollette; idempotenza
dei pagamenti; audit immodificabile.

**Non nel database.** Lo stato di un addebito calcolato dai pagamenti: sarebbe
workflow. Il saldo si legge da `v_charge_balances`, lo stato lo scrive il service
layer. Anche "almeno un owner per organizzazione" resta applicativo, perché
richiederebbe un trigger cross-row.

---

<a id="adr-016"></a>

## ADR-016 — Autorizzazione duplicata fra RLS e service layer

**Decisione.** `authorization::ensure_staff` e simili ripetono quello che la RLS già
impone.

**Motivo.** Non è ridondanza: sono due livelli con compiti diversi. La RLS è
l'autorità e non è aggirabile nemmeno da una query scritta a mano, ma da sola
risponderebbe "zero righe" dove il chiamante merita un `403` — e un `404` al posto
di un `403` nasconde un problema di permessi facendolo sembrare un bug.

**Conseguenze.** Un ruolo va tenuto allineato in due punti. Il ruolo nell'access
token serve solo al controllo anticipato: la RLS rilegge sempre la membership, quindi
un token con un ruolo gonfiato non ottiene righe in più.

---

<a id="adr-017"></a>

## ADR-017 — Nessuna cancellazione dura, ma nemmeno un 403 informativo

**Decisione.** Una risorsa non visibile dà `404`, non `403`. Credenziali errate e
account inesistente danno lo stesso `401` con lo stesso corpo.

**Motivo.** Un `403` conferma che la risorsa esiste; un `401` differenziato è un
oracolo per enumerare gli account. Nel caso del login si spende anche il tempo di
una verifica Argon2 su un hash di riferimento, altrimenti la latenza distinguerebbe
gli account esistenti a parità di risposta.

**Conseguenze.** Diagnosticare un problema di permessi è meno immediato: la
distinzione vive nei log lato server, correlata dal `request_id`.

---

<a id="adr-018"></a>

## ADR-018 — Verifica dell'architettura in CI

**Decisione.** I vincoli architetturali sono controlli eseguibili, non voci di una
checklist di review: albero delle dipendenze del dominio, assenza di SQL negli
handler, dieci invarianti dello schema, idempotenza del migratore, assenza di test
saltati.

**Motivo.** Un vincolo controllato solo in code review sopravvive qualche mese.

**Conseguenze.** La pipeline è più lunga e richiede un PostgreSQL reale. In cambio
i controlli 4, 9 e 10 hanno già trovato difetti reali.

---

<a id="adr-019"></a>

## ADR-019 — Stati temporali derivati dalle date, non memorizzati

**Difetto osservato.** `v_property_occupancy` e `v_owner_dashboard` contavano i
contratti con `status = 'active'`. Un contratto a termine resta `active` anche dopo
la scadenza, perché nessuno lo aggiorna: l'occupazione e il canone atteso
includevano contratti finiti da mesi, e la cifra cresceva nel tempo senza che
nessuno avesse sbagliato nulla.

**Decisione.** Lo stato memorizzato cambia solo su **azioni esplicite**
(`activate`, `terminate`, `cancel`). "In essere", "in scadenza" e "scaduto" si
**calcolano** da `start_date` e `end_date`, in due punti che devono rispondere la
stessa cosa:

- `app.lease_in_force(status, start, end)` nel database, usata dalle viste e dal
  filtro `in_force` dell'elenco;
- `Lease::is_in_force`, `expires_within`, `effective_status` nel dominio.

`expiring` ed `expired` restano fra i valori dell'enum PostgreSQL — una riga
scritta a mano potrebbe contenerli, e un valore sconosciuto in lettura deve dare
errore — ma **l'applicazione non li scrive mai**. L'elenco rifiuta con `422` un
filtro `status=expired`, indicando `in_force`: restituire zero righe sarebbe il
modo peggiore di dire che quello stato non esiste in database.

**Motivo.** L'alternativa è uno scheduler che ogni notte porta ad `expired` i
contratti scaduti. Aggiunge un componente da far girare, monitorare e recuperare
dopo un fermo, e apre comunque una finestra in cui il database dice una cosa e il
calendario un'altra. La scadenza non è una decisione: è una conseguenza, e le
conseguenze si derivano.

**Conseguenza: `end_date` è esclusiva.** Il contratto copre
`[start_date, end_date)`. Non è una scelta nuova: è quella già presa dai vincoli
`leases_no_overlap_property` e `leases_no_overlap_room`, che usano
`daterange(start_date, COALESCE(end_date, 'infinity'), '[)')`. Con un confine
inclusivo, il giorno di `end_date` sarebbe insieme "ancora in essere" per il
predicato e "slot libero" per il vincolo di esclusione: due contratti sullo stesso
immobile risulterebbero in essere per un giorno e la dashboard sommerebbe due
canoni — cioè lo stesso errore di conteggio che questa decisione corregge. Il
prezzo è che "contratto fino al 31/12" si registra con `end_date = 2027-01-01`.

**Altre conseguenze.** Le due viste sono state riscritte in
`migrations/0017_lease_in_force_views.sql`; `leases_expiring_90d` non guarda più lo
stato `expiring` ma i contratti in essere che finiscono entro 90 giorni. La finestra
di preavviso vive in `Lease::EXPIRING_WINDOW_DAYS` e coincide con quella della
dashboard: due valori diversi farebbero dire "in scadenza" a una schermata e non
all'altra. Il rinnovo automatico (`auto_renew`) resta non implementato: quello *è*
una decisione, e richiede lo scheduler.

---

<a id="adr-020"></a>

## ADR-020 — Accettazione di un invito via RLS, non con il pool privilegiato

**Contesto.** Accettare un invito richiede tre scritture in un'organizzazione di cui
non si è ancora membri: creare la propria membership, collegare la parte
contrattuale, marcare l'invito. Le policy esistenti vietavano tutte e tre.

**Decisione.** Si estende la Row Level Security
(`migrations/0018_invitation_acceptance_rls.sql`), non si usa `begin_privileged()`.

**Motivo.** La regola 20 di `CLAUDE.md` limita il pool privilegiato a registrazione,
login e rotazione del refresh token, ed è ciò che ne tiene piccola la superficie.
Quei tre flussi esistono perché un contesto di sicurezza non **può** esistere:
avvengono prima dell'autenticazione, e la policy su `users` mostra una riga solo a
chi è già identificato. Qui il contesto esiste — chi accetta è autenticato — e manca
soltanto la membership. Aggiungere un quarto caso avrebbe reso la regola una lista di
eccezioni invece di un criterio.

**Come.** Due funzioni `SECURITY DEFINER`, `app.has_pending_invitation(org_id)` e
`app.current_user_email()`, e cinque policy: `org_members` INSERT di sé stessi con
ruolo `tenant`, `org_invitations` SELECT e UPDATE del proprio invito,
`lease_parties` SELECT e UPDATE della riga con la propria email e `user_id IS NULL`.

**Due controlli indipendenti.** Il token resta verificato dall'applicazione, per
confronto sullo hash: la policy non lo vede. L'applicazione prova che chi chiama
**possiede** il token; la RLS prova che esiste davvero un invito pendente per
quell'email. Nessuno dei due da solo basterebbe, e sono difendibili separatamente:
un difetto nel confronto dello hash non concede l'accesso a un'organizzazione che non
ha invitato nessuno.

**Conseguenza: la lettura non può dipendere dallo stato "pendente".** Marcare
l'invito accettato produrrebbe una riga che chi la scrive non potrebbe più rileggere
— lo stesso difetto di [ADR-008](#adr-008). Le policy di SELECT dell'invitato si
basano quindi sull'email, che l'operazione non cambia; il vincolo "pendente" vive
nelle `USING` degli UPDATE, dove si applica alla riga esistente.

**Conseguenza: l'ordine delle scritture è parte del progetto.**
`has_pending_invitation` diventa falso appena l'invito è marcato accettato, quindi
marcarlo per primo si toglierebbe il permesso di completare il resto.
`AcceptInvitation` scrive nell'ordine membership → parte contrattuale → invito, e il
commento nel codice lo dice.

**Difetto osservato durante l'implementazione.** Le tre scritture riuscivano e
l'accettazione falliva sull'**ultimo** statement: l'evento di dominio. Chi accetta
arriva con il contesto della propria organizzazione personale — di quella
dell'invito non è ancora membro, e il middleware rifiuterebbe di selezionarla — ma
`domain_events_insert` ammetteva solo `org_id = app.current_org_id()`. Poiché
l'evento entra nella stessa transazione del comando, il rifiuto annullava tutto. La
policy ora ammette anche `app.is_org_member(org_id)`, che a quel punto è vero:
"posso annunciare un fatto di un'organizzazione di cui sono membro" è comunque
isolamento, e non lega la scrittura dell'evento all'ordine degli statement.

**Verifica.** Un test di integrazione esegue l'accettazione con una fabbrica di unità
di lavoro il cui `begin_privileged()` **solleva un panic**. Non è un commento che
dice "non si usa il pool privilegiato": è un test che fallisce se qualcuno lo usa.

**Alternativa scartata.** `begin_privileged()` per l'accettazione. Più rapido da
scrivere, e sarebbe stato il quarto uso di un percorso che ne ammette tre — dopo il
quarto il criterio non esiste più, e la superficie di codice che gira con
`BYPASSRLS` cresce senza che nessuno debba giustificarlo.

---

<a id="adr-021"></a>

## ADR-021 — Il confine di isolamento si difende con i privilegi di colonna, non con il `WITH CHECK`

**Difetto osservato.** Le policy introdotte da [ADR-020](#adr-020) permettevano a un
utente autenticato con un invito pendente di iscriversi a **qualunque**
organizzazione di cui conoscesse l'identificatore, e di leggere nome ed email di
tutti i suoi membri. Riprodotto su database reale prima della correzione.

**Causa.** Il `WITH CHECK` di una policy di UPDATE vincola **solo le colonne che
nomina**; tutte le altre restano libere. Su `org_invitations_invitee_accept` il
`WITH CHECK` era `email = app.current_user_email()`, quindi `org_id` non era
vincolato:

```sql
UPDATE app.org_invitations SET org_id = '<altra org>' WHERE email = '<la mia>';
```

passava la `USING` — la riga *esistente* era un invito pendente e valido — e passava
il `WITH CHECK`, perché l'email non cambiava. Da lì
`has_pending_invitation(<altra org>)` diventava vero e
`org_members_accept_invitation` ammetteva l'iscrizione come `tenant`.

La stessa forma su `lease_parties_invitee_link`, dove il `WITH CHECK` fissava `email`
e `user_id` ma non `lease_id`: l'invitato poteva agganciarsi a un contratto diverso
da quello per cui era stato invitato.

**Causa radice.** Queste due sono le uniche policy che concedono scrittura a un **non
membro**, attraversando il confine dell'organizzazione. Altrove una colonna non
vincolata resta dentro il perimetro di chi ha già accesso, e il danno massimo è
spostare un proprio dato fra proprie organizzazioni. Qui la colonna non vincolata
*è* il confine.

**Decisione.** Non si irrigidisce il `WITH CHECK`. Una policy non può confrontare la
riga nuova con quella vecchia — non esiste `OLD` — quindi ogni tentativo in quella
direzione finirebbe per dipendere da sottigliezze di snapshot, che è esattamente il
genere di ragionamento che ha prodotto il difetto. Si toglie invece al ruolo
applicativo il privilegio di aggiornare le colonne di confine
(`migrations/0019_invitation_column_privileges.sql`). Il permesso è verificato prima
delle policy, quindi la manovra fallisce con `permission denied` qualunque cosa la
policy ammetta.

**Conseguenze.** L'elenco delle colonne concesse deve restare allineato a ciò che i
repository aggiornano: aggiungerne una a una `SET` richiede di aggiungerla anche al
`GRANT`. È voluto — è il punto in cui ci si ferma a chiedersi se quella colonna debba
davvero essere scrivibile dall'applicazione. Il controllo 11 delle invarianti
verifica che le colonne di confine restino non aggiornabili, e due test di
integrazione riproducono i tentativi e li attendono respinti.

**Regola generale che ne segue.** Quando una policy concede scrittura oltre il
confine che il resto dello schema difende, il `WITH CHECK` non basta: va accompagnato
da un privilegio di colonna che fissi le colonne del confine. Vale per ogni policy
futura che ammetta un non membro.
