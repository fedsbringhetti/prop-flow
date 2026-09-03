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

---

<a id="adr-022"></a>

## ADR-022 — Un codice stabile per ogni errore, e i messaggi passano in inglese

**Problema.** Ogni risposta d'errore portava `type`, `title` e `detail`, tutti e tre
in italiano e nessuno dei tre stabile. `type` dice la *classe* del problema
(`conflict`, `validation`), che è troppo grossolana: sotto `conflict` stavano email
già registrata, nome di stanza duplicato, invito già pendente e sovrapposizione di
locazione, che per un client sono quattro cose diverse. Restava `detail`, cioè una
frase in prosa. Un client che volesse mostrare un messaggio proprio — o
semplicemente decidere se riprovare — non aveva altro appiglio che confrontare
stringhe italiane destinate a cambiare alla prima riscrittura.

**Decisione.** Ogni risposta acquista un campo `code` stabile, nella forma
`<dominio>.<condizione>`, e i messaggi rivolti al client passano in inglese.

```json
{
  "type": "https://propflow.dev/problems/conflict",
  "title": "Period not available",
  "status": 409,
  "code": "lease.period_overlap",
  "detail": "an active lease already covers this property for the given period",
  "request_id": "…"
}
```

La divisione dei ruoli è netta: `title` e `detail` sono per un umano che legge i
log e possono essere riscritti quando si trova una formulazione migliore; `code` è
il contratto e non cambia. `type` resta la classe e serve a chi vuole gestire
"tutti i conflitti" senza enumerarli.

**Perché un codice e non il testo.** Perché il testo è l'unica parte che vogliamo
poter migliorare. Legare un client alla prosa significa scegliere fra un messaggio
scritto male per sempre e un client che si rompe: il codice toglie il dilemma.

**Perché la localizzazione è del client.** Il server non sa in quale lingua va
mostrato l'errore — dipende dall'utente finale, non dalla richiesta — e non sa in
quale registro: la stessa condizione si dice diversamente in un'app per inquilini e
in un cruscotto per gestori. Un `Accept-Language` sul server sposterebbe le
traduzioni qui dentro senza risolvere il secondo problema.

**Perché adesso.** I punti d'errore sono 128. Al primo client esterno sarebbero
qualche centinaio, e sarebbero da cambiare con qualcuno che già dipende dalle
stringhe.

### Il codice nasce dal tipo, non dal bordo

È la parte che decide se la tassonomia serve o è decorazione. Quattro payload erano
stringhe libere — `Conflict { resource: "utente con questa email" }`,
`Invariant(String)`, `Forbidden(&'static str)`, `NotFound { entity: "immobile" }` —
e da una stringa libera un codice si può solo indovinare confrontando testo, con un
refuso in un letterale che cambia il contratto pubblico senza che nulla se ne
accorga. Sono diventati quattro enum:

| Prima | Ora | Dove |
|---|---|---|
| `Invariant(String)` | `Invariant(Invariant)` | `domain::error` |
| `Forbidden(&'static str)` | `Forbidden(Denial)` | `domain::error` |
| `NotFound { entity: &'static str }` | `NotFound(Entity)` | `domain::error`, `application::error` |
| `Conflict { resource: &'static str }` | `Conflict(Conflict)` | `application::error` |

`DomainError::code()` e `AppError::code()` delegano al tipo che porta la
condizione. Il bordo HTTP legge `error.code()` **una volta sola**, prima del match,
e lo trascrive in ogni ramo: nessun ramo può inventarne uno o dimenticarsene.
`AppError::Integrity { details }` resta una stringa perché è per definizione il
vincolo che non sappiamo tradurre, e il suo codice —
`conflict.constraint_violated` — è deliberatamente muto. Vederlo ripetutamente nei
log sullo stesso vincolo è il segnale che serve una variante di `Conflict`.

### Quattro scelte non ovvie

**Le varianti possono condividere un codice, e alcune lo fanno.** Le tre transizioni
di contratto rifiutate danno tutte `lease.invalid_transition`, e le due operazioni
su un invito già accettato danno `invitation.already_accepted`. Il criterio è il
rimedio del client: se è lo stesso, il codice è lo stesso, e il `detail` distingue
il caso per chi legge. Un test lo fissa, così separarle resterebbe una scelta e non
una svista.

**`auth.token_expired` è distinto da `auth.token_invalid`.** Al primo il client
risponde rinnovando, al secondo rimandando al login, e senza la distinzione
riautenticherebbe l'utente a ogni scadenza. Non è un oracolo: per ottenere
`token_expired` bisogna presentare un token che abbiamo firmato noi. Il **refresh**
invece resta indistinto su `auth.invalid_credentials` — lì separare "scaduto" da
"riusato" direbbe a chi ha rubato un token se è ancora vivo. Per lo stesso motivo
login con password errata e login su account inesistente condividono il codice: la
scelta di ADR-017 sul `404` vale identica qui.

**Il filtro su uno stato derivato ha un codice proprio.** `?status=expired` era una
`Validation` sul campo `status`, quindi avrebbe dato `validation.failed` —
indistinguibile da "`status` non è un valore ammesso", che è un errore diverso sullo
stesso campo. È diventato `AppError::UnsupportedQuery`, `422` come prima e con lo
stesso `errors[]`, ma con codice `lease.derived_status_filter`. Il valore non è
malformato: è derivato (ADR-019), ed è una condizione a sé.

**Undici motivi di rifiuto, quattro pubblicati.** `Denial` contiene anche
`account.inactive`, `account.locked`, `session.expired`, `invitation.revoked` e
simili, che login, refresh e accettazione ripiegano su `auth.invalid_credentials`
prima del bordo. I codici esistono perché il tipo li richiede e perché nei log la
distinzione serve, ma **non compaiono in `docs/05-api.md`**: un codice che l'API non
può produrre non è parte del contratto, e pubblicarlo prometterebbe una distinzione
che rifiutiamo di fare.

### Cosa resta fuori

`auth.denied_by_policy` — la RLS che rifiuta una scrittura già autorizzata dal
service layer — mantiene lo stato `403`. Se scatta è un difetto nostro e non un
errore del chiamante, quindi `500` sarebbe più onesto; portarcelo è una decisione
sul comportamento e non sulla tassonomia, e non entra in un cambiamento che
introduce i codici. Il codice serve intanto a renderlo cercabile nei log.

**Conseguenze sul contratto.** Aggiungere `code` è additivo e non rompe nessuno. Il
passaggio dei `detail` all'inglese romperebbe un client che confrontasse le
stringhe, che è esattamente il motivo per cui `code` esiste: sulla linea 0.x è
ammesso, e va nel changelog come cambiamento visibile. La tabella dei codici in
[05 — API](05-api.md) è da qui in poi parte del contratto pubblico: aggiungere un
codice è additivo, rinominarne uno è una rottura.

---

<a id="adr-023"></a>

## ADR-023 — La configurazione locale la carica la shell, non `.env`

**Difetto osservato.** `README.md`, `CLAUDE.md` e [06 — Sviluppo](06-sviluppo.md)
indicavano `cp .env.example .env` come passo del primo avvio, e il passo successivo
falliva:

```
$ cp .env.example .env && cargo run -p propflow-api
Error: configurazione non valida
Caused by: missing field `private_key_path` for key "default.auth"
```

Nessun crate dipende da `dotenvy` — compare in `Cargo.lock` solo come dipendenza
transitiva di sqlx — e `docker-compose.yml` dichiara le variabili inline sotto
`environment:`, senza `env_file:`. Il file `.env` non lo leggeva nessuno. Era il
primo comando che un contributore nuovo esegue, ed era sbagliato in tre punti
contemporaneamente.

**Decisione.** `.env` resta un file per la **shell**, non una sorgente di
configurazione, e la documentazione riporta il passo che lo carica:

```bash
set -a; source .env; set +a
```

Le sorgenti restano due, quelle che `AppConfig::load` dichiara: `config/default.toml`
e l'ambiente, con l'ambiente che vince.

**Alternativa scartata: aggiungere `dotenvy` ai binari.** Sarebbe una terza sorgente
di configurazione, con una precedenza in più da definire e da documentare, presente
in produzione per servire solo lo sviluppo: un `.env` finito per sbaglio nella
directory di lavoro di un container cambierebbe il comportamento del servizio senza
comparire da nessuna parte. `source` è esplicito, non entra nei binari, e ciò che
produce sono variabili d'ambiente come tutte le altre.

**Conseguenza accettata: `config/default.toml` si risolve rispetto alla directory di
lavoro.** L'API va avviata dalla radice del repository o dall'immagine, dove il file
sta in `/app/config`. Altrove il file non viene trovato e figment lo salta senza
protestare, perché è dichiarato opzionale: le quattro impostazioni obbligatorie
falliscono rumorosamente, le altre ripiegano sui default del codice.

Non c'è un avviso perché non c'è ancora dove scriverlo: la configurazione si carica
**prima** della telemetria — è la configurazione a dire come inizializzarla — quindi
un `warn!` non avrebbe alcun subscriber, e scrivere su stderr da una funzione di
libreria significa stampare anche quando chi chiama ha scelto i log JSON. Un percorso
assoluto risolto da `CARGO_MANIFEST_DIR` sistemerebbe lo sviluppo e sarebbe falso
nell'immagine, dove non esiste alcun manifest.

**Conseguenze.** Il difetto vero non era il passo mancante: era che la documentazione
poteva sbagliare senza che nulla se ne accorgesse. Le impostazioni sono ora verificate
da `crates/api/tests/docs_config_reference.rs`, che deriva l'elenco dal sorgente di
`AppConfig` e del migratore e lo confronta con le tabelle di
[07 — Operazioni](07-operazioni.md#configurazione), compresi i default dichiarati.
Gira senza database, quindi in ogni `cargo test`.

---

<a id="adr-024"></a>

## ADR-024 — Lo storage oggetti: firma nostra, trasporto altrui, cifratura letta e non dichiarata

**Contesto.** La porta `ObjectStorage` era dichiarata dal primo giorno e senza
adattatore. Il piano della prima fetta dei documenti
([09 — Stato e roadmap](09-stato-e-roadmap.md#piano-documenti-fetta-1)) aveva già
deciso `rusty-s3` sopra `reqwest`, l'upload multipart attraverso l'API con download
pre-firmato, e la cifratura a riposo lato bucket. Tre punti si sono decisi
implementando, e stanno qui perché nessuno dei tre è deducibile dal codice.

**Decisione 1: `reqwest` con `rustls-no-provider` e il provider `ring` installato
all'avvio.** La feature `rustls` di reqwest 0.13 implica `__rustls-aws-lc-rs`, cioè
`aws-lc-rs`, cioè il toolchain C e cmake nell'immagine di build — esattamente ciò che
[ADR-012](#adr-012) evita scegliendo `rust_crypto` per `jsonwebtoken`. Con
`rustls-no-provider` il provider non c'è, e reqwest **va in panico** costruendo il
client se nessuno ha installato un default. Lo installa `S3ObjectStorage::new`:

```rust
let _ = rustls::crypto::ring::default_provider().install_default();
```

`ring` è già nell'albero, portato da sqlx con `tls-rustls-ring`: nessun crate in più,
nessuna versione duplicata. Il risultato dell'installazione si ignora di proposito —
fallisce solo se un provider c'è già, che è il caso in cui non c'è nulla da fare.

**Decisione 2: la cifratura si legge dalla risposta, non si dichiara.** Il piano
diceva «`is_encrypted` nasce `true`». Nasce invece dall'header
`x-amz-server-side-encryption` che il fornitore restituisce sulla `PUT`, e
`encryption_key_ref` registra quel valore. Su un bucket senza cifratura predefinita la
colonna dice `false`.

Il motivo è che l'alternativa è una colonna che mente. `is_encrypted = true` scritto a
priori è vero solo se qualcuno ha configurato il bucket, e la configurazione del bucket
non sta in questo repository: è un passo di provisioning che si può dimenticare, e
l'unico posto dove ce ne accorgeremmo è proprio quella colonna. Leggerlo dalla risposta
costa zero — l'header è già lì — e trasforma una speranza in un fatto verificabile.

Resta il limite già dichiarato nel piano, che nessuna di queste righe risolve: con SSE
la chiave è del fornitore, quindi non protegge da un accesso amministrativo al bucket.

**Decisione 3: il bucket lo crea il provisioning, non l'applicazione.** La porta non
ha un `create_bucket` e non deve averlo: darglielo significherebbe darlo all'API anche
in produzione, dove creare bucket non è un'operazione che un servizio HTTP debba poter
fare. Lo creano `docker-compose.yml` (servizio `minio-setup`) in sviluppo e
`ops/ci/prepare-object-storage.sh` nei test.

**Conseguenza operativa.** La cifratura a riposo è un passo di configurazione del
bucket, documentato in [07 — Operazioni](07-operazioni.md#configurazione). Se manca,
non fallisce nulla: i documenti si caricano e si scaricano, e `documents.is_encrypted`
resta `false`. È il modo di accorgersene interrogando il database invece di credere a
una riga di configurazione.

---

<a id="adr-025"></a>

## ADR-025 — Il limite del corpo è doppio, e le rotte di caricamento stanno in un gruppo a parte

**Difetto anticipato dal piano, e più grande del previsto.** Il piano avvertiva che
`RequestBodyLimitLayer` è montato sull'intero router in `crates/api/src/lib.rs` e non
ammette deroghe per rotta: un limite più largo dichiarato sopra la rotta di upload non
viene mai raggiunto, perché il layer esterno rifiuta per primo. Alzare quello generale
funzionerebbe, ed è la trappola vera — indebolisce ogni endpoint per servirne uno.

Implementando è emerso che i limiti sono **due**, e il secondo non è un layer.
`Multipart` di axum applica anche `DefaultBodyLimit`, che vale **2 MiB** e non
compare fra i layer del router: alzando solo `RequestBodyLimitLayer` un file da 3 MiB
fallisce lo stesso, e l'errore arriva dal parser multipart mentre si legge il corpo.
Cercarlo fra i layer non lo trova, ed è il modo di perderci una giornata.

**Decisione.** Il router si compone in due gruppi:

- il gruppo generale — tutto tranne il caricamento — porta
  `RequestBodyLimitLayer::new(body_limit_bytes)`, con il default di 1 MiB invariato;
- `routes::documents::upload_router` porta **entrambi** i limiti a
  `upload_body_limit_bytes` (25 MiB di default): `RequestBodyLimitLayer` per i byte in
  transito e `DefaultBodyLimit::max` per il parser multipart.

I due limiti sono due impostazioni distinte, `PROPFLOW__SERVER__BODY_LIMIT_BYTES` e
`PROPFLOW__SERVER__UPLOAD_BODY_LIMIT_BYTES`, entrambe in
[07 — Operazioni](07-operazioni.md#configurazione).

**Conseguenza sul modello di errore.** Il superamento del limite può manifestarsi in
due punti — il layer, oppure il parser multipart mentre legge — e il chiamante non
deve vedere due codici diversi a seconda di quale ha incontrato per primo: entrambi
diventano `413` con `request.payload_too_large`.

**Verificato da due test che si tengono a vicenda**
(`crates/api/tests/http_contract.rs`): un caricamento da 3 MiB deve **passare** —
sopra entrambi i limiti, quindi fallirebbe se se ne fosse alzato uno solo — e un
corpo JSON oltre 1 MiB su `POST /api/v1/properties` deve **fallire** con `413`, che è
ciò che si romperebbe alzando il limite generale invece di separare i gruppi.

---

<a id="adr-026"></a>

## ADR-026 — I privilegi di colonna anche dove non c'è un confine da difendere

**Contesto.** [ADR-021](#adr-021) ha introdotto i privilegi di colonna per un caso
preciso: due policy che concedevano scrittura a un **non membro**, attraversando il
confine dell'organizzazione. La regola generale che ne è seguita — «quando una policy
concede scrittura oltre il confine, il `WITH CHECK` non basta» — descrive perché
*quelle due* ne avevano bisogno. Le segnalazioni pongono un caso che quella regola non
copre, e in cui lo strumento conviene lo stesso.

**Difetto osservato.** La `WITH CHECK` di `tickets_update` vincola **solo** `org_id`.
La `USING` lascia passare lo staff, chi ha segnalato e chi è assegnato; nessun'altra
colonna è nominata, quindi tutte restano libere per chiunque la `USING` ammetta — e
ammette l'inquilino che ha aperto la segnalazione.

Riprodotto su database reale, con `UPDATE` scritte a mano come inquilino sotto RLS,
sulla segnalazione aperta da lui:

| Tentativo | Esito |
|---|---|
| `actual_cost = 9999.00` | riuscito |
| `assigned_to` = un altro utente | riuscito |
| `status = 'closed'`, saltando il flusso | riuscito |
| `priority = 'urgent'` | riuscito |
| spostarlo su un immobile in cui non abita | riuscito |
| toccare la segnalazione di un altro | rifiutato dalla `USING` |
| spostarlo in un'altra organizzazione | rifiutato dal trigger `assert_org_matches_property` |

**Perché non è ADR-021.** Due cose che l'esperimento ha stabilito e che il
ragionamento da solo non dava. Il **confine di isolamento tiene**: dopo tutte le
scritture l'inquilino vede ancora una sola segnalazione, non ha guadagnato visibilità
su niente. E il **danno è registrato**: lo storico annota `open → closed` con lui come
autore, e `tickets` è fra le tabelle con il trigger di audit, quindi ogni `UPDATE`
finisce in `audit_log`. Non è una violazione dell'isolamento: è **corruzione di dati
dentro la propria organizzazione**, tracciata.

**Causa radice, e criterio nuovo.** Lo schema non ha **alcun vincolo** fra `room_id`,
`lease_id` e `property_id` di una segnalazione. Il trigger
`assert_org_matches_property` verifica soltanto che l'`org_id` del ticket coincida con
quello dell'immobile: una stanza di un altro immobile, o il contratto di un altro
immobile, sono accettati senza protestare. Riproducendo la prova qui sopra si è
ottenuta esattamente questa riga — una segnalazione spostata su un immobile, con il
`lease_id` che continuava a puntare al contratto del primo.

Nessuna policy può impedirlo, e non per come è scritta: **una policy non può
confrontare la riga nuova con quella vecchia**. È la stessa constatazione di ADR-021,
e vale qui identica pur con una posta in gioco diversa.

**Decisione.** `migrations/0020_ticket_column_privileges.sql` toglie a `propflow_app`
l'`UPDATE` sull'intera tabella `app.tickets` e gliela riconcede sulle sole nove
colonne che i repository scrivono: `status`, `category`, `priority`, `assigned_to`,
`assigned_at`, `estimated_cost`, `actual_cost`, `resolved_at`, `closed_at`.

Restano fuori `property_id`, `room_id`, `lease_id`, `reported_by`,
`reported_by_kind`, `title`, `description`, `reference`, `currency` e `deleted_at`.
`updated_at` non compare perché nessuna query la nomina: la scrive il trigger
`set_updated_at`, che è `BEFORE UPDATE` e non è soggetto ai privilegi di colonna del
chiamante.

**Il criterio, dichiarato.** Lo strumento serve in due casi, non in uno:

1. quando una policy concede scrittura **oltre il confine** di isolamento (ADR-021);
2. quando una colonna **non ha un motivo legittimo di cambiare** e lo schema non ha
   un vincolo che ne difenda la coerenza. Allora il privilegio di colonna è la sola
   difesa disponibile, e costa una riga.

Il secondo caso è quello delle segnalazioni, ed è quello che rende la scelta quasi
gratuita: una segnalazione aperta sull'immobile sbagliato **si annulla e si riapre** —
`cancelled` esiste per questo — e non si corregge spostandola.

**Conseguenza sul dove va fatto il controllo.** L'incoerenza strutturale può ora
nascere **solo all'apertura**. Il controllo applicativo sta lì, una volta sola
(`OpenTicket` legge stanza e contratto dai rispettivi repository e confronta
`property_id`), e non su ogni transizione. Senza la migration andrebbe ripetuto
ovunque, o dimenticato una volta.

**Cosa i privilegi non fanno.** Non dicono nulla su chi possa chiudere una
segnalazione o valorizzarne il costo: `status`, `assigned_to` e i due costi restano
concesse — devono esserlo, è il lavoro dell'applicazione. Quella parte resta
autorizzazione applicativa, transizione per transizione.

**Conseguenze e manutenzione.** L'elenco concesso e le colonne che compaiono nella
`UPDATE` del repository sono **la stessa lista**. Aggiungerne una senza aggiungerla
alla `GRANT` fa fallire la scrittura con `permission denied`, e fallisce nei test di
integrazione, che girano come `propflow_app`. Il controllo 12 delle invarianti dello
schema (`ops/sql/checks/schema_invariants.sql`) verifica che le colonne strutturali
restino non aggiornabili, con un nome proprio — `colonna-strutturale-scrivibile` —
distinto da quello del controllo 11, perché il criterio è diverso e confonderli
renderebbe illeggibile la ragione di entrambi.

**Alternativa scartata: irrigidire il `WITH CHECK`.** Stessa ragione di ADR-021, e una
in più: qui non basterebbe nemmeno in linea di principio, perché la condizione da
imporre («`room_id` appartiene a `property_id`») richiederebbe alla policy di leggere
`app.rooms`, e la coerenza da difendere riguarda comunque il rapporto fra la riga
vecchia e quella nuova.

**Alternativa scartata: un trigger di coerenza.** Un `BEFORE UPDATE` che confronti
`OLD` e `NEW` potrebbe farlo, e sarebbe la via naturale se le colonne dovessero
restare scrivibili in qualche caso. Non è così: non devono cambiare mai, e togliere il
privilegio esprime *questo*, mentre un trigger esprimerebbe «può cambiare, ma solo
così». Un trigger in più è anche un trigger in più su ogni `UPDATE` di ogni
segnalazione, mentre il controllo dei privilegi è già nel percorso.

---

<a id="adr-027"></a>

## ADR-027 — Il riferimento di una segnalazione si numera per organizzazione

**Difetto osservato.** `tickets.reference` nasceva da `app.ticket_reference_seq`, una
sola sequenza per il cluster. La prima segnalazione di qualunque organizzazione era
`TK-000001`; la millesima della piattaforma era `TK-001000` anche se per il suo
proprietario era la seconda. Il numero rivelava quante segnalazioni esistono
sull'intera piattaforma.

Non è un dato sensibile e la fuga è modesta. È però un dato che non si intendeva
dare, e la ragione per cui si è corretto subito non è la gravità ma **la forma del
costo**: finché nessuno ha dati reali è una migration su tabelle vuote, dopo sarebbe
rinumerare o convivere con due formati per sempre. È una delle poche decisioni la cui
finestra si chiude da sola.

**Decisione.** Un contatore per organizzazione — `app.ticket_reference_counters`,
una riga per `org_id` con l'ultimo numero usato — riempito da un trigger
`BEFORE INSERT` su `app.tickets` (`migrations/0021_ticket_reference_per_org.sql`).

L'unicità cambia forma insieme alla numerazione: `tickets_reference_uk` era su
`(reference)` e diventa su `(org_id, reference)`, perché due `TK-000001` in due
organizzazioni diverse sono ora legittimi.

**Perché un contatore e non una sequenza per organizzazione.** Una sequenza per org
richiederebbe DDL dinamico a ogni registrazione — `CREATE SEQUENCE` dentro un
trigger — e lascerebbe dietro di sé oggetti di schema che nessuna migration ha creato
e che nessuno cancella. Una riga in una tabella si crea, si legge, si ispeziona e se
ne va con l'organizzazione a cui appartiene.

**Perché un trigger e non l'applicazione.** Così `reference` resta fuori dalla
`GRANT UPDATE` della `0020` e fuori dalla portata del ruolo applicativo, che è ciò
che il controllo 12 delle invarianti protegge. Il repository non nomina la colonna e
non deve saperne nulla; se un giorno la nominasse, il trigger la sovrascriverebbe
comunque. Calcolare il numero nell'applicazione avrebbe richiesto di concedere la
colonna in scrittura, cioè di disfare la 0020 per un contatore.

**Conseguenza sulla concorrenza, da dire ad alta voce.** `ON CONFLICT DO UPDATE`
prende un lock di riga: due aperture concorrenti **nella stessa organizzazione** si
serializzano fino al commit. Per un'azione umana su una casa è irrilevante, e in
cambio la numerazione non ha buchi — una transazione annullata non consuma un numero,
cosa che una sequenza invece fa. Se un giorno esistesse un'organizzazione che apre
segnalazioni a ritmo di macchina, questa è la riga da guardare per prima.

**Conseguenza sul contratto pubblico.** Il riferimento **non è più un identificatore
globale**. Per raggiungere una segnalazione serve il suo `id`, e un client che
indicizzasse per `reference` senza l'organizzazione fonderebbe righe diverse. È
documentato in [05 — API](05-api.md#reference).

**Il contatore non è manipolabile dall'applicazione.** La tabella ha la RLS attiva e
una sola policy, di `SELECT`: al ruolo applicativo i privilegi predefiniti lasciano
`INSERT` e `UPDATE` a livello di tabella, ma nessuna policy li ammette, quindi
l'inserimento è rifiutato e l'aggiornamento non trova righe. Senza questa asimmetria
un membro qualsiasi potrebbe riportare il contatore indietro e far collidere il
prossimo riferimento con uno già emesso. Due test di persistenza lo fissano.

**Le righe già presenti non si rinumerano.** Il contatore parte dal numero più alto
già usato in quell'organizzazione — non dal conteggio delle righe, che con un'org
avente `TK-000001` e `TK-000005` ripartirebbe da 3 e si scontrerebbe con `TK-000005`
due aperture dopo.

**Alternativa scartata: non esporre affatto il riferimento.** Avrebbe rimandato la
decisione senza pagarla, ma toglie al proprietario un numero corto da citare al
telefono, che è l'unica ragione per cui una colonna del genere esiste accanto a un
UUID.

---

<a id="adr-028"></a>

## ADR-028 — Lo stato «scaduto» di un addebito si deriva, come quello di un contratto

**Contesto.** [ADR-019](#adr-019) ha stabilito per i contratti che gli stati
conseguenza del **calendario** non si memorizzano: `expiring` ed `expired` esistono
nell'enum e nessuno li scrive, perché uno stato memorizzato invecchia da solo — nessuno
lo aggiorna quando la data passa — e la dashboard finisce per contare cose finite mesi
prima. Gli addebiti pongono lo stesso caso: `app.charge_status` ha `overdue`.

**Decisione.** `overdue` non si scrive mai. Un addebito scaduto è un addebito `issued`
la cui `due_date` è passata e il cui residuo è positivo, e la risposta dell'API porta
**due** stati, come i contratti:

- `status`, quello memorizzato, che cambia solo su azioni esplicite;
- `effective_status`, che vale `overdue` quando la scadenza è passata.

**La definizione vive in un posto solo, e quel posto è la vista.**
`app.v_charge_balances.is_past_due` è
`due_date < CURRENT_DATE AND amount > COALESCE(paid_amount, 0)`. Il dominio la ripete
in `Charge::is_past_due(today, paid_amount)` per poterla verificare senza database —
stessa coppia di `app.lease_in_force` e `Lease::is_in_force`, e stesso obbligo di
rispondere la stessa cosa.

**Il secondo parametro non è una comodità.** Il piano prevedeva `is_past_due(today)`,
con un argomento. Non poteva funzionare: `paid_amount` **non è un dato
dell'aggregato** — vive in `app.payments` e lo calcola la vista — e una funzione che
lo ignorasse non sarebbe *la stessa* formula, sarebbe una che le somiglia finché non
esistono pagamenti. Passarlo esplicitamente è ciò che rende verificabile l'uguaglianza.

Il confronto è fra `Decimal` e non fra `Money`, che non implementa `PartialOrd`
apposta ([ADR-002](#adr-002)). È lecito **solo** perché la valuta è una sola — vedi
[ADR-030](#adr-030), dove quella limitazione ha esattamente questa origine.

**La formula non guardava lo stato, e `effective_status` sì.** Era la sola asimmetria
fra i due: `v_charge_balances.is_past_due` era vero anche per un addebito **annullato**
con una scadenza nel passato, perché la vista non filtrava su `status`, e a escluderlo
restava `Charge::effective_status`, che deriva `overdue` solo dagli stati
dell'insoluto.

**L'asimmetria è finita con la migration `0024`**, e questo paragrafo resta al passato
perché la sua conseguenza — descritta qui sotto — è il difetto che quella migration ha
chiuso. Vedi [ADR-032](#adr-032): la vista azzera residuo e scadenza per `void` e
`written_off`, e `Charge::is_past_due` la segue, altrimenti smetterebbe di essere *la
stessa* formula. `effective_status` resta com'era — il suo `is_outstanding()` esclude
già più di quanto la vista escluda — e la sovrapposizione fra i due controlli è voluta.

**Chi leggeva la vista direttamente ereditava quella definizione più larga**, e la
conseguenza era peggiore di quanto questo ADR dicesse quando è stato scritto.
Riprodotto su database vero dopo l'esecuzione della fetta contabile, con tre addebiti
sullo stesso contratto:

| stato | importo | `outstanding_amount` |
|---|---|---|
| `issued` | 800.00 | 800.00 |
| **`void`** | **120.00** | **120.00** |
| `issued`, scaduto | 40.00 | 40.00 |

`v_owner_dashboard.total_outstanding` ha risposto **960.00**, dove il numero vero è
840.00: l'addebito annullato — quello per cui all'inquilino è stato detto che la
richiesta era caduta — continua a contare come credito. `written_off` fa lo stesso, ed
è peggio, perché è una perdita e non un credito. Non è quindi solo `overdue_charges`:
è anche il totale insoluto, che è il numero di testa della dashboard.

Il difetto **non nasceva con quella fetta** — la vista era così dalla `0015` — ma è
quella fetta ad averlo reso raggiungibile, perché prima non esisteva alcun addebito.
**L'elenco dell'API era invece corretto**: `?outstanding=true` enumera i tre stati
dell'insoluto come letterale nella query del repository, quindi `void` e `written_off`
non vi entravano. Sbagliavano le sole viste.

**Corretto dalla migration `0024`**, primo passo del modulo dei pagamenti: lo stesso
scenario risponde ora `total_outstanding = 840.00`, ed è sotto test. La ragione per cui
la correzione è arrivata *lì* e non prima è che «cosa significa insoluto» è la domanda
di quel modulo, e deciderla altrove avrebbe voluto dire deciderla due volte. Vedi
[ADR-032](#adr-032).

**Conseguenza sul filtro dell'elenco.** `?status=overdue` risponde **422** con il
codice `charge.derived_status_filter`, indicando `?outstanding=true`. È la stessa forma
di `?status=expiring` sui contratti: restituire una pagina vuota sarebbe il modo
peggiore di dire che quello stato in database non c'è.

**Conseguenza sul fuso orario, da dire ad alta voce.** `CURRENT_DATE` dipende dal fuso
della sessione PostgreSQL. Un addebito in scadenza oggi diventa scaduto alla mezzanotte
**del database**, non a quella di chi guarda: con il container in UTC e un utente in
Italia lo scarto è di una o due ore, una volta al giorno, sul solo giorno della
scadenza. Non si corregge qui — correggerlo significa decidere di chi è il fuso orario
di riferimento, che è una domanda di prodotto — ma il sintomo («scaduto troppo
presto») è di quelli che si cercano nel posto sbagliato. Per questo `ChargeBalance`
porta `is_past_due` **dalla vista** invece di ricalcolarlo: in caso di scarto vince la
definizione che ha prodotto il numero mostrato accanto.

---

<a id="adr-029"></a>

## ADR-029 — Un addebito emesso non si riscrive, e sulle bollette lo stesso strumento non si usa

**Contesto.** [ADR-026](#adr-026) ha dichiarato il criterio per cui i privilegi di
colonna convengono anche senza un confine da difendere: «una colonna non ha un motivo
legittimo di cambiare e lo schema non ha un vincolo che ne difenda la coerenza». Il
modulo contabile è il primo che lo applica **due volte con esiti opposti**, ed è la
prova che il criterio serve a decidere e non a estendersi da sé.

**Difetto osservato.** La `WITH CHECK` di `charges_update` è identica alla `USING` —
entrambe `org_id = app.current_org_id() AND app.is_org_staff(org_id)` — e una
`WITH CHECK` che nomina il solo `org_id` lascia libera ogni altra colonna. È la stessa
constatazione di ADR-021 e ADR-026: **una policy non può confrontare la riga nuova con
quella vecchia**.

Concretamente, uno staff può riscrivere l'`amount` di un addebito già emesso. Non è
una violazione dell'isolamento — resta dentro la propria organizzazione, ed è
registrata in `audit_log`, perché `app.charges` è fra le tabelle con il trigger di
audit della `0013` — ma è la corruzione che **conta di più** fra quelle possibili,
perché un addebito non è un dato interno: è una **pretesa verso una persona**.
Riscriverne l'importo dopo averla comunicata non è una correzione, è cambiare le carte
in tavola. La traccia resta nell'audit, ma la riga che l'inquilino rilegge dice
un'altra cifra senza che nulla, **nella riga stessa**, ricordi la prima.

**Decisione, parte prima: sugli addebiti sì.**
`migrations/0023_charge_column_privileges.sql` revoca a `propflow_app` l'`UPDATE`
sull'intera `app.charges` e gliela riconcede su **una colonna sola**:

```sql
REVOKE UPDATE ON app.charges FROM propflow_app;
GRANT  UPDATE (status) ON app.charges TO propflow_app;
```

È la `GRANT` più corta del progetto, e la lunghezza è la misura di quanto il criterio
si applichi qui: tutto il resto — `lease_id`, `charge_type`, `amount`, `currency`,
`due_date`, `period_start`, `period_end`, `description`, `utility_bill_id`,
`generation_key`, `issued_at`, `document_id`, `notes`, `deleted_at` — descrive la
pretesa **nel momento in cui è stata fatta**, e cambiarla significa farne un'altra.

`deleted_at` è fra le negate, e non per completezza: un addebito **non si cancella**.
L'asimmetria con la bolletta — che annullata sparisce, vedi sotto — diventa così
strutturale invece di restare una convenzione.

**Il modulo dei pagamenti non avrà bisogno di altro.** Riconciliare un incasso porta
l'addebito a `partially_paid` o `paid`, cioè scrive `status`, che è concessa. È il
motivo per cui questa migration si scrive **adesso** e non insieme ai pagamenti:
posarla ora rende il modulo successivo additivo — un metodo di dominio nuovo su
`Charge` e uno use case, nessuna migration.

**Decisione, parte seconda: sulle bollette no**, ed è la prima volta che ADR-026 dà
«no». Sarebbe l'automatismo naturale: la tabella accanto, lo stesso difetto nella
`WITH CHECK`. Sarebbe sbagliato. Una bolletta si **digita**, dodici campi alla volta, e
finché non è validata correggerne uno è il lavoro previsto, non un'anomalia. La `GRANT`
dovrebbe quindi nominare quasi ogni colonna, e **un elenco che contiene quasi tutto non
difende niente**: si pagherebbe l'onere di manutenzione — ogni colonna aggiunta a una
`UPDATE` va aggiunta anche là — senza comprare la proprietà per cui l'onere esisteva.

L'immutabilità della bolletta è quindi un'invariante di **dominio**:
`UtilityBill::apply` rifiuta le modifiche da `allocated` in poi, cioè da quando quei
numeri sono la base di una pretesa già fatta a qualcuno.

**Conseguenza da dire ad alta voce: non esiste un `PATCH` sugli addebiti.** Prorogare
una scadenza — «ti do una settimana in più» — oggi si fa annullando e riemettendo. È
scomodo ed è voluto: è il punto in cui ci si ferma a chiedersi se `due_date` debba
davvero essere scrivibile, e il giorno in cui la risposta sarà sì è una migration
nuova, una riga. Un test di contratto fissa che la rotta non esista (`405`), perché
altrimenti la sua assenza si leggerebbe come una dimenticanza.

**Conseguenze e manutenzione.** Il controllo 12 delle invarianti
(`ops/sql/checks/schema_invariants.sql`) estende l'elenco `colonna-strutturale-scrivibile`
con le tredici colonne dell'addebito, **mantenendo il nome**: il criterio è quello di
ADR-026 in entrambi i casi, e inventargli un terzo nome renderebbe illeggibile la
ragione comune. Verificato anche al contrario, concedendo `UPDATE (amount)` a mano e
vedendo il controllo fallire con `colonna-strutturale-scrivibile / charges.amount`.

---

<a id="adr-030"></a>

## ADR-030 — La valuta resta una sola, e la ragione non è l'importo: è la vista dei saldi

**Contesto.** La domanda è stata rinviata due volte. Il modulo delle segnalazioni l'ha
posta sui costi di una riparazione e l'ha spostata qui per nome; qui ha una risposta,
con una motivazione che allora non si poteva ancora vedere.

**Decisione.** `currency` resta al default `EUR` e **non è dichiarabile**: non compare
in nessuna `INSERT` né in nessuna `UPDATE` del modulo contabile, ed è fra le colonne
che il controllo 12 delle invarianti verifica non essere scrivibili.

**La ragione non è la colonna.** La colonna c'è già, su `app.utility_bills` e su
`app.charges`, con il suo default: accettarla come parametro sarebbe un campo in un
DTO. La ragione sono le **due viste**:

- `app.v_charge_balances` calcola
  `amount - SUM(payments.amount - refunded_amount)` **senza confrontare `currency`**,
  e le due colonne esistono su entrambe le tabelle;
- `app.v_owner_dashboard.total_outstanding` somma `outstanding_amount` di tutta
  l'organizzazione allo stesso modo.

Bastano due valute perché quei due numeri diventino **silenziosamente falsi**: non un
errore di arrotondamento, una somma di grandezze diverse.

**Il dominio lo aveva già previsto, e il SQL no.** `Money` non implementa `PartialOrd`
proprio per rendere impossibile quel confronto ([ADR-002](#adr-002)); le viste lo fanno
lo stesso, perché il SQL non ha quel tipo. È il caso in cui una difesa di tipo esiste a
un livello e non all'altro, e la difesa che manca decide.

**Cosa costerebbe cambiare idea.** Una migration sulle due viste e una **scelta di
prodotto** su cosa significhi un totale insoluto multi-valuta — un numero per valuta?
una conversione, e a quale tasso, e del giorno di quale evento? Finché quella domanda
non ha una risposta, la colonna scrivibile sarebbe un modo di renderla urgente per
sbaglio.

**Conseguenza sul dominio.** `Charge::is_past_due` confronta `Decimal` e non `Money`,
e può farlo **solo** grazie a questa decisione: è la riga da rileggere il giorno in cui
qualcuno vorrà dichiarare la valuta.

---

<a id="adr-031"></a>

## ADR-031 — Il documento di una bolletta si vede se si vede la bolletta

**Difetto osservato.** `app.can_access_attachment` risolveva il ramo `utility_bill` su
`app.can_access_property(b.property_id)`, mentre la policy `utility_bills_select`
chiede staff **oppure** essere parte del contratto indicato:

```
utility_bills_select  →  is_org_staff(org_id)
                         OR (lease_id IS NOT NULL AND can_access_lease(lease_id))
can_access_attachment →  can_access_property(b.property_id)
```

`can_access_property` è vera per **ogni** inquilino dell'immobile. In un co-living con
contratti per stanza, il coinquilino A soddisfa quindi la seconda e non la prima: non
vede la riga della bolletta imputata al contratto di B, e ne scaricherebbe la
scansione — la fattura con il nome, l'indirizzo e i consumi di B.

Finché nessuna bolletta aveva allegati la cosa non si vedeva. È la fetta contabile ad
aprire le rotte `/api/v1/utility-bills/{id}/documents`, ed è quindi quella fetta a
doverla correggere: **la migration viene prima del bordo HTTP**, non dopo.

**Decisione.** `migrations/0022_bill_attachment_access.sql` riallinea il ramo alla
policy con un `CREATE OR REPLACE FUNCTION`. La regola diventa quella che si legge in
una riga: *il documento di una bolletta si vede se si vede la bolletta.*

**Il ramo `charge` non si tocca, ed è corretto così.** Risolve su
`can_access_lease(c.lease_id)`, che è **esattamente** la condizione di
`charges_select`. Le due parti già coincidono, e allinearle di nuovo per simmetria
significherebbe riscrivere una riga giusta.

**Come si è saputo che la migration serviva.** Non dal ragionamento, e questa è la
parte che vale la pena ricordare. Il primo test scritto passava dallo use case
(`ListDocuments`), e passava **anche con la funzione sbagliata**: il controllo
applicativo `ensure_owner_is_visible` rilegge la bolletta sotto la policy e restituisce
`404` prima che la RLS entri in gioco. Lo strato applicativo **mascherava il difetto
invece di dimostrarlo**.

La prova richiede di interrogare `can_access_attachment` **direttamente, sotto il
contesto RLS del coinquilino**, che è ciò che fa `documents_select` quando
l'applicazione non c'è di mezzo. Fatto così, il test fallisce con la definizione della
`0014` e passa con quella della `0022` — riprodotto su database reale in entrambe le
direzioni. È la lezione generale: **un test che passa da uno strato non prova nulla
sullo strato sotto**, e la difesa in profondità che [ADR-016](#adr-016) chiede si
verifica solo interrogando lo strato che si vuole verificare.

**Conseguenze sui dati esistenti: nessuna.** Nessun documento aveva
`owner_type = 'utility_bill'`, perché l'enum di dominio non lo contemplava e il
caricamento lo rifiutava. La correzione **restringe** l'accesso — si vede meno, non di
più — quindi non può aprire nulla che fosse chiuso.

**Quando tornerà disallineato.** Il giorno in cui qualcuno introducesse la bozza vera
dell'addebito: restringere `charges_select` con
`status <> 'draft' OR is_org_staff(org_id)` senza toccare questa funzione lascerebbe
l'inquilino a scaricare gli allegati di una bozza che non vede. Le due migration vanno
insieme, ed è scritto sia qui sia nel commento della `0022`.

---

<a id="adr-032"></a>

## ADR-032 — La vista azzera il residuo per gli stati terminali, e `paid` non è fra loro

**Contesto.** [ADR-028](#adr-028) ha registrato, riproducendolo su database vero, che
`app.v_charge_balances` non guarda `status`: un addebito **annullato** da 120.00
continuava a portare `outstanding_amount = 120.00`, e
`v_owner_dashboard.total_outstanding` rispondeva **960.00** dove il numero vero era
840.00. All'inquilino era stato detto che quei 120 euro non li doveva più, e il numero
di testa della dashboard continuava a contarli come credito. `written_off` fa lo stesso
ed è peggio, perché è una perdita e non un credito.

Il difetto non nasce con la fetta contabile — la vista è così dalla `0015` — ma è
quella fetta a renderlo raggiungibile, perché prima non esisteva alcun addebito. Era il
primo debito della tabella in [09](09-stato-e-roadmap.md#debiti-tecnici-noti), ed era
**l'unico debito del progetto la cui finestra si chiude da sola**: finché nessun client
legge quei campi, cambiarne il significato è additivo; dopo è una rottura.

**Decisione.** La migration `0024` dà alla vista una condizione: quando lo stato è
`void` o `written_off`, `outstanding_amount` vale `0` e `is_past_due` è falso.
`paid_amount` resta quello vero, perché è un fatto e non una pretesa.

Correggere la vista è **il primo passo del modulo dei pagamenti** e non un lavoro a
parte: la domanda «cosa significa insoluto» *è* la domanda di quel modulo, e deciderla
altrove avrebbe voluto dire deciderla due volte.

### `paid` non è nell'elenco, ed è la parte che conta

Due stati soli azzerano, e la tentazione di aggiungerne un terzo è forte perché
`paid` sembra appartenere allo stesso insieme. Non ci appartiene.

Azzerare per stato **dove l'aritmetica è già significativa** inverte il rapporto fra i
due: lo stato memorizzato scavalcherebbe il calcolo fatto sui pagamenti. Se un giorno
`status = 'paid'` e la somma dei pagamenti non torna, quello scarto va **visto**, non
nascosto — in contabilità la deriva fra ciò che una riga dichiara e ciò che i movimenti
dicono è precisamente il difetto che si vuole scoprire. Lo stesso vale per `draft`,
che nessuno scrive.

`void` e `written_off` sono diversi **in natura**: lì non è dovuto niente a prescindere
dai pagamenti, perché la pretesa è caduta. È l'unico caso in cui lo stato è l'autorità
sul residuo, e per questo l'elenco si scrive per esteso invece di derivarlo da
`is_outstanding`.

### Alternative scartate

**Azzerare tutto ciò che non è nell'insoluto**, `draft` e `paid` compresi. Più
simmetrico e peggiore, per la ragione qui sopra.

**Togliere le righe dalla vista.** È la via che sembra ovvia ed è sbagliata:
`find_balance_by_id` serve il dettaglio di un addebito annullato, che l'inquilino
**deve** poter vedere — è l'asimmetria deliberata con la bolletta, che annullata sparisce
— e senza la riga riceverebbe `404` su qualcosa che gli spetta. Un test della fetta
contabile lo fissava già, e sarebbe fallito: il modo giusto di scoprirlo, ma meglio non
scoprirlo affatto.

### Conseguenze

**Una vista sola rimette a posto due numeri.** `v_owner_dashboard` filtra già
`outstanding_amount > 0` e conta su `is_past_due`: non è stata toccata, e non doveva
esserlo.

**L'asimmetria che ADR-028 dichiarava è finita, e il dominio l'ha seguita.** Quel
documento diceva che la formula della vista «non guarda lo stato» mentre
`Charge::effective_status` sì. Ora la vista guarda i due terminali, e
`Charge::is_past_due` fa lo stesso: se non lo facesse smetterebbe di essere *la stessa*
formula, che è l'unica ragione per cui quella funzione esiste. `effective_status` resta
com'era — il suo `is_outstanding()` esclude già più di quanto la vista escluda — e la
sovrapposizione fra i due controlli è voluta, non ridondanza da togliere.

**Sull'API il cambiamento è di significato, non di forma:** `outstanding_amount` di un
addebito annullato passa da `"120.00"` a `"0.00"`. Additivo solo perché nessun client
lo legge ancora.

**`security_invoker` va ripetuto per esteso.** `CREATE OR REPLACE VIEW` conserva le
`reloptions` esistenti quando la clausola manca, ma affidarsi a quella conservazione
significa che il giorno in cui qualcuno ricrea la vista da zero la RLS si apre in
silenzio: la vista girerebbe con i diritti del proprietario e ogni organizzazione
vedrebbe i saldi di tutte. Il piano temeva questo passaggio più di ogni altro e
prevedeva di aggiungere un controllo alle invarianti; il controllo **c'era già** — è il
numero 3, `vista-senza-security-invoker`, che verifica ogni vista di `app` — e la
migration lo ha attraversato verde. È il caso raro in cui una trappola prevista si
scopre già disinnescata, e vale la pena scriverlo perché la prossima volta si guardi
l'elenco delle invarianti prima di temere.

---

<a id="adr-033"></a>

## ADR-033 — La chiave di idempotenza è obbligatoria e la fornisce il client

**Contesto.** `app.payments.idempotency_key` è `NOT NULL` dalla migration `0008`, con
l'indice unico `payments_idempotency_uk` su `(org_id, idempotency_key)`. Lo schema
esigeva una chiave e non diceva **chi** la genera: è la sola domanda di questa fetta in
cui sbagliare costa denaro.

**Decisione.** La chiave la fornisce il **client**, in un header `Idempotency-Key`
obbligatorio sulle due rotte che creano un pagamento. Il service layer la rilegge
**prima** di inserire; se la trova, restituisce il pagamento già registrato con `200`.

### Perché non il server

Se a generarla fosse il server, ogni ritentativo produrrebbe una chiave nuova, quindi
**un secondo pagamento**, quindi un saldo che scende **due volte**. Un doppio invio — un
tap ripetuto, un retry di rete, un client che non ha visto la risposta — dimezzerebbe
l'insoluto senza che nessuno se ne accorga, e la traccia in `audit_log` mostrerebbe due
inserimenti perfettamente legittimi: due righe valide, ciascuna con il proprio autore e
il proprio istante, nessuna anomalia da cercare.

Toglierebbe cioè ogni protezione **proprio dove il danno è denaro**, e lo farebbe in
silenzio. È l'inverso del criterio con cui questo progetto sceglie dove mettere le
invarianti: si difende ciò che, se cede, non si vede cedere.

### Perché un header e non un campo del corpo

È una proprietà della **richiesta**, non del pagamento: la stessa chiave deve poter
accompagnare il medesimo invio ripetuto, e un campo del corpo inviterebbe a rigenerarla
insieme al resto. È anche la convenzione con cui i client la trattano già.

Assente dà `400 request.missing_header` e non `422`: non c'è nulla di malformato nel
corpo, manca un pezzo del trasporto.

### Perché `200` e non `409` sulla chiave ripetuta

Un `409` costringerebbe il client a distinguere «l'ho già mandato io» da «qualcun altro
ha usato la mia chiave», distinzione che **non può fare**. Il `200` con il pagamento
già registrato è la convenzione dell'idempotenza e lascia il client dove voleva essere:
con la risposta che non aveva ricevuto.

**Il `409` resta per due casi che la rilettura non assorbe**, ed è giusto che restino
distinti:

- la stessa chiave verso un **altro** addebito. Non è un doppio invio, è un riuso, e un
  `200` con un pagamento diverso da quello chiesto sarebbe una bugia;
- una chiave già usata da qualcuno di cui il chiamante **non vede** i pagamenti. La
  rilettura gira sotto la RLS del chiamante, quindi non la trova, e l'inserimento viola
  l'indice unico. È irriducibile: distinguere i due casi richiederebbe di leggere una
  riga che al chiamante non spetta. Con chiavi generate dal client — UUID o ULID — la
  collisione è teorica.

Entrambi rispondono `payment.idempotency_key_reused`, che dice al client la sola cosa
azionabile: quella chiave è occupata, ne serve una nuova.

### La cassa non si normalizza

`A1` e `a1` restano chiavi diverse. Il generatore è il client e nulla dice che le tratti
come uguali; confonderle rifiuterebbe come duplicato un pagamento **legittimo**, che è
il danno opposto e peggiore. Si normalizzano invece gli spazi ai bordi e si rifiuta
tutto ciò che non sia ASCII stampabile: il valore arriva da un header, e uno spazio
interno o un byte di controllo non sopravvive intatto al primo proxy — mentre due chiavi
che differiscono per uno spazio sono due pagamenti per l'indice e uno solo per chi
guarda. È lo stesso ragionamento con cui `InvoiceNumber` e `SupplierVat` si
normalizzano perché `utility_bills_dedup_uk` possa fare il proprio lavoro, applicato
dove il costo dell'errore è più alto — con la differenza che lì normalizzare **di più**
era giusto, e qui sarebbe il danno.

### Il complemento: la chiave non si riscrive

L'indice unico protegge dall'inserimento doppio e non dalla **riscrittura**: uno staff
che potesse cambiare `idempotency_key` su una riga esistente libererebbe la chiave per
un secondo inserimento, cioè per lo stesso saldo che scende due volte. La migration
`0025` la mette fuori dalla `GRANT UPDATE`, insieme a tutto ciò che il pagamento
registra — importo, pagante, metodo, riferimento polimorfico — applicando il criterio di
[ADR-026](#adr-026): un pagamento **registra un fatto già avvenuto**, e un fatto non si
corregge riscrivendolo. Le sole colonne concesse sono `status`, `paid_at` e
`reconciled_at`, cioè le tre che il dominio muove.

`paid_at` è fra le concesse **proprio perché il rifiuto la azzera**:
`payments_succeeded_ck` è `status <> 'succeeded' OR paid_at IS NOT NULL` e vincola una
sola direzione — obbliga ad avere la data quando si è riusciti, non a toglierla
muovendosi altrove. È la terza volta che questo schema presenta questa forma, dopo
`tickets_closed_ck` e `utility_bills_validated_ck`: ormai è un pattern, e la regola è
cercarla in ogni `CHECK` scritto come `stato ≠ X OR colonna IS NOT NULL` e assumere che
l'azzeramento tocchi al dominio.

---

<a id="adr-034"></a>

## ADR-034 — I centesimi di resto vanno ai primi in ordine di `user_id`

**Contesto.** Cento euro divisi fra tre coinquilini fanno 33,33 tre volte, e avanza un
centesimo. Il resto **non si può buttare**: il trigger `expense_shares_balanced` della
migration `0009` rifiuta la transazione se la somma delle quote non è esattamente
l'importo della spesa, e lo fa al `COMMIT` perché è `DEFERRABLE INITIALLY DEFERRED`.
Quel centesimo va assegnato a qualcuno, e la scelta di **a chi** è una regola contabile
che due esecuzioni della stessa ripartizione devono dare uguale.

**Decisione.** I centesimi di resto si distribuiscono **uno per uno ai primi
partecipanti**, in ordine di `user_id` crescente. La regola è unica per `equal` e per
`weighted`, e l'aritmetica lavora in **interi** — centesimi e millesimi — senza mai
dividere.

### Perché `user_id` e non un ordine più naturale

Il criterio è la **riproducibilità**: la stessa spesa ricalcolata deve dare le stesse
quote. `joined_at` sembra l'ordine ovvio e non lo è, perché **cambia**: chi esce dal
gruppo e rientra riprende la data del rientro — vedi `ExpenseGroupMember::rejoin`, dove
lasciare la data vecchia violerebbe il senso di `expense_group_members_dates_ck`. Un
ordine che cambia sposta il centesimo di una spesa **già registrata** ogni volta che
qualcuno va e viene, e nessuna riga registra a chi era toccato.

Il nome non serve nemmeno a dirlo: si corregge, e non è unico.

`user_id` è un UUIDv7, quindi ordinarlo equivale a ordinare per data di **creazione
dell'account** — un fatto che non cambia mai. Non è un ordine con un significato per
l'utente, ed è precisamente il punto: deve essere arbitrario e stabile, non equo. La
distribuzione la fa il **numero** di spese, non l'ordine dentro una.

L'ordinamento avviene dentro `Split` e non è preteso dal chiamante: una
`list_members` con un `ORDER BY` diverso non deve poter cambiare chi paga il centesimo,
e il test `the_result_does_not_depend_on_the_order_of_the_input` lo fissa.

### Alternative scartate

**Il resto a chi ha anticipato.** Più semplice da spiegare, e sbagliata sul caso reale:
concentra su una persona sola tutti i centesimi di tutte le spese che ha pagato lei. Su
un anno di spesa quotidiana in una casa dove uno fa la spesa più spesso degli altri non
è più un arrotondamento, è un addebito sistematico.

**A rotazione fra le spese.** La più equa e la peggiore da implementare: richiede uno
stato che ricordi a chi è toccato l'ultima volta — una colonna e una migration — e rende
il calcolo di una quota **non riproducibile** guardando la sola spesa. Si perderebbe
esattamente la proprietà per cui questa decisione esiste.

**Per `weighted`, il resto a chi ha la parte frazionaria più grande.** È la regola
standard dell'apportionment, ed è più «giusta» del nostro ordine arbitrario. Si è
scelta la riproducibilità: una regola sola, un test solo, e nessuna sorpresa nel
passare da un metodo all'altro. Su un centesimo la differenza di equità non esiste; su
due regole da tenere allineate il costo sì.

### Perché in interi, e perché è la parte che conta

**`rust_decimal` non arrotonda da sé, e la sua divisione non è esatta.** Una divisione
produce molte cifre e viene troncata alla precisione massima del tipo, quindi un valore
matematicamente intero può arrivare come `4.999…9`; una `trunc` a due decimali su quel
valore perde un centesimo in più, e la quota risultante dipende da **come il chiamante
ha costruito i numeri**. In contabilità «quasi sempre uguale» è il difetto peggiore,
perché non si manifesta sui casi che si provano — e qui si manifesterebbe come una
transazione rifiutata al `COMMIT`, cioè lontano dalla query che l'ha causata.

Quindi non si divide mai. Gli importi diventano **centesimi** e i pesi **millesimi**,
entrambi interi esatti perché le scale di `app.money_amount` (`NUMERIC(14,2)`) e di
`expense_group_members.weight` (`NUMERIC(6,3)`) lo garantiscono, e tutta l'aritmetica
sta in `i128` — che regge il prodotto fra il massimo importo (10^14 centesimi) e il
massimo peso (10^6 millesimi) senza avvicinarsi al limite.

Ne segue che `Weight` è un value object separato da [`Money`], e non una comodità:
passare un peso come importo accetterebbe due decimali dove ne servono tre. È la stessa
distinzione che `Consumption` fa nelle bollette.

### La conseguenza da sapere leggendo le quote

Una quota può valere **`0.00`**, e lo schema lo prevede:
`app.expense_shares.share_amount` è `CHECK (share_amount >= 0)`, non `> 0`. Un centesimo
diviso fra tre dà `0.01 / 0.00 / 0.00`, e un peso abbastanza piccolo porta a zero la
quota di chi lo ha. Non è un errore da correggere: è la sola ripartizione la cui somma
torna.

---

<a id="adr-035"></a>

## ADR-035 — Un privilegio di colonna può difendere un'invariante che un trigger non copre

**Contesto.** [ADR-026](#adr-026) ha stabilito quando usare i privilegi di colonna: una
`WITH CHECK` che nomina il solo `org_id` lascia libera ogni altra colonna, perché una
policy non può confrontare la riga nuova con quella vecchia. Il criterio è «una colonna
non ha un motivo legittimo di cambiare **e lo schema non ha un vincolo che ne difenda la
coerenza**».

Su `app.expenses` la seconda metà del criterio si legge male, e la lettura sbagliata è
quella che viene naturale: **un vincolo che difende la coerenza c'è**, ed è il trigger
`expense_shares_balanced`, che impone che la somma delle quote sia esattamente
l'importo della spesa. Sembrerebbe quindi il caso in cui il privilegio non serve.

**Decisione.** Il privilegio serve, e la migration `0028` lo introduce. Il criterio di
ADR-026 va letto più stretto: non basta che un vincolo esista, deve **coprire la
scrittura che si sta permettendo**.

### Il buco, per esteso

```
CREATE CONSTRAINT TRIGGER expense_shares_balanced
    AFTER INSERT OR UPDATE OR DELETE ON app.expense_shares
```

Il trigger è su **`app.expense_shares`**. Guarda le quote. Una `UPDATE` su
`app.expenses` che cambia `amount` **non lo invoca**, e lascia la somma delle quote
diversa dall'importo della spesa — cioè viola in silenzio l'invariante che quel trigger
esiste per difendere, passando dalla porta accanto.

Non è un caso limite: `app.tg_expense_shares_balanced` legge `app.expenses.amount` come
**prima cosa** che fa. La spesa è metà dell'invariante, e nessuno la sorveglia.

Riprodotto su database vero prima di scrivere la migration: `UPDATE app.expenses SET
amount = 500.00` su una spesa con 100,00 di quote passa, il commit riesce, e nessuno
protesta. Con la `0028` la stessa scrittura come `propflow_app` fallisce con
`permission denied for table expenses`.

### La regola che se ne ricava

**Un vincolo difende ciò che tocca, non ciò che riguarda.** Prima di concludere che un
privilegio di colonna è superfluo perché «c'è già un trigger», va verificato su quale
tabella il trigger è definito e quali operazioni lo invocano. Un `CONSTRAINT TRIGGER` su
una tabella figlia non protegge la tabella padre, e questo schema ne ha un esempio
esattamente dove i conti sono conti.

Il complemento vale al contrario: `app.expense_shares` **non** ha privilegi di colonna,
ed è corretto — là il trigger scatta a ogni scrittura, e l'invariante è difesa da chi di
dovere.

### La differenza con gli addebiti, che è deliberata

`deleted_at` è fra le colonne **concesse** su `app.expenses` e fra le **negate** su
`app.charges` (migration `0023`), e le due scelte opposte hanno la stessa ragione: si
guarda che cosa la tabella usa per annullare.

- un addebito si annulla con `status = 'void'`, e deve **restare visibile** a chi se
  l'era visto chiedere. Negare `deleted_at` è ciò che gli impedisce di sparire;
- `app.expenses` **non ha una colonna di stato** — verificato, non dedotto — quindi il
  soft delete *è* l'annullamento di una spesa. Negarlo lascerebbe una spesa sbagliata
  senza alcun modo di essere disfatta.

### Perché qui l'elenco è corto, e sulle bollette non lo era

ADR-026 ha dato «no» su `app.utility_bills` perché la `GRANT` avrebbe nominato quasi
ogni colonna: una bolletta si digita, dodici campi alla volta, e finché non è validata
correggerne uno è il lavoro previsto — un elenco che contiene quasi tutto non difende
niente e si paga solo l'onere di manutenzione.

Su `app.expenses` è l'opposto: **cinque colonne concesse** (`description`, `category`,
`expense_date`, `notes`, `deleted_at`) e otto negate, ognuna con una ragione propria.
Fra le negate, due meritano di essere nominate:

- **`amount`** è il buco descritto sopra, ed è la ragione della migration;
- **`utility_bill_id`** è la colonna di `expenses_bill_uk`, indice unico e **parziale**:
  azzerarlo **libera la bolletta per una seconda ripartizione**, cioè per un secondo
  addebito ai coinquilini della stessa spesa. È lo stesso danno della `idempotency_key`
  dei pagamenti ([ADR-033](#adr-033)), su una colonna che oggi nessuno scrive — e il
  fatto che una scrittura futura fallisca con `permission denied` invece di passare in
  silenzio è il promemoria giusto per il giorno in cui il legame entrerà.

Il controllo 12 delle invarianti dello schema verifica l'elenco **al contrario**, e
copre ormai cinque tabelle sotto un nome solo: il criterio è quello di ADR-026 in tutti
i casi, e inventargli un nome nuovo renderebbe illeggibile la ragione comune.

### Il corollario sulla direzione, che la `0029` ha aggiunto

Un privilegio di colonna dice **quali colonne** si possono scrivere e non **in che
verso**. Su `app.expense_group_members` servono entrambe le cose: le tre colonne di
identità non si riscrivono, e un membro non staff non deve poter portare `left_at` da
`NULL` a una data sulla riga di un altro — cioè toglierlo dal gruppo — mentre deve poter
fare l'opposto, perché reintegrare qualcuno è un `UPDATE` sulla riga di un altro.

La direzione la esprime la `WITH CHECK`, che vede la riga **nuova**:
`user_id = app.current_user_id() OR app.is_org_staff(org_id) OR left_at IS NULL`. La
`USING` resta larga, e stringerla sarebbe l'errore: governa quali righe si possono
toccare, e toccare la riga di un altro serve.

È il complemento del criterio, non un'eccezione: quando una policy non può confrontare
la riga nuova con quella vecchia, può però **vincolare la riga nuova da sola** — e
quando ciò che si vuole impedire è una transizione e non una riscrittura, è lì che va
scritto.

---

<a id="adr-036"></a>

## ADR-036 — Il perimetro della casa: una funzione che proietta, non una policy allargata

**Contesto.** Due domande che sembrano una sola, e che lo schema uscito dalla `0014`
sbagliava in direzioni opposte.

Un coinquilino **non aveva modo di sapere chi altro abita casa sua**: in un co-living
per stanza ognuno ha il proprio contratto, `app.can_access_lease` è per contratto, e
l'`user_id` dei coinquilini non compariva da nessuna parte. Il modulo delle spese
co-living era quindi usabile solo con un contratto unico condiviso — chi creava un
gruppo non poteva aggiungerci nessuno finché il proprietario non gli passava gli
identificatori a mano.

Lo stesso coinquilino, allo stesso tempo, **leggeva la riga `app.users` di ogni persona
dell'organizzazione**: `users_select` ammetteva `app.shares_org_with`, e quella riga
porta nome, **email** e **telefono**. Inquilini di immobili diversi, con cui non esiste
alcun rapporto, compresi.

**L'immobile non era il confine, né in un senso né nell'altro.** Sui contratti la
visibilità era più stretta — il singolo contratto — sull'anagrafica più larga.

**Decisione.** Il confine è la **casa**, e in tutte e due le direzioni.

1. I residenti si leggono da `app.property_residents(property_id)` →
   `(user_id, full_name, room_name)`, una funzione `SECURITY DEFINER` che restituisce
   **righe** e ha il controllo di accesso nel proprio corpo. **Nessuna policy di lettura
   si allarga.**
2. `users_select` si restringe da `app.shares_org_with` ad `app.shares_home_with`, con
   due rami sullo staff — `app.is_staff_of_user` e `app.is_my_staff` — che tengono in
   piedi il rapporto proprietario ↔ inquilino.

### Perché non si è allargata una policy, che era la prima idea

Estendere `lease_parties_select` al perimetro dell'immobile riusando
`app.can_access_property` **è stato provato su database vero e non funziona**, per due
ragioni a due profondità diverse. Vanno sapute entrambe, perché la prima nasconde la
seconda e chi risolve solo quella crede di avere finito.

*Prima profondità.* Scritta in linea nella policy, la condizione è un `EXISTS` su
`app.leases`, e **non fa comparire nemmeno una riga in più**: una sottoquery dentro una
policy subisce la Row Level Security della tabella che legge, quindi quel `SELECT` passa
da `leases_select`, falsa proprio per i contratti che si vogliono raggiungere. È la
trappola registrata in testa alla migration `0026`, qui alla **terza** occorrenza — le
prime due sono le `can_access_*` originali della `0014` e `app.created_expense_group`.

*Seconda profondità.* Superata la prima con una funzione `SECURITY DEFINER` che risolve
il contratto nel suo immobile, le righe compaiono — **finché la query non tocca
`app.leases`**:

| | X vede |
|---|---|
| `app.lease_parties` da sola | X, Y, Z ✓ |
| unita ad `app.leases` | **solo X** ✗ |

e unirle è obbligatorio, perché l'immobile e «in essere» stanno sul contratto.
`leases_select` è rimasta `can_access_lease(id)` e rifiltra tutto. **Il sintomo è un
elenco corto senza alcun errore.**

### Il prezzo che non si è pagato, misurato

Far funzionare quella via avrebbe richiesto di allargare **anche** `leases_select`.
Misurato sullo scenario a due immobili: da quel momento X legge `rent_amount` dei
contratti di Y e Z, cioè **il canone dei coinquilini**. Una policy è per riga e non per
colonna, e `leases.rent_amount` sta sulla riga.

Una funzione **proietta**: restituisce tre colonne scelte e nessuna riga in più diventa
leggibile. Ne segue che non c'è alcuna decisione da prendere sull'email — non c'è nulla
da cui ometterla. È il segno che il disegno è migliore e non solo diverso: la prima
stesura del piano trattava l'email come un compromesso da accettare e dichiarare, e non
aveva nemmeno visto il canone.

### La regola generale

**Quando ciò che serve è una proiezione, una funzione che restituisce righe è preferibile
a una policy allargata.** Una policy governa la riga intera e non sa distinguere le
colonne; allargarla per esporre tre campi ne espone venti, e il costo si scopre leggendo
la tabella, non la policy.

Il confine resta comunque **nel database**: la funzione è l'unica via all'elenco e il
controllo è nel suo corpo, non in una regola del service layer che una query scritta a
mano potrebbe aggirare.

### Restringere `users_select` non fa fallire nessun test, e va letto bene

Riducendo `users_select` al solo `id = app.current_user_id()` la suite resta interamente
verde. **Non prova che restringere sia sicuro: prova che la capacità per cui
`shares_org_with` esisteva — «mostrare i nomi delle controparti», dice il commento della
`0014` — non è mai stata costruita.** `app.users` compare in tre query, tutte in
`user.rs`, tutte nei flussi di autenticazione, tutte sul chiamante stesso.

È lo stesso equivoco già registrato nella fetta contabile: un test che passa da uno
strato non prova nulla sullo strato sotto. La verifica vera è la matrice dello scenario a
due immobili, e la finestra per restringere la chiude il primo client che legga un nome
da lì.

Lo schema **denormalizza i nomi dove si mostrano** — `lease_parties.full_name` ed
`email`, `expense_group_members.display_name` — ed è precisamente il motivo per cui
nessuno legge `app.users`. `app.property_residents` proietta da `lease_parties`: le due
metà di questa decisione sono indipendenti, e nessuna ha bisogno dell'altra.

### `shares_home_with` non filtra «in essere», e la differenza è voluta

`app.property_residents` filtra `app.lease_in_force` — l'elenco dice chi ci abita
**adesso** — mentre `app.shares_home_with` no: dice con chi si è **abitato**. Non è
un'incoerenza. Un conto condiviso sopravvive alla scadenza di un contratto, e la
migration `0027` esiste apposta per non perdere di vista chi è uscito con un debito
aperto: restringere anche qui renderebbe illeggibile il nome di una persona che si sta
ancora pagando. `users_select` è un **pavimento**, non un elenco.

### Le due funzioni sullo staff non sono l'una l'inversa dell'altra

`is_staff_of_user(u)` dice «io sono staff di un'organizzazione a cui `u` appartiene»;
`is_my_staff(u)` dice «`u` è staff di un'organizzazione a cui io appartengo». Senza la
prima il proprietario perderebbe di vista i propri inquilini, che non abitano con lui;
senza la seconda un inquilino non potrebbe **mai** sapere il nome del proprio
proprietario, e `properties.owner_id` punterebbe a una riga illeggibile. Scambiarle
produce una policy che sembra funzionare finché il chiamante non è un inquilino.

### Alternative scartate

**Un elenco delle persone dell'organizzazione.** È il perimetro che questa decisione
rifiuta, in entrambe le direzioni: sarebbe la stessa larghezza di `shares_org_with`, con
un endpoint a esporla.

**L'invito diretto a un gruppo di spesa**, che avrebbe evitato del tutto il problema
dell'`user_id`. Aprirebbe una via di ingresso al co-living parallela a quella dei
contratti, e le due divergerebbero.

**Verificare in `AddGroupMember` che la persona sia residente dell'immobile del gruppo.**
Sembra il complemento naturale e non lo è: `expense_groups.property_id` è **opzionale**,
e per giunta `ON DELETE SET NULL`, quindi la verifica avrebbe un'uscita per costruzione
— un gruppo senza immobile non ha nulla contro cui verificare. Una regola che si aggira
omettendo un campo facoltativo non è un confine; è un dosso che si legge come un confine,
ed è esattamente l'errore che la `0029` è servita a smettere di fare. Il gate che conta è
già in `can_access_expense_group`: si aggiunge qualcuno solo a un gruppo di cui si è già
membri. Vedi anche [09 — Stato e roadmap](09-stato-e-roadmap.md#piano-coinquilini-visibili-fetta-1).

**Conseguenze.** `app.shares_org_with` resta definita e non è più usata da alcuna policy;
rimuoverla è un cambiamento a sé. `app.property_residents` è la prima funzione
`SECURITY DEFINER` del progetto a restituire un insieme, e come le altre verifica
**anche** l'organizzazione attiva: bypassare la RLS significa bypassare l'isolamento, non
solo la visibilità per ruolo — il criterio della `0026`, qui alla sua quarta occorrenza.

---

<a id="adr-037"></a>

## ADR-037 — Una porta si definisce prima di avere un fornitore, e a quali condizioni è onesto farlo

**Contesto.** L'OCR delle bollette è il primo modulo in cui una macchina scrive nel
database, e per costruirlo servirebbe sapere quale macchina. La roadmap diceva di
scegliere il fornitore in quel momento — «i trait dei provider vanno definiti allora,
verificando la reference del fornitore scelto» — e questa fetta se ne scosta: definisce
`OcrProvider`, ne scrive un adattatore deterministico, e **non sceglie il fornitore**.

La ragione per cui la scelta non si può fare qui è che **non è una scelta tecnica**.
L'immagine di una bolletta porta fuori dal perimetro fornitore, indirizzo, importi e
spesso il codice cliente. Il commento della migration `0010` è esplicito nel tenere
prompt e immagine fuori da `ai_jobs`, e la fetta lo rispetta — ma il dato al fornitore ci
va comunque, e quella è una decisione di **trattamento**, che spetta a chi risponde del
trattamento e non si deduce mentre si scrive un adattatore.

Le due strade concrete che si sarebbero potute prendere hanno ciascuna un difetto che
non si compensa scrivendo più codice:

| | Cosa dà | Perché non basta decidere adesso |
|---|---|---|
| LLM con visione | legge e struttura in un colpo, e servirebbe anche al triage | l'immagine esce dal perimetro: è la decisione di trattamento |
| Tesseract in locale | niente esce dal perimetro | fa OCR e non estrazione: sulle bollette italiane, con un'impaginazione per fornitore, i campi andrebbero ricavati con euristiche nostre e l'accuratezza sarebbe bassa abbastanza da rendere il modulo un fastidio |

**Decisione.** La porta si definisce ora, il fornitore si sceglie dopo.

Il rischio va nominato perché è reale: **una porta disegnata senza un fornitore davanti
rischia di essere inventata**, cioè di descrivere ciò che è comodo a noi invece di ciò
che i fornitori danno. Le due condizioni che rendono la scelta onesta sono queste, e
nessuna delle due elimina il rischio:

1. **La porta chiede il minimo comune.** Un `&[u8]` con il suo MIME in ingresso; in
   uscita i campi trovati, ciascuno con la propria confidenza, più i contatori che
   `ai_jobs` ha già — modello, token, costo, latenza. Tutto ciò che un fornitore
   specifico offre in più — bounding box, numero di pagina, alternative di lettura —
   resta **fuori**: aggiungerlo dopo è additivo, toglierlo no.
2. **Esiste un adattatore che percorre la porta per intero**, e non un `todo!()`. Se la
   porta fosse impraticabile, si scoprirebbe adesso e non fra sei mesi.

**Il primo adattatore vero è il momento di rileggerla**, ed è scritto nel codice della
porta perché chi lo scriverà lo trovi. Se si rivelasse sbagliata, cambiarla allora costa
un adattatore finto e nessun client.

### Cosa compra una fetta che non produce valore per un utente

Va detto invece che lasciarlo scoprire: **questa fetta non produce valore per un utente
finale**. Nessuno vedrà una bolletta letta da una macchina finché il fornitore non
esiste.

Quello che compra è il **percorso** — coda, worker, job, revisione — che è indipendente
dal fornitore ed è dove stanno le trappole: la policy mancante su `ai_jobs`, lo stato
`pending_review` che era un vicolo cieco, il claim che va marcato prima di lavorare, il
confine fra ciò che si scrive sul job e ciò che si scrive sulla bolletta. Nessuna di
queste si sarebbe risparmiata scegliendo prima il fornitore, e tutte sarebbero state
scoperte con un fornitore vero davanti, cioè nel momento peggiore.

### L'alternativa scartata: aspettare, e non fare la fetta

È la scelta onesta se il modulo servisse a un utente domani. Non serve: le bollette si
registrano a mano e continuano a funzionare. E rimandare avrebbe avuto un costo suo — la
decisione sul fornitore sarebbe stata presa con addosso la pressione di far partire il
modulo, che è la condizione peggiore per una decisione sul trattamento dei dati.

### Il precedente, e la differenza

`ObjectStorage` fu dichiarata il primo giorno e implementata molto dopo, e nessuna riga
della porta è stata riscritta per accomodare S3: è la prova che il metodo funziona, ed è
la ragione per cui `docs/06-sviluppo.md` §Aggiungere un provider esterno lo prescrive.

La differenza è che lì il rimando era **tecnico** — non serviva ancora un bucket — e qui
è **di responsabilità**: la decisione esiste, è matura, e non spetta a chi scrive il
codice. È una distinzione che vale la pena tenere, perché rende la regola applicabile
anche quando il rimando non è comodo: si dichiara la porta quando la forma
dell'astrazione è chiara *e* si sa chi deve decidere il resto. Se non fosse chiara
nemmeno la forma, dichiararla sarebbe inventarla.

**Conseguenze.** `DeterministicOcrProvider` legge bollette **in chiaro** e rifiuta le
immagini: è la verità su cosa sa fare, ed è anche il ramo di errore che i test devono
poter esercitare. `ai_jobs.cost_amount` e `tokens_*` restano nulli, quindi il tetto di
spesa per organizzazione — per cui `ai_jobs_cost_ix` esiste — non è tarabile e resta
fuori perimetro. `provider_name` è un **dato** e non un ramo del codice, dichiarato
dall'adattatore con `OcrProvider::name`: il giorno in cui i fornitori saranno due, i job
dei due si distinguono con una `WHERE` e non con un deploy separato.
