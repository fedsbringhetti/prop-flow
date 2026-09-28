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

**Nota del 22 settembre 2026 — tre regole in più, dallo stesso lato del criterio.**
La sezione 12 ha aggiunto ciò che un contratto in essere vieta su un immobile, e
tutte e tre stanno nel service layer per la ragione di «almeno un owner»: il legame
è **cross-row** — il vincolo vive su `app.leases` e la scrittura avviene su
`app.properties` — e un trigger che lo esprimesse dovrebbe interrogare un'altra
tabella a ogni `UPDATE`.

- **in bozza non si affitta**: `ActivateLease` rilegge l'immobile e rifiuta su
  `draft` con `Invariant::PropertyNotLettable` → `property.not_lettable`;
- **con un contratto in essere non si torna in bozza**: `UpdateProperty` rifiuta
  `status: draft` con `Invariant::PropertyHasLeaseInForce` →
  `property.lease_in_force`;
- **con un contratto in essere non si archivia**: `ArchiveProperty` rifiuta con lo
  stesso invariante.

Come per «almeno un owner», il rifiuto è **parlante** e non una policy che nega: la
domanda passa da `LeaseRepository::has_in_force_for_property`, che interroga
`app.lease_in_force` — la stessa funzione delle viste e dell'elenco, perché due
definizioni di «in essere» farebbero rispondere l'archiviazione e la dashboard in
modo diverso sullo stesso contratto. Che non ci sia una rete nel database è una
conseguenza accettata: una `UPDATE` scritta a mano su `app.properties` aggira le
tre regole, come aggirerebbe «almeno un owner».

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

**Emendata da [ADR-051](#adr-051):** la prima delle tre scritture si salta quando la
membership esiste già ed è attiva. L'ordine resta questo, perché a rendere falso
`has_pending_invitation` è l'ultima scrittura e non la prima.

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

**Aggiornamento dell'8 settembre 2026: adesso lo carica `just`, e la decisione non
cambia.** Il `justfile` ha `set dotenv-load`, quindi `just api` e `just worker`
leggono `.env` da sé e il passo `set -a; source .env; set +a` serve solo a chi lancia
un binario a mano. **Non è una terza sorgente di configurazione** e non tocca niente
di ciò che è deciso qui sopra: nessun crate dipende da `dotenvy`, il processo continua
a non leggere quel file, e `just` lo carica **fuori** dal processo esattamente come
faceva la shell — produce variabili d'ambiente come tutte le altre. È cambiata
l'ergonomia, non una proprietà del sistema, ed è per questo che l'aggiornamento è una
nota e non un ADR nuovo.

L'occasione per scriverlo è stata `R3`: l'affermazione «`.env` lo carica la shell»
viveva in **otto** posti — il messaggio d'errore di `AppConfig` e il commento del tipo,
`CLAUDE.md`, `README.md`, `.env.example`, due righe di [06](06-sviluppo.md), due di
[07](07-operazioni.md) e questo ADR — ed è invecchiata in tutti insieme il giorno in cui
il `justfile` è entrato.

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
([11 — Piani conclusi](11-piani-conclusi.md#piano-documenti-fetta-1)) aveva già
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

> **`title` e `description` non sono più fuori.** La migration `0036` le riconcede, e il
> perché è in [ADR-048](#adr-048), che **emenda questa sezione invece di riscriverla**: il
> criterio dichiarato più sotto non cambia — cambia il fatto, perché ora quelle due
> colonne hanno un motivo legittimo di cambiare. Tutte le altre restano fuori, e per loro
> vale ancora «si annulla e si riapre».

**Il criterio, dichiarato.** Lo strumento serve in due casi, non in uno:

1. quando una policy concede scrittura **oltre il confine** di isolamento (ADR-021);
2. quando una colonna **non ha un motivo legittimo di cambiare** e lo schema non ha
   un vincolo che ne difenda la coerenza. Allora il privilegio di colonna è la sola
   difesa disponibile, e costa una riga.

Il secondo caso è quello delle segnalazioni, ed è quello che rende la scelta quasi
gratuita: una segnalazione aperta sull'immobile sbagliato **si annulla e si riapre** —
`cancelled` esiste per questo — e non si corregge spostandola. Resta vero per l'immobile,
la stanza e il contratto; **non** per il testo, che [ADR-048](#adr-048) ha reso
correggibile perché un refuso non è una riga incoerente.

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
membri. Vedi anche [11 — Piani conclusi](11-piani-conclusi.md#piano-coinquilini-visibili-fetta-1).

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

---

<a id="adr-038"></a>

## ADR-038 — Una sola origine, il refresh in un cookie, e nessuna difesa CSRF

**3 settembre 2026.** Il client web ha bisogno di una sessione in un browser. La
scelta di dove tenere i due token decide tre cose insieme — che cosa un XSS può
rubare, se serva CORS, e se serva una difesa CSRF — e conviene deciderle una volta
sola perché sono la stessa decisione vista da tre lati.

### La decisione

**Una sola origine.** `/` serve i file statici del client, `/api/v1` inoltra
all'API. In sviluppo lo fa il proxy di Vite, in produzione un reverse proxy —
`ops/web/Caddyfile` nel compose, un Ingress in Kubernetes.

**Il refresh token vive in un cookie** `HttpOnly; Secure; SameSite=Strict;
Path=/api/v1/auth`, e **solo** lì: `SessionResponse` non lo porta più, e le
richieste di refresh e di logout non lo accettano nel corpo. **L'access token vive
in memoria** JavaScript, e il client lo manda come `Authorization: Bearer`.

**Non c'è nessuna difesa CSRF**, e non è una dimenticanza.

### Perché il token non può stare in due posti

`HttpOnly` esiste per impedire a uno script di leggere il cookie. Se lo stesso
valore comparisse anche nel JSON del login, uno script lo prenderebbe da lì senza
dover toccare `document.cookie`: sarebbe metà misura a costo pieno. Trenta giorni
di sessione sono precisamente ciò che non deve essere leggibile da un XSS che
arrivi dopo il caricamento della pagina.

L'access token in memoria muore al ricaricamento, e non è un problema: si riottiene
dal cookie. Non sta in `localStorage` né in `sessionStorage`, quindi non è
leggibile da uno script che arrivi dopo.

**Alternativa scartata: entrambi in `localStorage`.** Nessuna modifica al backend,
e in cambio un XSS legge un refresh token che vale trenta giorni. Su un prodotto
che tratta contratti di locazione e importi è un rischio che si può accettare solo
dicendolo, e non c'è ragione di accettarlo.

**Alternativa scartata: un parametro che scelga fra cookie e corpo.** Un flag
sull'autenticazione è una superficie in più da sbagliare, e sposta su chi chiama
una decisione di sicurezza che deve essere del server.

### L'argomento per cui il CSRF non serve, per intero

Il CSRF è possibile quando il browser autentica una richiesta **da solo**, cioè con
un cookie che manda senza che nessuno glielo chieda. Qui non accade, per due
ragioni che si sommano:

- **ogni rotta di business è autenticata dall'header `Authorization`**, che un sito
  attaccante non può impostare su una richiesta cross-origin senza il permesso di
  CORS — che non concediamo, perché CORS **non è montato affatto**;
- **le sole rotte autenticate dal cookie sono `/auth/refresh` e `/auth/logout`**, e
  su quelle il cookie è `SameSite=Strict`: il browser non lo manda su una richiesta
  che parte da un altro sito. Non c'è niente da falsificare.

`Path=/api/v1/auth` non è cosmetico: limita l'invio del cookie alle sole rotte che
lo usano, quindi una richiesta a `/api/v1/properties` non se lo porta dietro nemmeno
per sbaglio. È difesa in profondità — se un giorno l'argomento sopra si incrinasse,
il cookie comunque non arriverebbe dove non serve.

### La condizione che lo tiene in piedi, e perché è un test

**L'argomento regge finché nessuna rotta di business è autenticata da un cookie.**
Il giorno in cui qualcuno ne aggiungesse una per comodità — una sessione a cookie
«per semplificare un endpoint» — tutto questo cadrebbe, e cadrebbe **in silenzio**:
nessun test fallirebbe, nessun avviso comparirebbe, e la falla si scoprirebbe
soltanto da fuori.

Per questo esiste `the_session_cookie_is_refused_by_business_routes` in
`crates/api/tests/http_contract.rs`. Presenta un cookie **valido** a cinque rotte di
business senza `Authorization` e pretende `401 auth.token_missing` da tutte. Non
verifica una funzionalità: verifica che una porta resti chiusa, ed è l'unica forma
in cui una nota di sicurezza sopravvive a chi non l'ha letta.

### Che cosa la singola origine fa risparmiare

Quasi tutte le conseguenze sono cose che **non** si devono fare, e vale la pena
elencarle perché sembravano lavoro obbligatorio:

- **CORS non va montato.** Non c'è nessuna richiesta cross-origin da autorizzare.
  Resta il debito registrato in [09](09-stato-e-roadmap.md) per il giorno in cui un
  client di terze parti esisterà, e quel giorno sarà una decisione sua;
- **il cookie è di prima parte**, quindi `SameSite=Strict` invece di
  `SameSite=None; Secure` — la differenza fra un cookie che il browser non manda
  mai altrove e uno che manda ovunque glielo si chieda;
- **nessun elenco di origini ammesse** da tenere allineato fra ambienti, che è una
  configurazione che si sbaglia in silenzio e si scopre in produzione.

**Alternativa scartata: origini separate** (`app.` e `api.`). Più comoda da
distribuire in modo indipendente, e costa tre pezzi di backend — CORS con
credenziali, cookie `SameSite=None`, difesa CSRF — ciascuno sbagliabile senza che
nulla protesti. Si comprerebbe indipendenza di deploy pagandola in superficie di
sicurezza.

**Alternativa scartata: i file statici serviti dall'API Rust.** Un servizio solo e
nessun proxy da configurare, ed è tentante. Costa però che la build del frontend
finisca nell'immagine dell'API, e che un processo HTTP faccia due mestieri con
profili di carico e di cache opposti — file immutabili con l'hash nel nome da un
lato, risposte personali non memorizzabili dall'altro. Il proxy va scritto comunque
per Kubernetes.

### Conseguenze

**È una rottura del contratto dell'API**, ammessa sulla linea 0.x: un client che
leggeva `refresh_token` dal corpo smette di trovarlo, e chi usa `curl` passa a un
cookie jar. Gli esempi di [05 — API](05-api.md#autenticazione) sono aggiornati.

`PROPFLOW__SERVER__SECURE_COOKIES` è **l'unica configurazione che questa fetta
introduce**, e il default è `true`: un ambiente che dimentica di dichiararla ottiene
il comportamento sicuro. Va portata a `false` solo in sviluppo su
`http://localhost`, dove il browser non manda un cookie che dichiara `Secure` — il
refresh fallirebbe sempre, e il sintomo (una sessione che dura quindici minuti) non
nomina la causa.

**Il logout senza cookie resta `204`.** Non c'è nulla da revocare, il cookie si
cancella comunque, e costringere un client che l'ha già perso a distinguere due
esiti che per lui sono lo stesso non lo aiuterebbe.

---

<a id="adr-039"></a>

## ADR-039 — Il client è una web app responsive, e non un'app mobile nativa

**3 settembre 2026.** `docs/09` elencava fra i trasversali mancanti un «client
mobile (Flutter o React Native), fuori perimetro backend». Questa decisione lo
rivede, e lo scrive qui perché una revisione che resta in una conversazione non
esiste per chi legge il documento vecchio.

### Perché l'ipotesi era stata scritta, e perché non regge

Era stata scritta **prima che il perimetro dell'API fosse noto**, quando «PropTech
bidirezionale» faceva pensare a due app. Rivista guardando che cosa fa davvero un
proprietario: registra una bolletta con dodici campi, legge tabelle di addebiti,
confronta la proposta di una macchina con i numeri che ha digitato, riaddebita a un
contratto. È lavoro da schermo grande e da tastiera, e su un telefono sarebbe
peggiore in ogni singolo passaggio.

L'inquilino invece è mobile per natura — fotografa un guasto, guarda quanto deve — e
una web app responsive lo copre bene. Le due cose che richiederebbero davvero
un'app nativa sono le **notifiche push** e un accesso alla fotocamera più fluido di
`<input capture>`: la prima non è nemmeno possibile finché `Notifier` non ha un
adattatore, la seconda non è nella fetta 1.

**Un codice solo copre entrambi i ruoli**, e su un progetto che ha già un backend
in Rust con undici moduli, un secondo linguaggio e un secondo ciclo di rilascio
sono un costo che va giustificato da un bisogno, non da un'ipotesi.

### La decisione

**React, TypeScript in `strict`, Vite**, in `apps/web/` dentro questo repository.
`shadcn/ui` su Radix e Tailwind per i componenti.

**Monorepo e non repository separato.** Il contratto dell'API e il client si
muovono insieme: una rotta che cambia e un client che non la segue si vedono
**nella stessa pull request**, che è esattamente il difetto che un repository
separato lascia scoprire tardi. Il costo è un filtro di percorso in CI, che è
`.github/workflows/ci.yml` §`changes`.

**Componenti copiati e non una libreria chiusa.** Un gestionale richiede tabelle
con ordinamento e filtri, form con validazione per campo, date che capiscano il
formato italiano; con una libreria chiusa ognuna di queste si personalizza
combattendo. Radix sotto garantisce accessibilità e gestione della tastiera, che è
la parte che nessuno riscrive bene a mano — e infatti il progetto usa Radix dove il
browser non offre niente (modali, menù) e i controlli nativi dove li offre
(`<select>`).

**Alternativa scartata: Tailwind nudo.** Controllo totale, e accessibilità, focus e
ARIA su modali e menù da scrivere a mano: è il modo più rapido di costruire
un'interfaccia che *sembra* professionale e non lo è.

**Alternativa scartata: MUI.** Il `DataGrid` risolverebbe da solo metà delle
schermate del proprietario, e il prodotto somiglierebbe a un'app Google salvo un
lavoro di tema considerevole; le parti da gestionale più utili sono per giunta a
pagamento.

### L'interfaccia è in italiano, e le stringhe stanno in un catalogo dal primo giorno

Gli utenti sono proprietari e inquilini italiani. Non contraddice la regola
sull'inglese di `CLAUDE.md`, che riguarda ciò che un estraneo incontra **dal di
fuori del repository** — commit, pull request, README — non ciò che un utente legge
nel prodotto.

**Ne segue un vincolo che vale dal primo giorno, ed è la parte che conta: nessuna
stringa visibile va scritta dentro un componente.** La traduzione in inglese è già
decisa e soltanto rimandata, e una fetta costruita con i testi nel JSX la rende una
riscrittura invece di un'aggiunta. Le stringhe stanno in `src/i18n/it.ts`, i
componenti ne usano le chiavi, e la lingua è una sola finché non ce ne sono due.

**Non serve una libreria di internazionalizzazione nella fetta 1** — servirà per
plurali e genere. Serve **il livello di indirezione**, che è la parte impossibile
da aggiungere dopo. Per la stessa ragione date, importi e numeri passano da
`Intl.DateTimeFormat` e `Intl.NumberFormat` **con la lingua come parametro**:
formattarli a mano lascerebbe `31/08/2026` e `184,20 €` anche nella versione
inglese.

### Quando questa decisione va rifatta

**Quando l'inquilino sarà il fuoco**, cioè nella fetta 2, e con davanti le due cose
che oggi non ci sono: le notifiche push, che richiedono un adattatore di
`Notifier`, e la fotocamera. È scritto qui perché chi farà quella fetta lo trovi
invece di riderivarlo.

---

<a id="adr-040"></a>

## ADR-040 — La perdita è un numero a sé, ed è il residuo

**4 settembre 2026.** Chiude le due semantiche che la fetta della dashboard aveva
lasciato aperte, e che erano scritte nel tipo `OwnerDashboard` perché chi le avrebbe
decise sarebbe arrivato da lì. La finestra in cui cambiarle era additiva la chiude
il primo client: questa è quella occasione.

### Il difetto: un numero che scende senza dire perché

`app.v_owner_dashboard` portava otto numeri e in nessuno si vedeva una perdita.
`void` e `written_off` escono entrambi da `total_outstanding` e da
`overdue_charges` — dalla migration `0024`, che è corretta — e da lì in poi sono
**indistinguibili**. Il proprietario vedeva l'insoluto scendere e non aveva modo di
sapere se fosse sceso perché qualcuno aveva pagato o perché si era rinunciato a
incassare.

I due stati sono diversi in natura, e la distinzione era già scritta nel dominio:
`void` dice «non doveva essere chiesto», `written_off` dice «era dovuto e ci
rinunciamo». Solo il secondo è una perdita, e la dashboard è il primo report.

### La decisione, che sono tre cose e non una

**1. Un nono numero, `written_off_total`** (migration `0032`), sommato sui soli
addebiti `written_off`. Si **aggiunge** e non ridefinisce: `total_outstanding`
continua a dire quanto è ancora dovuto. Aggiungere un campo è additivo; cambiare
cosa significa un campo che un client legge già è una rottura.

**2. La perdita è il residuo, non l'importo.** Un addebito da 800 € con 300 €
incassati e poi stornato è una perdita di **500**: gli altri trecento sono entrati.
La vista somma quindi `amount - paid_amount`.

**`outstanding_amount` non poteva produrlo**, ed è la parte che merita di essere
scritta: la `0024` lo azzera proprio su `written_off`. È la **stessa colonna** che
rende questo numero necessario — è azzerandola che la perdita è sparita dalla vista
— e che non può calcolarlo. Chi ci arriva cercando la via breve la trova, e non
funziona.

Nessun `GREATEST(..., 0)`: un sovrapagamento su un addebito poi stornato dà una
perdita negativa, e troncarla nasconderebbe uno scarto che la contabilità esiste per
scoprire. È lo stesso argomento con cui la `0024` tiene `paid` fuori dagli stati che
azzerano.

**3. Lo storno si estende a `partially_paid`, e l'annullamento no.** Senza questo,
il nono numero avrebbe contato **metà** delle perdite per una ragione strutturale:
`Charge::write_off` era ammesso dal solo `issued`, quindi un addebito su cui fosse
entrato del denaro non era stornabile. E il caso non è di scuola — l'inquilino che
versa trecento euro su ottocento e sparisce è precisamente il caso in cui una
perdita si materializza.

Un numero che dice meno del proprio nome è peggio di un numero assente, perché chi
lo legge non ha modo di sapere che è parziale. Le due cose stanno quindi nella
stessa fetta.

**`void` resta al solo `issued`**, e l'asimmetria è deliberata: annullare un
addebito su cui è entrato del denaro significherebbe che qualcosa è stato ricevuto
contro una pretesa che non è mai esistita, e a quella domanda questo modello non
risponde. Lo storno non ce l'ha — l'incassato resta incassato e si rinuncia al resto
— ed è la via che il client indica al posto del bottone che toglie.

Nessuna migration per il lato dominio: la `0023` concede già l'`UPDATE` sulla sola
`status`.

### Il credito da sovrapagamento si evidenzia, e non si filtra

Era la seconda semantica aperta. Un `outstanding_amount` negativo è un credito verso
l'inquilino, e la vista lo esclude dal totale con `WHERE outstanding_amount > 0`.

**Resta fuori dalla dashboard**: è un cruscotto di ciò che è dovuto, e un credito non
è un insoluto negativo. Compensarlo cambierebbe il significato di un numero che un
client legge già — quindi una rottura — e nasconderebbe esattamente lo scarto che la
contabilità esiste per scoprire.

**Si mostra per addebito**, nell'elenco e nella scheda, con la parola accanto al
numero e non con il solo colore.

**Filtrarlo lato server non si può**, e va detto perché sembra un'omissione: un
sovrapagamento porta lo stato ricalcolato a `paid`, quindi `?outstanding=true` lo
esclude per costruzione, e `ChargeQuery` non ha nulla sul segno del residuo. Un
filtro vero vorrebbe un parametro nuovo e un indice parziale che lo regga, cioè il
lavoro che `charges_outstanding_ix` è costato per l'insoluto. È **rinviato con la
sua ragione**: il caso d'uso «mostrami tutti i crediti» non è ancora nato, e
costruire l'indice prima del caso è il modo di costruire quello sbagliato.

### La conseguenza operativa: `CREATE OR REPLACE VIEW` riscrive tutto

Trovata sbagliando, e la si scrive perché il sintomo è muto. La `0032` è partita dal
corpo della `0015`, che è il primo che si trova cercando
`CREATE VIEW app.v_owner_dashboard`, e ha così **rimesso in piedi il difetto che la
`0017` aveva corretto**: `status = 'active'` al posto di
`app.lease_in_force(...)` in tre punti, cioè contratti scaduti di nuovo contati fra
quelli in essere e i loro canoni di nuovo sommati.

La migrazione non ha protestato — non aveva motivo — e il sintomo sarebbe stato due
numeri gonfi. L'ha vista un test di persistenza che la `0017` si era lasciata dietro
apposta.

**Ne segue una regola per ogni prossima sostituzione di vista: il corpo si riprende
dall'ultima migration che l'ha toccata, non dalla prima che l'ha creata.**

### Il filtro parametrico non usa un indice parziale su un enum

L'altra conseguenza misurata, e vale oltre questa vista. `GET /api/v1/payments`
porta con sé `payments_pending_ix`, parziale su `status = 'pending'`. Il pattern che
ogni altro elenco di questo progetto usa —
`($1::text IS NULL OR status = $1::app.payment_status)` — su un banco di 80 000
pagamenti **non lo tocca**: produce un `Bitmap Index Scan` su
`payments_org_status_ix` seguito da un `Sort`.

La causa non è la selettività. Il cast da `text` a un enum passa da `enum_in`, che
PostgreSQL dichiara `STABLE` e non `IMMUTABLE`: `'pending'::text::app.payment_status`
non viene ridotto a una costante nemmeno nel piano *custom*, e senza una costante il
planner non può dimostrare che il predicato della query implica quello dell'indice.

È la stessa conclusione di `charges_outstanding_ix` per una ragione diversa — là il
problema è che da un `bool` non si deriva *quali* valori l'indice contenga — e lo
stesso rimedio: **il predicato si scrive per esteso**. Il repository aggiunge una
condizione ridondante, accesa da un booleano che vale `true` esattamente quando il
filtro chiede `pending`. Con quella il piano diventa
`Index Scan using payments_pending_ix`, e ci resta anche dopo le sei esecuzioni che
portano al piano generico. Su ogni altro stato il ramo è spento e il risultato non
cambia, quindi `?status=` resta un filtro generale.

### `payer_user_id` resta facoltativo nell'API e diventa obbligatorio nel form

Il debito diceva «con il primo client si vede se l'attribuzione debba diventare
obbligatoria, o se convenga dedurla dalle parti del contratto». Costruendolo si è
visto che **nessuna delle due**, e che il problema stava altrove.

**Obbligatorio nell'API sarebbe sbagliato**: un bonifico da un conto che non si sa a
chi ricondurre esiste, e togliere il campo nullo lascerebbe l'API senza un modo di
dirlo — non c'è un valore sentinella per «deliberatamente non attribuito» che non sia
`NULL`.

**Dedurlo dalle parti del contratto non si può**: un contratto ha spesso più
inquilini con le loro quote, quindi «il» pagante non è determinato; e una parte senza
`user_id` è un invitato che non ha ancora accettato, a cui attribuire un incasso è
impossibile perché la policy confronta un identificatore che non esiste ancora.

**Il difetto vero è che ometterlo era il default.** Il rimedio sta nel form: nessun
vuoto inviabile, un elenco di chi ha accettato, e una voce «non attribuito» che
porta scritta la sua conseguenza — che chi ha pagato non vedrà quella riga nel
proprio elenco, pur vedendone l'effetto sul residuo. Un campo dimenticato diventa
così una scelta dichiarata, senza togliere una capacità legittima.

Quando **nessuno** ha ancora accettato l'invito resta la sola voce «non attribuito»,
con la ragione: è uno stato vero che l'API non può distinguere dalla distrazione, e
il client lo dice invece di offrire un menù di nomi che darebbero tutti lo stesso
esito.

Il debito è quindi **chiuso**, e la colonna resta com'è.

---

<a id="adr-041"></a>

## ADR-041 — «Quali organizzazioni sono mie» non appartiene a nessuna organizzazione

**4 settembre 2026.** Chiude il debito «nessun endpoint elenca le organizzazioni di
un utente», che la roadmap indicava come la prima cosa che un client avrebbe
incontrato. La fetta dell'inquilino ci è arrivata sopra al primo passo.

### Il difetto: un selettore che mostra due UUID

Il guscio aveva un menù dell'organizzazione attiva, ed è deliberato che ci sia — senza
`X-Org-Id` corretto gli elenchi rispondono `200` con zero elementi, cioè una schermata
vuota senza errore, e avere l'organizzazione scritta in alto rende quel caso
diagnosticabile in due secondi invece che in mezz'ora.

Ma **quel menù non poteva mostrare un nome**, perché nessun endpoint restituiva il
nome di un'organizzazione a chi non ne fosse staff: mostrava il ruolo e
l'identificatore grezzo. Per un proprietario, che ne ha una, era indistinguibile da un
elenco vero. **Per un inquilino sono due UUID fra cui indovinare**, ed è la differenza
fra un prodotto e una cosa che si sa usare solo se la si è scritta.

### La decisione

`GET /api/v1/organizations` → `{ items: [{ id, name, slug, role }] }`, autorizzata con
`ensure_authenticated`.

**`ensure_authenticated` e non `ensure_member`, ed è la parte che conta.** Chiedere
un'organizzazione attiva per poter domandare quali si hanno è **circolare**, e rompe
esattamente il caso che l'endpoint esiste per servire: chi non ne ha una selezionata,
o non ne ha più.

Non è una speranza, ed è stato verificato **prima** di deciderlo, perché è ciò che lo
rende possibile:

- `app.member_role(p_org_id)` interroga `app.org_members` filtrando su
  `app.current_user_id()` e sull'organizzazione **passata come argomento**;
  `app.current_org_id()` non vi compare;
- `organizations_select` è `app.is_org_member(id)`;
- `org_members_select` è `user_id = app.current_user_id() OR app.is_org_member(org_id)`;
- `app.set_request_context` accetta già un `p_org_id` nullo.

La query gira quindi sotto la Row Level Security del chiamante **con un contesto privo
di organizzazione**, e restituisce le proprie. `ensure_member` avrebbe funzionato quasi
sempre — il token porta l'organizzazione predefinita — e sarebbe fallito esattamente
quando serve. Il test di contratto verifica la chiamata **senza `X-Org-Id`**: è la
decisione verificata invece che dichiarata.

### Conseguenze e forma

**Nessun cursore.** Le organizzazioni di una persona sono poche per costruzione — la
propria, più quelle di chi le affitta casa — e una paginazione su un elenco che sta in
una riga è cerimonia. È la stessa scelta di `GET /properties/{id}/residents`.

**Senza prefisso `me`.** La Row Level Security restringe già alle proprie, e il
percorso dice il vero senza aggiungere una parola.

**Un join e non un ciclo di letture.** `OrgMemberRepository::list_organizations_for_user`
unisce `app.org_members` e `app.organizations` in una query sola: comporre la coppia
`(Organization, OrgMember)` avrebbe richiesto una `find_by_id` per appartenenza, cioè
un `N+1` su una domanda che una query risolve. La proiezione
`identity::OrganizationMembership` sta accanto agli aggregati come `PropertyResident`
sta accanto all'immobile, e per la stessa ragione: non ha invarianti da difendere.

**Solo le appartenenze attive**, e non per prudenza: `app.member_role` esige
`status = 'active'`, quindi un'appartenenza sospesa o lasciata comparirebbe nel menù e
poi non concederebbe una sola riga — di nuovo la schermata vuota senza errore.

**Il `localStorage` del client si restringe a una cosa sola:** *quale* organizzazione
era attiva. Era un elenco perché non c'era un endpoint; ora è una preferenza di quel
browser, ed è la distinzione che il commento in `session.tsx` già dichiarava e che
questo endpoint rende vera. Il ruolo si memorizza accanto perché serve **prima** che la
rete risponda — il guscio decide su di esso che cosa mostrare — e non è autorevole:
l'elenco vero lo corregge appena arriva, e corregge anche l'organizzazione attiva
quando non è più fra le proprie. Senza quella correzione ogni richiesta successiva
porterebbe un `X-Org-Id` che l'API rifiuta, cioè un'applicazione intera che risponde
`403`.

**Alternativa scartata: allargare `SessionResponse`.** Il login avrebbe potuto
restituire le appartenenze con i nomi. Non risolve il caso in cui l'elenco cambia
mentre la sessione vive — si accetta un invito e il menù resta vecchio fino al
prossimo login — e lega un dato che si rilegge a un'operazione che si fa una volta.

---

<a id="adr-042"></a>

## ADR-042 — Il ramo sul ruolo sta in un posto solo, e non è un confine di sicurezza

**4 settembre 2026.** Nasce da un difetto che si vede **provando** e non leggendo: prima
di questa fetta un inquilino che fosse entrato nel client non avrebbe incontrato una
schermata mancante, **ne avrebbe incontrate sei sbagliate**.

### Il difetto: le letture funzionano, le azioni no

`ListProperties`, `ListLeases`, `ListCharges`, `ListUtilityBills` sono tutte
`ensure_member`, e le policy restringono ai propri: un inquilino le chiama e riceve
`200` con i suoi dati. Il sintomo non era quindi una pagina vuota, ma sei pagine che
sembravano funzionare finché non si premeva qualcosa — «Registra un immobile», «Emetti
un addebito», «Registra un incasso» rispondevano tutte `403`. E la pagina iniziale era
la dashboard, che è `ensure_staff`: si atterrava su un errore.

### La decisione

La navigazione e la rotta iniziale sono **funzioni del ruolo**, e il ramo vive in un
posto solo: `useIsStaff()`, letto dalla sessione. I punti in cui si usa si contano
sulle dita — la navigazione, la rotta iniziale, e i bottoni di scrittura delle
schermate esistenti — e non c'è un `if` per componente.

**Non è un confine di sicurezza, e non va scambiato per tale.** A negare sono l'API e
la policy di riga dietro; qui si evita di *offrire* ciò che verrebbe rifiutato, che è
un'altra cosa. La differenza si perde facilmente, ed è scritta nel codice accanto al
ramo per questo.

**Alternativa scartata: nascondere le azioni schermata per schermata mentre si
costruisce.** Il ramo sul ruolo sarebbe finito scritto dodici volte, e la tredicesima
sarebbe stata dimenticata — che è precisamente come si costruisce un bottone che
risponde sempre `403`.

### Che cosa vede chi abita, e perché quelle quattro voci

*Addebiti*, *Segnalazioni*, *Contratto*, *Bollette*. Ognuna è una scelta:

> **Dalla fetta B-bis le voci sono cinque**: si aggiunge *Spese*, e resta sua soltanto.
> Il perché è in [ADR-043](#adr-043), che emenda questa sezione invece di riscriverla.

- **il riepilogo esce** perché gli risponde `403 org.staff_role_required`, e non per la
  Row Level Security: `GetOwnerDashboard` usa `ensure_staff`, che **stringe** la policy
  invece di ripeterla — la vista passa da `organizations_select`, che è
  `is_org_member`, quindi senza quel controllo un inquilino leggerebbe i numeri
  dell'organizzazione intera. Una voce che porta a un errore è peggio di una che manca;
- **gli immobili escono, e la ragione non è la sicurezza**: `ListProperties` è
  `ensure_member` e gliene mostrerebbe uno. Una voce di navigazione che porta a un
  elenco di un elemento è rumore, e la scheda dell'immobile resta raggiungibile dal
  contratto — che è anche il modo in cui ci si arriva pensando;
- **il contratto è al singolare**, perché per chi abita è uno;
- **le bollette restano**, ed è la voce su cui si esita: sono lette e non agite. Ma sono
  la ragione di metà dei suoi addebiti, e un riaddebito che non si può risalire a un
  documento è una cifra da accettare sulla fiducia.

**Atterra su `/addebiti`, e non su una dashboard sua.** È la domanda che si fa davvero —
*quanto devo* — ed è una rotta che esiste già, ristretta dalle policy ai suoi. Una
dashboard dell'inquilino sarebbe una proiezione nuova, quindi un endpoint nuovo, quindi
backend dentro una fetta di client; e soprattutto è una domanda di prodotto che nessuno
ha ancora posto: *quali* numeri. È rinviata con la sua ragione, e la nota è già nel tipo
`GetOwnerDashboard`.

### L'assegnazione è «prendi in carico», e non è una semplificazione

`AssignTicketCommand` vuole un `UserId`, e sembra chiedere un endpoint che elenchi le
persone dell'organizzazione. **Non lo è, perché quel menù non avrebbe nessuno dentro.**
Verificato: l'unico modo di diventare staff è `RegisterOwner`, che crea
l'organizzazione personale con ruolo `Owner`; `InviteTenant` scrive
`role: OrgRole::Tenant` **fisso**, e non esiste altra via per creare una membership.
`OrgRole::Manager` è nell'enum e nessuno lo scrive mai. Un'organizzazione ha quindi
esattamente uno staff, il suo owner.

L'azione è dunque «prendi in carico», che assegna a chi chiama, ed è la verità completa
del modello e non un ripiego. Il giorno in cui esisterà un invito che crea un gestore,
quell'azione diventa un menù — e sarà quella fetta a doverne discutere.

**Alternativa scartata: `GET /api/v1/members` dentro questa fetta.** Non è una rotta in
più: è decidere **quali colonne** un membro possa vedere di un'altra persona, cioè la
stessa domanda di [ADR-036](#adr-036), presa senza un caso d'uso che la guidi.
Costruirla ora significa costruire quella sbagliata.

### Il corollario: si instrada sui codici, non sul ruolo

Riapertura e annullamento di una segnalazione **non** sono staff-only, e il client non
può derivarli dal ruolo. A un coinquilino che provi a riaprire l'API risponde
`403 ticket.not_yours` e non `org.staff_role_required`: non gli manca un ruolo — nessun
ruolo gli darebbe quell'operazione — gli manca l'essere stato lui a segnalare, e i due
rimedi sono diversi. Per l'annullamento vale il contrario in un caso preciso: chi ha
segnalato può annullare **finché nessuno ci ha messo mano**, e da lì in poi
`org.staff_role_required` è il codice vero, perché annullare una segnalazione in
lavorazione butterebbe via il lavoro di qualcun altro.

Il client mostra quindi quelle due azioni a chi ha segnalato, e traduce i due codici in
due messaggi diversi.

---

<a id="adr-043"></a>

## ADR-043 — Il nome di un membro si scrive all'ingresso, perché dopo non esiste più

**7 settembre 2026.** Nasce leggendo `crates/api/src/routes/expenses.rs` per pianificare
le spese co-living nel client, e riguarda un modulo che era già completo: non cambia
l'API, cambia **come un client deve usarla**.

### Il fatto: nessuna risposta di quel modulo porta un nome

`GroupBalanceResponse`, `ExpenseShareResponse`, `SettlementResponse` e
`ExpenseResponse.paid_by_user_id` identificano le persone con il solo `user_id`. L'unico
nome che esista in tutto il modulo è `display_name` su `ExpenseGroupMemberResponse`, ed è
**facoltativo**: lo scrive chi chiama `POST /expense-groups/{id}/members`, oppure non
esiste.

Una schermata di saldi è fatta di nomi e numeri e di nient'altro. Senza `display_name`
mostra UUID, che è la condizione da cui [ADR-041](#adr-041) ha appena liberato il
selettore delle organizzazioni.

### Perché non basta risolverli quando servono

La via che viene in mente per prima è incrociare gli `user_id` con
`GET /properties/{id}/residents`, che dà `user_id` e `full_name`. Funziona, e **non nel
caso che conta**: quell'elenco copre chi abita **ora**, e la persona uscita di casa che
sta ancora saldando il suo conto non ci compare più — cioè precisamente il caso per cui
la migration `0027` tiene il suo debito nei saldi del gruppo. In più `property_id` sul
gruppo è facoltativo: un gruppo creato senza immobile non ha nemmeno un elenco da
interrogare, e nessun'altra rotta proietta un utente per identificatore, né deve
esistere ([ADR-036](#adr-036)).

Il difetto che ne verrebbe è il peggiore fra quelli possibili qui: **si romperebbe solo
sui gruppi vecchi con qualcuno uscito**, cioè tardi, su pochi dati, e per chi ha meno
modo di spiegare che cosa vede.

### La decisione

**Ogni volta che il client iscrive qualcuno a un gruppo manda `display_name`**, preso dal
`full_name` del residente scelto; e alla creazione del gruppo manda il proprio. Senza
eccezioni, perché è l'unica occasione: la riga della membership non sparisce mai —
`left_at` valorizzato, riga conservata — e quel nome resta leggibile anche quando la
persona non abita più lì.

Il ripiego, per i gruppi creati altrimenti — da `curl`, o da una versione futura che
dimentichi questa regola — è in `apps/web/src/features/expenses/names.ts`, in
quest'ordine: il nome dichiarato sulla membership, poi il residente se l'identificatore è
ancora fra quelli, poi un'etichetta neutra dal catalogo. **Mai l'UUID**: un
identificatore in faccia a chi legge non è informazione, è una resa.

### Conseguenze

- il nome è **immutabile in pratica**: il client non offre di cambiarlo, perché
  riscriverlo richiederebbe di ripassare dalla rotta che iscrive, e su un membro attivo
  quella risponde `409 expense_group.membership_exists`. È scritto nel form, dove si
  decide;
- **l'API resta com'è.** `display_name` non diventa obbligatorio: renderlo tale
  spezzerebbe i gruppi esistenti e toglierebbe all'API un default legittimo. La regola
  vive nel client, che è dove il nome è disponibile;
- chi aggiungerà un secondo punto d'ingresso ai gruppi — un'app, un import, una fetta
  futura — deve rispettare la stessa regola, e questa voce esiste perché la trovi.

*Alternativa scartata: un endpoint che proietti un utente per identificatore.* È la
domanda di [ADR-036](#adr-036) — quali colonne un membro possa vedere di un'altra
persona — riaperta per un problema di presentazione. Il perimetro è la casa, e resta.

### La navigazione dell'inquilino passa a cinque voci

Corollario di [ADR-042](#adr-042), che vale come emendamento alla sua sezione «Che cosa
vede chi abita»: alle quattro voci si aggiunge ***Spese***, e resta **sua soltanto**.
L'API la concederebbe anche allo staff — quasi tutto quel modulo è `ensure_member`, e il
proprietario è staff dell'organizzazione — ma le spese co-living sono il denaro che i
coinquilini si devono **fra loro**, e il proprietario ne è estraneo per costruzione. Una
voce che per lui è sempre vuota è il rumore che ADR-042 ha appena tolto dalla sua
navigazione con *Immobili*.

**Le schermate restano progettate e raggiungibili per URL**, come `PaymentsPage`: un
`owner` che abiti in una delle proprie case è in un gruppo come chiunque altro, e per lui
`/spese` risponde.

<a id="adr-044"></a>

## ADR-044 — Il rinnovo fra schede si serializza, e la sessione non si trasmette

**8 settembre 2026.** Passo `C2` della fetta C. Chiude un difetto documentato da tre
fette e **osservato** nella B-bis, e la decisione che prende non è «coordinare o no» —
quello era pacifico — ma **che cosa** le schede si scambino.

### Il fatto

Il refresh a singolo volo è una promessa in memoria, e due schede sono due contesti
JavaScript. Se l'access token di entrambe scade nello stesso istante, la seconda presenta
un refresh token **già ruotato**; l'API tratta il riuso come prova che qualcuno ne ha una
copia e revoca l'intera catena. Nella fetta B-bis è accaduto senza cercarlo, con due
schede aperte sulla stessa origine: `riuso di un refresh token già ruotato: revocate
tutte le sessioni` nei log, e la sessione fuori senza che nessuno avesse fatto niente.

«Raro» era vero solo finché si teneva una scheda sola. Con due account e due finestre —
cioè il collaudo che la fetta C prepara — non è un caso limite: è certo.

### La via che viene in mente per prima, e perché è sbagliata

**Trasmettere la sessione**: la scheda che vince rinnova e manda l'access token alle
altre con `BroadcastChannel`. Funziona, ed è il motivo per cui va scritta qui invece che
scartata in silenzio — chi la propone non sta sbagliando ragionamento, sta ignorando un
vincolo.

Il vincolo è che **l'access token vive in memoria apposta**: non è leggibile da uno
script che arrivi dopo il caricamento della pagina, e muore al reload. Metterlo su un
canale — o, peggio, in `localStorage` come ripiego — lo espone esattamente a ciò da cui
era stato tenuto fuori. Si comprerebbe un rinnovo risparmiato vendendo la sola proprietà
per cui quel token non sta in `localStorage`.

### La decisione: una coda, non una trasmissione

**Non serve condividerlo.** Ciò che le schede condividono è **il cookie**, e l'API ruota
a ogni uso: il riuso lo rileva soltanto su *due usi dello stesso token*. Se la seconda
scheda **aspetta** che la prima abbia finito e poi rinnova a sua volta, presenta il
cookie **nuovo** — che è una rotazione legittima, non un riuso.

Serve quindi un mutex per origine: `navigator.locks.request`, che è esattamente questo,
con ripiego su un lock in `localStorage` dove i Web Locks non ci sono. Il ripiego scrive
**un marcatore con una scadenza e mai il token**: un identificatore casuale e un istante,
cioè niente che serva a chi lo legga.

Costa `N` rinnovi per `N` schede invece di uno. È il prezzo giusto: succede una volta
ogni quindici minuti per scheda, e compra il fatto che il token non esce dalla memoria.

### Conseguenze

- **i due strati non sono ridondanti.** Il singolo volo risparmia i rinnovi *dentro* una
  scheda, il lock impedisce che due schede li facciano *insieme*. Il lock serializza
  anche in-scheda, quindi maschera il difetto che il singolo volo evita: i test che
  riproducono quel difetto disattivano **entrambi**, ed è scritto lì perché un verde che
  non prova nulla è peggio di un test assente;
- **il marcatore deve scadere.** Una scheda chiusa a metà rinnovo non può bloccare le
  altre per sempre: 15 secondi di vita, e dopo 20 di attesa si procede comunque senza il
  lock. Un rinnovo bloccato per sempre è peggio del difetto che il lock evita — là si
  perde la sessione e si rientra, qui l'applicazione resterebbe ferma senza dire perché;
- **`localStorage` non ha un confronta-e-scambia**, quindi il ripiego rilegge dopo aver
  scritto e riprova se non si ritrova. Resta una finestra teorica in cui due schede
  scrivono davvero insieme, e in quel caso si torna al comportamento di prima: il
  difetto, non una regressione. È il prezzo di non avere `navigator.locks`, che nei
  browser di questo decennio c'è.

*Alternativa scartata: un `SharedWorker` che tenga la sessione.* È il disegno corretto
per un'applicazione che ne avesse bisogno, e qui sarebbe un terzo contesto da far vivere,
morire e ricollegare per risolvere un problema che un lock chiude in venti righe.

<a id="adr-045"></a>

## ADR-045 — Accettare un invito sceglie l'organizzazione, ed è l'unico momento in cui si sa quale

**8 settembre 2026.** Passo `C1` della fetta C. Chiude **a metà** — deliberatamente — un
debito registrato da due fette, e la metà che chiude è quella che conta.

### Il fatto

`pick_default_membership` preferisce il ruolo più alto — owner, poi manager, poi tenant —
e la registrazione crea **sempre** un'organizzazione personale di cui si è owner. Un
inquilino che si registra da sé atterra quindi sempre nella propria organizzazione vuota,
e deve cambiare a mano dal selettore. Lo si è visto nel collaudo della B-bis: la prima
schermata di un inquilino appena registrato è «Comincia registrando il tuo primo
immobile», che è il contrario di ciò che quella persona è venuta a fare.

### La decisione

**Accettare un invito è il momento in cui l'ambiguità non c'è**: quella persona ha appena
dichiarato in quale organizzazione vuole entrare. La risposta di
`POST /auth/accept-invitation` porta `organization_id` e `role`, e il client chiama
`selectOrganization` con quelli, poi manda alla pagina iniziale **del ruolo** — che per
un inquilino è `/addebiti` e non la radice.

*Alternativa scartata: cambiare `pick_default_membership` nel backend.* Vorrebbe dire
decidere **per tutti** quale appartenenza vinca, e il caso legittimo esiste: chi affitta
una casa e ne abita un'altra è owner della propria e tenant di un'altra, e nessuna delle
due è ovvia. La scelta del client è locale, è reversibile dal selettore, e non richiede
una migration.

### Il debito non si chiude del tutto, e va detto

Chi entra da un browser nuovo, mesi dopo, riparte dal default di sempre: il ricordo vive
in `localStorage` e quel browser non ce l'ha. **Si chiude la prima volta, non ogni
volta** — ed è la prima volta a far dire «non funziona».

### Il codice non viaggia in un URL

Corollario, e riguarda la schermata che riceve il codice. `accept_token` è **una
credenziale**: chi ce l'ha entra in un'organizzazione. Un link `/invito/<token>`
comprerebbe un clic e scriverebbe quella credenziale in tre archivi che nessuno ripulisce
— la cronologia del browser, il `Referer` verso qualunque risorsa esterna, e i log di
ogni proxy davanti al client. **È un campo in cui incollare**, coerente con la modale che
lo consegna, che lo chiama «codice» e dice che vale sette giorni.

Ne segue una conseguenza progettata invece che subita: accettare richiede una sessione,
perché la rotta è autenticata. Chi arriva su `/invito` senza account **non riceve un
`401`**: trova scritto che deve prima registrarsi, con il link, e che poi torna lì — e ci
torna davvero, perché lo `state.from` della redirezione al login, che fino a questa fetta
nessuno leggeva, adesso decide dove si atterra dopo essere entrati.

### La chiamata va senza `X-Org-Id`, e non si «uniforma»

È l'operazione che **crea** l'appartenenza: dichiarare un'organizzazione di cui non si è
ancora membri fa rispondere `403` a una richiesta legittima. `acceptInvitation` lo fa già
con `withoutOrganization`, ed è verificato da un test che guarda **l'header vero** e non
il codice che lo scrive — perché la prossima persona che passa di lì vedrà un'eccezione
alla regola «`X-Org-Id` sempre» e sarà tentata di toglierla.

<a id="adr-046"></a>

## ADR-046 — L'attivazione guarda il senso delle quote, non solo la somma

**9 settembre 2026.** Passo `U4` della sezione 3. È la regola che impedisce di
riprodurre il danno che la stessa sezione consegna gli strumenti per riparare.

### Il fatto

In database sono esistiti contratti **attivi** con due parti così:

| parte | quota | intestatario | ha accettato |
|---|---|---|---|
| `inq2` | **100,00** | sì | **no** |
| `inq3` | **0,00** | no | sì |

Cioè: l'inquilino entrato davvero non doveva niente, e il 100% era di qualcuno che non
aveva mai accettato l'invito. Il contratto era `active`, quindi aveva superato ogni
controllo — perché il controllo che c'era è `100 + 0 = 100`, e quella somma è giusta.

Il difetto nasceva dal concorso di quattro cose, e nessuna delle quattro è sbagliata da
sola: il form propone `100` e *intestatario principale* alla prima parte; ogni invito
successivo riceve `remaining_share`, che a quel punto è **zero**, e nessuno lo dice;
l'attivazione verifica la somma; e niente di tutto questo si correggeva
([`R17`](12-da-fare.md#lelenco), chiuso da questa sezione).

### Decisione

**Un inquilino con quota zero impedisce l'attivazione**, accanto all'invariante che
esisteva già sulla somma. Una quota a zero non è un caso d'uso che sappiamo nominare: un
inquilino che non deve niente non è un inquilino, è una riga rimasta indietro.

**Aggiungere una parte è ammesso solo finché il contratto è in bozza o in attesa di
firma.** `AddLeaseParty` non guardava lo stato — controllava il ruolo, caricava il
contratto e inseriva — quindi si poteva aggiungere una parte a un contratto **attivo**, e
da quel momento le quote non sommavano più a 100 senza che niente protestasse:
l'invariante è verificata *al momento* dell'attivazione, e nessuno la riverificava dopo.
Il client offriva già il bottone solo in bozza, ma quella è una scelta di che cosa
offrire, non un confine — la stessa distinzione di [ADR-042](#adr-042).

### Che cosa questa regola **non** garantisce, e va detto

**Non pretende che l'intestatario principale abbia accettato l'invito.** Un contratto si
firma sulla carta prima che l'inquilino apra un account, e pretenderlo bloccherebbe un
giro legittimo: si attiva, poi si invita. È la seconda faccia del difetto osservato — il
100% era di chi non aveva accettato — e resta possibile. La si accetta perché il rimedio
costerebbe più del danno: impedirebbe di attivare un contratto vero in attesa che una
persona si registri.

**Non sana i contratti già rotti.** Le invarianti valgono sulle transizioni future: un
contratto già attivo con una quota a zero resta com'è, e si corregge a mano con
`PATCH /lease-parties/{id}`, che questa sezione introduce. Non esiste una migration di
bonifica, e non è una dimenticanza: quale sia la quota giusta lo sa solo chi amministra.

**Non impedisce che le quote smettano di sommare a 100 dopo l'attivazione.** La
correzione di una parte non ricontrolla la somma, ed è deliberato: per portare due parti
da `100 / 0` a `50 / 50` bisogna passare da uno stato intermedio in cui la somma non fa
100. Su un contratto attivo la responsabilità è di chi corregge.

### Alternative scartate

**Ricontrollare la somma a ogni correzione.** Renderebbe impossibile il rimedio, per la
ragione qui sopra: non si possono cambiare due righe in una richiesta.

**Ammettere l'aggiunta su un contratto attivo ricontrollando la somma dopo.** Non si può
fare in una richiesta sola — per aggiungere un terzo inquilino bisogna prima abbassare le
quote degli altri due — quindi o si rifiuta, o si inventa una richiesta che porta tutte le
parti insieme. Quella è il **subentro**, che è un'operazione a sé e non è progettata: sta
in [12](12-da-fare.md) come voce nuova.

**Un valore minimo di quota diverso da zero** (per esempio l'1%). Sarebbe una soglia
arbitraria travestita da regola: zero è l'unico valore che significa qualcosa — «questa
persona non paga niente» — e gli altri sono decisioni di chi amministra.

---

<a id="adr-047"></a>

## ADR-047 — Una segnalazione può essere di interesse comune, e il ramo sta nella funzione

**9 settembre 2026.** Nasce da [`R19`](12-da-fare.md#lelenco), pianificata come sezione 5
delle segnalazioni.

### Il fatto, che non è un difetto

Una segnalazione aperta sull'immobile intero — la caldaia condominiale, la serratura del
portone — **non la vede nessun altro inquilino**. Non è una svista: è ciò che
`app.can_access_ticket` dice dalla `0014`. Concede l'accesso allo staff, a chi ha
segnalato, a chi è assegnato, e poi a `t.lease_id IS NOT NULL AND can_access_lease(...)`
— quindi il quarto ramo passa dal **contratto**, e una segnalazione senza contratto resta
privata.

Quel default è giusto e **resta**: un guasto dentro casa propria non riguarda i vicini di
pianerottolo, e la maggior parte delle segnalazioni è di quel tipo.

### La decisione

Una colonna `app.tickets.visible_to_residents BOOLEAN NOT NULL DEFAULT false`, una
casella nel form di apertura, e **un quinto ramo in `app.can_access_ticket`**:

```sql
OR (t.visible_to_residents
    AND EXISTS (SELECT 1 FROM app.property_residents(t.property_id) pr
                 WHERE pr.user_id = app.current_user_id()))
```

Migration `0035_a_ticket_can_be_shared_with_residents.sql`. La visibilità è un **confine
del database** e non un filtro applicativo: metterla nel service layer lascerebbe la riga
leggibile a una query scritta a mano, che è la constatazione della `0029` e della `0030`.

### Perché nella funzione e non in `tickets_select`

È la parte che si sbaglia, ed è la ragione principale per cui questa voce esiste.
`app.can_access_ticket` non serve una policy sola: la chiamano `tickets_select`,
`ticket_messages_select`, `ticket_status_history_select` e
`can_access_attachment('ticket', …)`. Allargare la sola policy di riga darebbe a un
coinquilino una segnalazione **che si apre e non si legge** — senza thread, senza
storico, senza le fotografie del guasto, e senza un errore che spieghi perché.

La funzione si sostituisce con `CREATE OR REPLACE FUNCTION` e la stessa firma: le policy
la referenziano per oid, quindi non vanno toccate né ricreate.

### Che cosa la condivisione dà, e che cosa non dà

**Dà anche il thread e le fotografie**, ed è voluto: una visibilità che permetta di
leggere «ho chiamato il tecnico» ma non di rispondere «da me non funziona ancora»
sarebbe una bacheca, non una segnalazione condivisa. `ticket_messages_insert` passa dalla
stessa funzione, quindi un residente può scrivere nel thread.

**Non dà nessuna scrittura sulla segnalazione.** `tickets_update` ha un predicato **in
linea** — staff, `reported_by`, `assigned_to` — e non passa da `can_access_ticket`: un
residente non riapre, non annulla, non corregge, non chiude. È la stessa asimmetria che
il modulo ha già per il coinquilino sullo stesso contratto, e la ragione per cui questa
migration è più piccola di quanto sembri.

**Le note interne restano invisibili**: `ticket_messages_select` ha un `AND` a parte su
`is_internal`, che questo ramo non tocca.

### Il predicato è `app.property_residents`, e la scelta non è di gusto

La `0030` ha introdotto due predicati sul perimetro della casa, e vanno scelti e non
riscritti — una terza definizione di «chi abita qui» è la trappola che quel file
documenta alla sua terza occorrenza.

`app.shares_home_with(p_user_id)` è **utente↔utente**: userebbe `reported_by` come
termine di confronto, e ne verrebbero due difetti. `reported_by` è
`ON DELETE SET NULL`, quindi la condivisione sparirebbe da sé il giorno in cui l'autore
viene cancellato; e quella funzione **non filtra `app.lease_in_force`** — è deliberato,
il suo commento lo dice, perché è un *pavimento* per la leggibilità dei nomi e non un
elenco di chi ci abita adesso. Un inquilino uscito due anni fa vedrebbe le segnalazioni
nuove della casa in cui non vive più.

`app.property_residents(p_property_id)` è **ancorata all'immobile**, che è la dimensione
giusta perché `property_id` è obbligatorio su ogni segnalazione. Filtra `lease_in_force`,
filtra `left_at IS NULL`, esclude i garanti, e ha già `can_access_property` come confine
nel corpo.

### Conseguenze

- **il booleano va scritto per primo**, e il ramo per ultimo nell'`OR`.
  `can_access_ticket` è `COST 500` e viene valutata **per riga** nell'elenco: con il
  booleano davanti, la funzione che restituisce righe non viene chiamata affatto sulle
  segnalazioni private, che sono il default;
- **la funzione ha il confine dentro**, quindi sul pool privilegiato — che non ha
  contesto — risponde «no». È la lezione che la sezione 4 ha lasciato scritta in un test
  su `v_property_occupancy`: chi verifica questo ramo deve installare
  `app.current_org_id` **e** `app.current_user_id`;
- la colonna entra nella `GRANT UPDATE` di `propflow_app` — si può cambiare idea dopo
  l'apertura, vedi [ADR-048](#adr-048) — e **non** entra fra le colonne strutturali del
  controllo `colonna-strutturale-scrivibile`;
- il nome della colonna dice **residenti** e non «tutti gli inquilini»: quest'ultima
  formula si legge come «dell'organizzazione», che sarebbe un confine più largo e
  sbagliato — un inquilino di un'altra casa dello stesso proprietario non c'entra. La
  casella nel form usa invece la parola comune, perché lì il confine è già chiaro dal
  contesto.

### Alternative scartate

**Un `ALTER POLICY tickets_select` con il ramo in più.** È la via che il backlog
indicava, e produce la segnalazione che si apre e non si legge.

**Una terza funzione, `app.lives_in_property(uuid)`.** Sarebbe più leggibile di un
`EXISTS` su una funzione tabellare, e sarebbe la seconda definizione di «chi abita qui»
che la `0030` esiste per impedire.

**Un filtro applicativo.** Lascia la riga leggibile a chi non passa dal service layer, e
questo progetto ha già una migration — la `0029` — dedicata a smettere di fidarsi di
regole applicative su confini che il database lascia aperti.

---

<a id="adr-048"></a>

## ADR-048 — Il testo di una segnalazione si corregge, e questo emenda ADR-026

**9 settembre 2026.** Nasce da [`R19-bis`](12-da-fare.md#lelenco). **Revoca in parte
[ADR-026](#adr-026)**, e va letta con quella davanti.

### Il fatto: oggi non è una scelta applicativa, è un privilegio

Chi apre una segnalazione con un refuso nel titolo non può correggerlo, e non perché
manchi una rotta: `title` e `description` **non sono scrivibili dal ruolo applicativo**.
La `0020` gliele ha tolte, e a difendere quella scelta ci sono tre cose oltre alla
migration:

| Dove | Che cosa |
|---|---|
| `ops/sql/checks/schema_invariants.sql` | il controllo `colonna-strutturale-scrivibile` fallisce se `propflow_app` acquista l'`UPDATE` su quelle colonne |
| `crates/infrastructure/tests/persistence.rs` | `the_application_role_cannot_move_a_ticket_to_another_property` asserisce il `permission denied` |
| `docs/05-api.md`, i commenti del dominio, il catalogo del client | la frase «di una segnalazione si modificano solo i costi» |

Ne segue che questa non è una capacità da aggiungere: è una decisione da **revocare**, e
il posto per farlo è qui, prima del codice.

### Che cosa cambia, e che cosa no

**Il criterio di ADR-026 non cambia.** Diceva che il privilegio di colonna serve *«quando
una colonna non ha un motivo legittimo di cambiare»*. Quello che cambia è il fatto: ora
ce l'ha. La `0020` teneva `title` e `description` fuori con l'argomento «una segnalazione
sbagliata si annulla e si riapre», che vale per l'**immobile** — dove riaprire è l'unico
modo di rimettere insieme una riga coerente — e non per un refuso, dove costa a chi
segnala di rifare tutto e di perdere il riferimento `TK-000042` che ha già comunicato.

**Restano fuori** `property_id`, `room_id`, `lease_id`, `reported_by`,
`reported_by_kind`, `reference` e `currency`, e per loro la frase della `0020` resta vera
alla lettera. La causa radice che ADR-026 descrive — lo schema non ha **alcun vincolo**
fra `room_id`, `lease_id` e `property_id` — non è stata rimossa, e il privilegio resta la
sola difesa che copre quella porta.

### La decisione

Migration `0036_a_report_can_be_corrected.sql`:
`GRANT UPDATE (title, description) ON app.tickets TO propflow_app`, insieme a
`visible_to_residents` che la [`0035`](#adr-047) ha appena introdotto.

E una rotta a sé: **`PATCH /api/v1/tickets/{id}/report`**, con `title`, `description` e
`visible_to_residents` facoltativi e la semantica generale del `PATCH` (assente =
invariato; nessuno dei tre è azzerabile). L'autorizzazione è **una sola per l'intera
rotta**: lo staff sempre, chi ha segnalato finché la segnalazione è `open`.

### Perché una rotta a sé e non `PATCH /tickets/{id}`

Allargare il `PATCH` dei costi darebbe una rotta con **due autorizzazioni diverse a
seconda dei campi presenti** — i costi allo staff, il testo a chi ha segnalato. Il modo
in cui si sbaglia è concreto: con la semantica del `PATCH` i campi sono *assenti*, quindi
l'autorizzazione dipenderebbe da quali chiavi ci sono nel JSON, e un campo aggiunto un
anno dopo senza toccare quel ramo sarebbe scrivibile da chi non deve. È un difetto muto,
che non compare in nessun test finché qualcuno non manda quella combinazione.

`report` è un **sostantivo** e non un verbo perché non è una transizione: le sotto-risorse
di questo modulo — `/triage`, `/assign`, `/close` — sono `POST` e hanno effetti propri,
mentre questa è la correzione di un attributo e resta `PATCH`. I tre campi stanno insieme
perché sono tutti e tre **ciò che chi ha segnalato ha dichiarato**, e hanno per questo la
stessa autorizzazione.

### Chi corregge, e fino a quando

**Lo staff sempre, chi ha segnalato finché è `open`.** È la forma che l'annullamento ha
già — «finché nessuno ci ha messo mano» — e la regola vive in **una sola funzione**
condivisa con `cancel`: due copie della stessa condizione sono due regole che
divergeranno alla prima modifica di una delle due.

I codici di rifiuto restano quelli in catalogo, con la distinzione che già esiste:
`ticket.not_yours` a chi non ha segnalato — non gli manca un ruolo, gli manca l'aver
segnalato — e `org.staff_role_required` a chi ha segnalato ma la segnalazione è già in
lavorazione, dove il ruolo serve davvero.

Il dominio rifiuta su `cancelled` con `ticket.cancelled`, come fa già `apply_costs`. Su
una **chiusa** si corregge, e serve: la si rilegge mesi dopo, e un titolo sbagliato lì è
sbagliato per sempre.

### Che cosa il privilegio smette di garantire

Va detto per intero, perché è il pezzo che si perde revocando. Dal momento del `GRANT`,
un inquilino che sia `reported_by` può riscrivere titolo e descrizione **da qualunque
stato** con una `UPDATE` scritta a mano sotto RLS: il limite «finché è `open`» è
applicativo, e una policy non può esprimerlo perché **non può confrontare la riga nuova
con quella vecchia** — la constatazione di ADR-021 e ADR-026, qui a proprio sfavore.

Resta vero ciò che la `0020` aveva **misurato** e non dedotto: il confine di isolamento
tiene — dopo la scrittura quell'inquilino non vede una riga in più — e ogni `UPDATE`
finisce in `audit_log`. È esattamente il compromesso che già vale per `status`,
`assigned_to` e `actual_cost`, che sono concesse e la cui disciplina è applicativa,
transizione per transizione.

**Non si prova a stringere il confine nella `USING` di `tickets_update`.** Restringerla a
`status = 'open'` per chi ha segnalato romperebbe la **riapertura**, che parte proprio da
`closed`.

### Conseguenze

- il controllo `colonna-strutturale-scrivibile` perde due righe, e il commento che lo
  accompagna cambia con esse: continua a dire perché le altre sei ci sono;
- il test dei privilegi sposta due `UPDATE` dal blocco dei rifiuti a quello delle
  concessioni. **L'emendamento del controllo e del test è il documento della revoca**, e
  va fatto nello stesso passo della migration — non è un fastidio da togliere di mezzo
  per far passare `just ci`;
- `docs/05-api.md` cambia in due punti: la tabella delle rotte e la sezione «Perché le
  transizioni sono sotto-risorse e il `PATCH` è solo per i costi», che resta vera per
  categoria, priorità, immobile, stanza e contratto;
- **categoria e priorità non diventano correggibili**: sono il lavoro del triage, e
  lasciarle modificabili altrove renderebbe `triaged` una dichiarazione non verificabile.

### Alternative scartate

**`PUT /tickets/{id}/report` con i tre campi obbligatori.** Toglie il problema dei campi
assenti alla radice, e obbliga chi cambia solo la casella di visibilità a rimandare
indietro un testo che non ha toccato, cioè a sovrascriverlo con una copia possibilmente
vecchia. Il progetto non usa `PUT` da nessuna parte e non è questo il caso per
introdurlo.

**Nessuna rotta: la correzione passa dall'annullamento**, che è ciò che la `0020`
prescrive oggi. Su un refuso costa il riferimento già comunicato e tutto il testo.

**Solo chi ha segnalato, senza lo staff.** Lascerebbe un refuso incorreggibile appena la
segnalazione passa ad `assigned`, e romperebbe la simmetria con l'annullamento senza
comprare niente: ogni scrittura è comunque in `audit_log`.

---

<a id="adr-049"></a>

## ADR-049 — I nomi delle persone viaggiano nella risposta, ed è il primo lettore di `users_select`

**9 settembre 2026.** Nasce da [`R21`](12-da-fare.md#lelenco) e
[`R22`](12-da-fare.md#lelenco). Applica il criterio di [ADR-043](#adr-043) a un modulo in
cui la fonte del nome è diversa, e **chiude una finestra** che la
[`0030`](#adr-036) aveva lasciato aperta per iscritto.

### Il fatto

`TicketResponse` porta `assigned_to` come UUID e `assigned_at`, e nessun nome: la scheda
mostra **la data al posto della persona**. `TicketMessageResponse` porta `author_id` e
`author_kind`, quindi il thread dice il **ruolo** — *Inquilino*, *Proprietario* — e non
chi.

Il ruolo è già a schermo e non va rifatto. Il caso che resta scoperto è il **co-living**:
due coinquilini nel thread compaiono tutti e due come *Inquilino*, e «me lo dice il
proprietario o un coinquilino?» ha già risposta mentre «quale coinquilino?» non ne ha
nessuna.

### La decisione

Un **modello di lettura**, con la forma che la sezione 3 ha già stabilito per i nomi degli
immobili: una porta, **una query per pagina**, la composizione nello use case, campi
opzionali nel DTO. `TicketResponse` guadagna `reported_by_name` e `assigned_to_name`;
`TicketMessageResponse` guadagna `author_name`.

La fonte è `app.users.full_name`, letta **sotto Row Level Security**. Il precedente è
`list_organizations_for_user` ([ADR-041](#adr-041)), che si affida alla policy invece di
ripetere il confine nel `WHERE`: la RLS restringe il join, e ciò che il chiamante non
può vedere semplicemente non torna.

**Quando il nome non torna, il campo è assente e il client mette un'etichetta neutra dal
catalogo. Mai l'UUID**: è la regola di [ADR-043](#adr-043), ed è la stessa forma di
«Immobile archiviato» che la sezione 3 usa quando un `LEFT JOIN` non trova il nome.

### Perché non una rotta che risolva le persone

Perché è già stata scartata due volte. [ADR-036](#adr-036) ha deciso che il perimetro è
la casa e che nessuna rotta proietta un utente per identificatore; [ADR-043](#adr-043)
l'ha riconfermato, e con parole che valgono identiche qui: *«è la domanda di ADR-036 —
quali colonne un membro possa vedere di un'altra persona — riaperta per un problema di
presentazione»*.

I nomi viaggiano quindi **dentro** la risposta della segnalazione, che è la via che la
sezione 3 ha percorso per gli immobili, e non da una rotta che risolva le persone.

### Perché non si denormalizza il nome sulla riga

Lo schema denormalizza i nomi **dove si mostrano** — `lease_parties.full_name`,
`expense_group_members.display_name` — e sarebbe stata la terza occorrenza. Si scarta per
due ragioni. Costerebbe due colonne, una migration e due `GRANT` per un dato che la
policy già concede. E soprattutto **congelerebbe il nome al momento della scrittura**: su
una membership è la proprietà che serve — [ADR-043](#adr-043) esiste perché quel nome
sopravviva a chi se ne va — mentre su una segnalazione aperta ieri il nome giusto è
quello di adesso.

### La finestra che si chiude

La `0030` aveva ristretto `users_select` dall'organizzazione alla casa, e nel farlo aveva
scritto una cosa che questa voce viene a incassare:

> Il verde della suite non è la prova che sia giusto. Restringendo, **nessun test
> fallisce**. […] prova che la capacità per cui `shares_org_with` esiste — «mostrare i
> nomi delle controparti» — **non è mai stata costruita**. La finestra per restringere è
> aperta e la chiude il primo client che legga un nome da lì.

**Questa sezione è quel primo client.** Da qui in avanti `users_select` è portante:
restringerla non è più gratuito, e i suoi quattro rami vanno letti come contratto e non
come pavimento. In particolare:

| Ramo | Che cosa tiene in piedi qui |
|---|---|
| `shares_home_with(id)` | un coinquilino legge il nome dell'altro nel thread — **il caso di `R22`** |
| `is_my_staff(id)` | l'inquilino legge il nome di chi ha in carico la sua segnalazione — **il caso di `R21`** |
| `is_staff_of_user(id)` | il proprietario legge i nomi dei propri inquilini |
| `id = current_user_id()` | il proprio nome |

`shares_home_with` **non filtra `lease_in_force`**, ed è quello che rende leggibile il
nome dell'autore di un messaggio su una segnalazione vecchia di chi si è nel frattempo
trasferito. Era già scritto nel suo commento come una scelta; qui diventa una dipendenza.

### Conseguenze

- **`reported_by_name` entra pur non essendo nella lettera di `R21` né di `R22`**: è lo
  stesso identificatore, risolto dalla stessa query, mostrato nella stessa scheda tre
  righe più su di «In carico a». Lasciarlo fuori consegnerebbe una schermata in cui un
  nome compare e l'altro no;
- il ruolo resta **accanto** al nome e non al suo posto — *Proprietario · Mario Rossi* —
  perché dice una cosa che il nome non dice: da che parte del rapporto sta chi scrive;
- una **query per pagina** e non una per riga: l'elenco ne mostra venticinque, e
  risolverli uno alla volta sarebbe il problema `N+1` scritto a mano. È la stessa nota
  che porta `LeaseRepository::labels`;
- la [sezione 6](12-da-fare.md#lordine-di-esecuzione) — `R7`, il proprio account —
  eredita questo modo di risolvere un nome, ed è la ragione per cui l'ordine di
  esecuzione mette la 5 prima della 6.

---

<a id="adr-050"></a>

## ADR-050 — Un thread chiuso non riceve messaggi, e riaprire dice perché

**9 settembre 2026.** Nasce da [`R23`](12-da-fare.md#lelenco) e
[`R24`](12-da-fare.md#lelenco), che sono **una decisione sola**: la seconda esiste perché
la prima chiude una porta.

### Il fatto

`PostTicketMessage` fa `ensure_member` e **non guarda lo stato**. Nessun invariante lo
vieta, quindi il thread di una segnalazione chiusa o annullata accetta scritture: una
conversazione continua sotto uno stato che dice «finita», e nessuno la legge più perché
la segnalazione è fuori da ogni coda.

E `POST /tickets/{id}/reopen` **non accetta nulla**: né il client né l'handler hanno un
corpo. Una segnalazione torna viva e non c'è niente, da nessuna parte, che dica perché.

### La decisione, parte prima: si scrive nei sei stati di lavorazione

Un invariante di dominio legato a `TicketStatus::is_working()`. È lo stesso predicato che
`close` e `cancel` usano già come guardia, e riusarlo invece di enumerare «non `closed` e
non `cancelled`» copre da sé il terzo caso: `resolved`, che l'applicazione non scrive mai
ma che una riga scritta a mano può contenere ([ADR-019](#adr-019)).

**È una regola di dominio e non un controllo di interfaccia**: nasconderla nel client
lascerebbe la rotta aperta. **Leggere resta permesso**, e un thread chiuso si rilegge per
intero: ciò che finisce è la scrittura.

### La decisione, parte seconda: il motivo della riapertura è obbligatorio

Con la parte prima, riaprire diventa **l'unico modo** di dire «non è risolto». Il motivo
smette quindi di essere un di più: è l'unica cosa che spiega perché la segnalazione è
tornata viva, e `POST /tickets/{id}/reopen` acquista un corpo con `reason` non vuoto.

**È un'asimmetria con `resolution_notes`, che resta facoltativo**, e ha una ragione: la
chiusura lascia comunque `closed_at` e una riga di storico, mentre la riapertura senza
motivo non lascia niente — e la parte prima ha appena tolto il canale alternativo.

### Dove va il motivo: nel thread

`ReopenTicket` scrive, **nella stessa transazione** della transizione, un messaggio
pubblico con quel testo, di chi ha riaperto. Tre ragioni:

- **il thread non si riscrive mai** — `app.ticket_messages` ha due sole policy, `_select`
  e `_insert` — quindi il motivo sopravvive alla chiusura successiva. `resolution_notes`
  invece viene **cancellato dalla riapertura stessa**, e una colonna simmetrica lo
  sarebbe a sua volta alla chiusura dopo;
- una segnalazione si riapre **più volte**, e tre riaperture hanno tre motivi. Una colonna
  ne tiene uno;
- chiude il cerchio della parte prima invece di lasciarlo aperto: chi vuole dire «non è
  risolto» lo dice riaprendo, e le sue parole finiscono dove finiscono tutte le altre.

### `ticket_status_history.reason` resta non scritta, e non è una svista

È la colonna fatta apposta — esiste dalla `0007`, `TicketStatusChange::reason()` la legge,
il DTO la serializza e il client la **renderizza già** — ed è sempre `null`. Riempirla
avrebbe chiuso un campo morto, e costa un **secondo canale verso il trigger**: la riga la
scrive `tg_ticket_status_history`, che è `SECURITY DEFINER` perché quella tabella non ha
policy di `INSERT`, quindi il motivo dovrebbe arrivargli da una `set_config`
transazionale — il meccanismo di `app.current_user_id()`, che esiste e funziona.

Si scarta per due ragioni. Una `set_config` locale vale per **tutta la transazione**,
quindi una seconda transizione nella stessa richiesta erediterebbe il motivo della prima,
e il rimedio — un trigger che consuma il valore azzerandolo — è macchina nuova per un
campo di testo. E `TicketRepository::update` prende `&Ticket`: il motivo **non è un
attributo dell'aggregato**, e farcelo passare vorrebbe dire o un campo che non appartiene
alla segnalazione, o una firma diversa per una transizione sola.

**La differenza con il numero falso della sezione 4 va detta**, perché le due cose si
somigliano: là `v_property_occupancy` **mostrava** uno zero non vero, qui un campo vuoto
non compare affatto — `HistoryRow` lo renderizza già condizionalmente. Un campo non
riempito è un debito; un numero falso è un difetto.

### Conseguenze

- la guardia della parte prima sta nello use case che **pubblica**, non nel costruttore
  del messaggio: la riapertura scrive il proprio messaggio dopo aver riportato lo stato a
  `open`, e con la guardia nel costruttore inciamperebbe nella propria regola;
- la sequenza comune delle transizioni vive in `run()` (`transition.rs`), che esiste
  perché nessuna dimentichi `TicketStatusChanged`. **Non si duplica per la riapertura**:
  acquista un punto in cui inserire un messaggio dentro la stessa unità di lavoro, e le
  altre sei lo lasciano vuoto;
- il client toglie la casella di scrittura sugli stati terminali e **dice perché**,
  indicando la riapertura a chi ha segnalato; il dialogo di riapertura passa da conferma a
  form con un campo obbligatorio, e il testo avverte che quello che si scrive finisce
  nella conversazione;
- un invariante nuovo, un codice nuovo, un messaggio nel catalogo.

### Alternative scartate

**Ammettere le repliche su `closed`** e vietarle solo su `cancelled` e `resolved`.
Lascerebbe la riapertura facoltativa, e con essa il caso che il modulo è fatto per
evitare: una conversazione che continua sotto uno stato che dice «finita».

**Il motivo facoltativo.** Con la parte prima è l'unica traccia che resta, e un campo
facoltativo che è anche l'unica traccia è un campo che resterà vuoto proprio quando
serve.

**Una colonna `reopen_reason` su `app.tickets`.** Simmetrica a `resolution_notes` e con lo
stesso difetto: ne tiene uno solo, e la chiusura successiva dovrebbe cancellarlo — cioè
perderebbe esattamente ciò per cui esiste.

---

<a id="adr-051"></a>

## ADR-051 — Un inquilino che torna passa dalla stessa porta

**10 settembre 2026.** Nasce da [`R31`](12-da-fare.md#lelenco), trovata **usando** e non
leggendo il codice: mezza giornata di collaudo percorsa da chi non l'aveva scritto.
Emenda [ADR-020](#adr-020) su un punto solo — quando la membership si inserisce — e non
tocca niente del resto.

### Il fatto

Il ciclo di vita normale del prodotto: un contratto scade o si disdice, e con la stessa
persona se ne fa un altro. **Oggi non si può**, e il modo in cui non si può è il peggiore:

1. `InviteTenant` non guarda niente: crea la parte contrattuale ed emette l'invito. Il
   proprietario vede un codice buono e lo consegna;
2. `AcceptInvitation` fa **sempre** `uow.members().insert(...)`, che sbatte contro
   `org_members_org_user_uk`;
3. `map_membership_conflict` lo traduce in `409 membership.already_exists`.

L'inquilino riceve un errore su un invito appena emesso, e non c'è nessuna via
alternativa: `lease_parties.user_id` **lo scrive solo quel percorso**. `PATCH
/lease-parties/{id}` tocca nome, telefono, ruolo, quota e intestatario, non l'utente.

**Nessun test lo copriva, e la ragione vale più del difetto**: ogni test di integrazione
costruisce un inquilino **nuovo**. Il difetto vive nella *seconda* volta che qualcosa
succede alla stessa persona, che è esattamente lo spazio che una suite verde non
attraversa. Vale la pena portarsi dietro la domanda: *quali altri percorsi il prodotto
attraversa solo la seconda volta?*

### La decisione

`AcceptInvitation` verifica la membership prima di inserirla: se chi accetta è **già
membro attivo** di quella organizzazione, salta l'inserimento e prosegue a collegare la
parte contrattuale e a marcare l'invito.

**Il ruolo non si tocca.** Chi è già `owner` o `manager` e viene invitato come `tenant`
resta quello che è: l'invito serve a collegare la parte contrattuale, non a retrocedere
qualcuno. È anche il caso vero di [01](01-prodotto.md) — chi affitta una casa e ne abita
un'altra.

### Perché una via sola e non due

L'invito è già il gesto che il proprietario compie: lo ha appena compiuto, e ha ricevuto
un codice buono. Una rotta «collega una persona già dentro» darebbe **due meccanismi per
lo stesso fatto**, e il proprietario dovrebbe sapere in anticipo quale gli tocca — cioè
ricordarsi se quella persona è già stata sua inquilina.

### La guardia è sulla membership **attiva**, e oggi non serve a niente

`org_members` ha `status` e `deleted_at`, ma nessuno li valorizza finché
[`R16`](12-da-fare.md#uscire-da-unorganizzazione) non esiste: oggi da un'organizzazione
non si esce, quindi non c'è nessuno da far rientrare e la distinzione fra «una
membership» e «una membership attiva» non cambia il comportamento di una sola riga.

Va scritta lo stesso, ed è il punto per cui questa voce esiste. Il giorno in cui `R16`
arriva, una guardia che accettasse una membership *qualunque* farebbe rientrare chi è
stato fatto uscire — e sarebbe **questa** la porta di servizio, trovata mesi dopo in
un'altra sezione, quando nessuno starà più guardando qui.

Il conflitto sull'indice **resta mappato**: `org_members_org_user_uk` non è parziale su
`deleted_at` e `OrgMemberRepository::find` filtra il soft delete, quindi una membership
non attiva non si vede e l'`INSERT` sbatte comunque. Il giorno in cui `R16` la produrrà,
quel `409` sarà la domanda giusta da porsi e non un errore da nascondere.

### Che cosa non cambia di ADR-020

**L'ordine delle scritture.** `app.has_pending_invitation` abilita la membership e il
collegamento della parte, e diventa falso appena l'invito è marcato accettato: a renderlo
falso è l'**ultima** scrittura, quindi saltare la **prima** non sposta le altre due. La
`SELECT` aggiunta passa dal primo ramo di `org_members_select` — `user_id =
app.current_user_id()` — che non chiede di essere già membri, quindi non serve nessuna
policy nuova e **nessuna migration**.

**La verifica del token e le due difese indipendenti** restano identiche: l'applicazione
prova il possesso del token, la RLS prova che un invito pendente esiste.

### Verifica

Due test di integrazione, e provano i due lati. Il primo percorre la riproduzione che ha
prodotto questa voce — invito, accettazione, secondo invito, seconda accettazione — e
pretende `200` con **due** contratti visibili e **una sola** appartenenza; il secondo fa
accettare un invito come inquilino a chi è già `owner` e pretende che il ruolo resti
`owner`. Il secondo è l'assert che qualcuno toglierebbe «uniformando» il ramo.

### Alternativa scartata

**`PATCH /lease-parties/{id}` che accetta `user_id`.** Sposta la scelta della persona su
chi amministra, che dovrebbe cercarla in un elenco — e l'elenco delle persone di
un'organizzazione **non esiste ed è aperto apposta** ([ADR-036](#adr-036),
[ADR-043](#adr-043)). Costruirlo per questo sarebbe riaprire quella domanda per un
problema di comodità.

---

<a id="adr-052"></a>

## ADR-052 — Il codice di recupero si incolla, non si clicca

**10 settembre 2026.** Scritta **prima** del codice, come il metodo impone e come
[ADR-051](#adr-051) non è stata. Governa il recupero della password: la forma del
codice, che cosa fa alle sessioni, che cosa risponde a chi lo chiede e come si limita.

### Il fatto

Le rotte di autenticazione sono quattro — `register`, `login`, `refresh`, `logout` — e
non c'è **nessun** modo di recuperare una password. Il primo «non mi ricordo la
password» di un utente vero finisce in una `UPDATE` scritta a mano sul database di
produzione. È la ragione per cui la posta ([Z1](11-piani-conclusi.md#piano-utenti-veri))
viene prima: senza un canale non c'è niente da recapitare.

### 1. Un codice da incollare, non un link da cliccare

[ADR-045](#adr-045) ha già deciso questa domanda per l'invito, e l'argomento vale
identico: **il token è una credenziale**, e un link `/nuova-password/<token>` la scrive
in tre archivi che nessuno ripulisce — la cronologia del browser, il `Referer` verso
qualunque risorsa esterna, e i log di ogni proxy davanti al client.

L'email porta quindi un codice e la schermata ha un campo in cui incollarlo. Nessun
meccanismo nuovo: `OpaqueTokenFactory` e `TokenHash` esistono per l'invito, e in
database finisce lo hash.

**Il costo è reale e va detto**: incollare è più scomodo di un clic, e succede nel
momento in cui una persona è già frustrata. L'alternativa — il link, con la pagina che
ripulisce l'URL e `Referrer-Policy: no-referrer` — chiuderebbe due archivi su tre e
introdurrebbe **due forme diverse** per due credenziali che si somigliano. Si sceglie la
coerenza; se un collaudo dirà che l'attrito è troppo, sarà una voce nuova con una misura
dietro invece di un'ipotesi.

### 2. Confermare revoca **tutte** le sessioni

È la parte che si dimentica, ed è quella che rende il recupero una difesa invece di una
comodità: chi ha ottenuto l'accesso a un account ha in mano un refresh token che vive
**trenta giorni**, e cambiare la password senza revocarlo non lo butta fuori.

`SessionRepository::revoke_all_for_user` esiste già — la usa il rilevamento del riuso di
un token ruotato — quindi è una chiamata e non un meccanismo. Chi recupera resta fuori
anche lui e rifà l'accesso: è il comportamento giusto, perché non sa quali sessioni
fossero sue.

**Ma «tutte le sessioni» sono i refresh token, non gli access token**, e la differenza va
detta perché la frase si legge come «lo butta fuori adesso». L'access token è un JWT
firmato e **non è revocabile** ([ADR-012](#adr-012)): chi ne ha uno in mano resta dentro
fino alla scadenza, quindici minuti. La revoca chiude la catena — nessun rinnovo, e da lì
in poi è fuori — ma la finestra esiste.

È il baratto che ADR-012 ha già scelto, e recuperare la password non lo cambia: chiuderla
vorrebbe dire una lista di revoca consultata a ogni richiesta, cioè uno stato condiviso
per ogni chiamata di ogni utente per coprire quindici minuti. **Scoperto scrivendo il
test**, che asseriva la cosa sbagliata — che la vecchia sessione smettesse subito di
leggere — e che ora asserisce quella giusta: il refresh non rinnova più.

### 3. La risposta è la stessa per un indirizzo noto e per uno sconosciuto

Regola di [ADR-017](#adr-017), che il modulo degli inviti applica già: distinguere
sarebbe un oracolo per sapere chi ha un account. La rotta risponde `202` sempre, e la
schermata dice «se quell'indirizzo ha un account, riceverà un codice» — che è la verità
e non una scusa.

**E il tempo di risposta è uguale**, che è la metà che si dimentica insieme. Per un
indirizzo senza account non si crea nessuna riga: un ramo fa una lettura, l'altro fa una
lettura, calcola uno hash e inserisce. ADR-017 governa il **contenuto** della risposta,
non la sua durata, e un tempo che distingue è un oracolo come un codice che distingue. Il
percorso è quindi a **tempo costante**: anche per un indirizzo sconosciuto si calcola lo
hash di un valore fittizio.

**Ne segue che l'invio non può essere riportato al chiamante.** Si tenta solo per un
indirizzo che ha un account, quindi qualunque cosa distingua i due rami — un codice di
errore, un messaggio, una durata — riapre l'oracolo. Un relay caduto è perciò **muto**, e
il rimedio non sta nella risposta: sta nel controllo di prontezza, che interroga SMTP
apposta perché una posta ferma si sappia **prima** che la incontri chi è chiuso fuori
(vedi `Notifier::probe`).

### 4. Il limite agli invii è per **email**, e sta nel modello

Un recupero senza limiti è un modo per far arrivare cinquanta email a un indirizzo
altrui. Il rimedio **non è il rate limiting per IP**: quello vuole uno stato condiviso, un
servizio in più da avviare, mettere in sicurezza, monitorare e ripristinare, e non serve
con una sola istanza — resta fra i trasversali mancanti.

È invece una colonna: quando è stato emesso l'ultimo codice per quell'email, e un rifiuto
**silenzioso** se è troppo recente. Silenzioso perché dirlo sarebbe di nuovo l'oracolo del
punto 3.

### 5. Il pool privilegiato è legittimo qui, ed è l'unico posto nuovo in cui lo diventa

La regola 27 di `CLAUDE.md` riserva `begin_privileged()` ai flussi in cui un contesto di
sicurezza **non può** esistere: registrazione, login, rotazione del refresh token. Chi ha
dimenticato la password non è autenticato **per definizione**, quindi non ha un contesto
da installare e la Row Level Security non ha nulla su cui appoggiarsi.

È la differenza con l'accettazione di un invito, dove [ADR-020](#adr-020) ha scelto di
estendere la RLS invece di aggirarla: là il contesto **esiste** — chi accetta è
autenticato — e manca solo la membership. Qui non esiste affatto.

Va scritto nel commento del caso d'uso, o alla prossima lettura sembrerà una violazione
della regola.

### Conseguenze

- una migration nuova: la tabella dei codici, con lo hash e non il codice, una scadenza
  breve, l'uso singolo e la colonna che regge il limite;
- due rotte, `POST /api/v1/auth/password-reset` e
  `POST /api/v1/auth/password-reset/confirm`, e due schermate nel client;
- una variante nuova di `NotificationContent`, con il suo `Debug` redatto: il codice è
  una credenziale come il token d'invito;
- **il cambio password da autenticati non arriva con questa voce.** Sta in
  [`R7`](12-da-fare.md#interfaccia-e-prodotto) e condivide con il recupero la revoca
  delle sessioni e la validazione, quindi sembra incluso e non lo è.

### Alternativa scartata

**Una password provvisoria spedita per email.** Toglie la tabella e il giro in due tempi,
e mette una credenziale **valida** in un archivio che non si ripulisce — la casella di
posta — invece di un codice a uso singolo che scade. È lo stesso argomento del punto 1,
peggiorato: il link almeno non era la password.

---

<a id="adr-053"></a>

## ADR-053 — Si entra per invito: l'inquilino nel conto di chi lo ospita, il proprietario con un token di piattaforma

**13 settembre 2026.** Scritta **prima** del codice, come il metodo impone: è il primo
passo della [sezione 8](11-piani-conclusi.md#piano-sezione-8), e la sezione non scrive
una riga finché queste sei domande non hanno una risposta. Chiude la parte di `R9` che è
una decisione; il resto di `R9` è il codice che ne discende.

### Il fatto

Chiunque si registri diventa **owner di un'organizzazione propria**: `RegisterOwner` è
l'unico caso d'uso di registrazione e chiama `Organization::personal(...)` e
`OrgMember::active(..., OrgRole::Owner, ...)` senza un ramo. Un inquilino invitato deve
registrarsi *prima* di accettare — `AcceptInvitation` è `Authenticated` — quindi **ogni
inquilino è anche owner di un'organizzazione vuota**, dove può registrare immobili a
ragione: la Row Level Security e `ensure_staff` glielo concedono, e il client lì gli
mostra «Registra un immobile» perché `useIsStaff()` dice il vero.

[ADR-042](#adr-042) lo dichiarava: il ramo sul ruolo nel client è una scelta di che cosa
offrire e **non un confine**. Il confine mancava nel modello, ed è tollerabile finché il
prodotto è «per un proprietario che conosci». Smette di esserlo il giorno in cui la
registrazione è aperta a sconosciuti — cioè il deploy.

Verificando le premesse del piano sono emerse altre due cose che questa decisione deve
coprire: il login preferisce la membership `Owner` (`pick_default_membership`), quindi un
inquilino appena registrato atterra **nella propria organizzazione vuota** e non in
quella di chi lo ha invitato; e un utente **senza nessuna organizzazione** è uno stato
che il guscio non conosce — l'effetto di riallineamento esce su `items.length === 0`, e
ogni elenco risponderebbe `403 org.not_selected`.

### 1. Un utente nato inquilino non ha un'organizzazione propria

La membership arriva **solo con l'invito**. La registrazione di un inquilino parte
dall'invito — il codice ricevuto per email, incollato come in [ADR-045](#adr-045) — e
finisce nell'organizzazione di chi lo ha invitato, con la parte contrattuale collegata,
**in una sola transazione**: utente, membership, collegamento, invito marcato. Non
esiste un istante in cui l'inquilino è registrato e senza organizzazione.

*Alternativa scartata: l'organizzazione personale come oggi, nascosta meglio.* È `R9`
stesso; nascondere non è un confine, e ADR-042 l'aveva già detto.

*Alternativa scartata: un'organizzazione personale «spenta»*, con una colonna e un ramo
in ogni policy dello staff. Sembra economica ed è una migration larga più un **secondo
criterio di staff** da tenere allineato al primo per sempre: `is_org_staff` è chiamata
in ogni policy di scrittura dello schema.

**Conseguenza sul guscio.** Lo stato «nessuna organizzazione» diventa uno stato di
prima classe, anche se la transazione sopra fa sì che nessun utente nuovo ci nasca:
ci si arriva **uscendo** dall'unica organizzazione che si aveva (sezione 6, `R16`) —
da inquilino, da gestore, o da proprietario che non era l'ultimo. *(Emendato il 16
settembre 2026: la prima stesura diceva «o quando l'invito che ci aveva portati dentro
viene revocato», ed è impreciso — un invito accettato non si revoca, e uno non accettato
non ha mai fatto entrare nessuno.)* Il guscio mostra una
schermata che dice che cosa si è e che cosa si aspetta — un invito — invece di una
navigazione vuota che risponde `403`. `getOrganizations` e `/me` funzionano già senza
`X-Org-Id` per la ragione del [poscritto della 6](11-piani-conclusi.md#poscritto-sezione-6).

### 2. Un proprietario nasce da un invito di piattaforma

Un genere di invito nuovo, **senza organizzazione**: la crea l'accettazione. Chi lo
accetta si registra e riceve ciò che `RegisterOwner` dà oggi a chiunque — utente,
organizzazione personale, membership `owner` — in una transazione, dietro un token.
`RegisterOwner` smette di essere una rotta aperta.

**Tabella nuova, `app.owner_invitations`**, e non `org_invitations` con `org_id`
annullabile: `org_invitations_staff` è `FOR ALL USING (is_org_staff(org_id))`, e un
`org_id` nullo la renderebbe falsa o da riscrivere; l'indice parziale
`org_invitations_pending_uk` è su `(org_id, email)`; e ogni query su quella tabella
erediterebbe un ramo «senza organizzazione» che oggi non ha. La tabella nuova ha la forma
di `password_reset_codes` (migration `0037`, [ADR-052](#adr-052)): `email`,
`token_hash`, `expires_at`, `accepted_at`, `accepted_by`, `revoked_at`, `created_at`,
nessun `org_id`, nessun `deleted_at`. Lo hash è SHA-256 del token in chiaro, come
`OpaqueTokenGenerator` fa per inviti e sessioni.

*Alternativa scartata: registrazione libera più una colonna «verificato»* che sblocca lo
staff. Lascia aperta la porta della registrazione e sposta il confine in ogni policy
dello staff — lo stesso costo dell'organizzazione «spenta», con in più un utente che
esiste e non può fare niente.

### 3. La verifica è fuori dal prodotto, e il token si emette da un runbook

Come si accerti che una persona sia un proprietario — un'email, una telefonata, un
documento — è una decisione di chi amministra la piattaforma e **non vive nel prodotto**.
Nel prodotto vive solo il fatto che il token esiste.

Il token lo emette uno **script in `ops/`**, con il ruolo del migratore, documentato in
[07](07-operazioni.md): genera 256 bit casuali, li codifica come fa
`OpaqueTokenGenerator` (`base64url` senza padding), scrive lo hash nella tabella e
stampa il token in chiaro **una volta**, perché lo si mandi. Nessuna rotta HTTP:
sarebbe la prima rotta *di piattaforma* del prodotto, un perimetro di sicurezza nuovo
da difendere e testare per un gesto che oggi si fa qualche volta al mese. `platform_role`
resta com'è — assegnato a mano, valore `'admin'` — e **questa decisione non lo usa**: il
giorno in cui l'emissione diventa una rotta, `is_platform_admin()` è il suo guardiano,
ed è già scritto.

### 4. Gli utenti esistenti: il database si svuota

Non c'è produzione, e non c'è nessun utente che non sia un dato di prova: il primo
deploy è [rinviato](12-da-fare.md#lordine-di-esecuzione) per mancanza di mezzi, e i
database locali si ricreano. **Nessuna migration di dati**, nessuna regola per
distinguere «chi è un proprietario» da «chi si era registrato per un invito», nessuna
archiviazione di organizzazioni personali vuote. Chi ha un database locale con dei dati
lo azzera (`just down-hard`, poi `just setup`) quando prende questa sezione.

Va detto che cosa questa scelta **non** copre, perché è il caso che tornerà: il giorno in
cui ci sarà un database con utenti veri, un cambiamento della stessa specie avrà bisogno
della migration che qui non si scrive. Questo ADR non la anticipa.

### 5. Il login non cambia

`pick_default_membership` preferisce `Owner`, poi `Manager`, poi `Tenant`. Con la
decisione 1 un inquilino non ha una membership `Owner` da preferire, e con la 4 non
esistono utenti vecchi che ce l'abbiano: il caso che rendeva l'ordine sbagliato
**sparisce** invece di essere corretto. Per chi è owner di un'organizzazione e inquilino
in un'altra — il caso vero di [01](01-prodotto.md) — preferire quella che amministra
resta giusto.

### 6. `/invito` è l'ingresso di chi è invitato, per entrambi i generi

Un codice incollato in un campo, come [ADR-045](#adr-045); se la persona non ha un
account, la registrazione **parte da lì** e porta il codice con sé, invece di mandare a
una registrazione che crea un'organizzazione. I due generi di invito si distinguono dal
codice — il server sa in quale tabella cercarlo — e il form chiede le stesse cose:
nome, password. Il nome dell'organizzazione lo chiede solo a chi accetta un invito da
proprietario, perché solo lui ne crea una.

**L'email della registrazione è quella dell'invito**, e non un campo: un invito è per
una persona, e la policy `has_pending_invitation` confronta l'email dell'utente con
quella dell'invito. Chi vuole un altro indirizzo chiede un altro invito.

**Il genere del token lo dice il server, e solo a chi lo possiede** *(aggiunto il 16
settembre 2026, confermato dal passo `F7`)*. Il form non sa se il codice incollato è da
inquilino o da proprietario, e **non esiste una rotta di sola lettura** che glielo
dica: risponderebbe anche per codici che chi chiede non possiede, cioè un oracolo su
quali token sono stati emessi. Il form chiede nome e password e manda a
`register-from-invitation`; se il token è un invito da proprietario **valido** il
server risponde `409 invitation.owner_token`, e solo allora il form chiede il nome
dell'organizzazione e rimanda lo stesso corpo a `accept-owner-invitation`. Chi
indovina un token riceve `401` come per un token inesistente: il `409` esce solo a chi
ha in mano un token che darebbe comunque un account.

**Le risposte a «token inesistente» e a «token valido, ma già usato o scaduto» sono la
stessa** (`401 auth.invalid_credentials`, come già per l'invito e per il codice di
recupero): un token è una credenziale, e distinguere i due casi darebbe un oracolo su
quali token sono stati emessi.

### Conseguenze

- `RegisterOwner` diventa l'accettazione di un invito da proprietario; nasce la
  registrazione dell'inquilino dall'invito; `AcceptInvitation` resta per chi **ha già**
  un account — l'inquilino che torna di [ADR-051](#adr-051), o chi è owner altrove;
- entrambe le registrazioni girano sul pool privilegiato (regola 27: prima
  dell'autenticazione non c'è contesto) e **senza Row Level Security**: la prova di
  possesso del token è dell'applicazione, per hash e per stato, e l'email dell'invito
  vincola quella dell'utente;
- il guscio ha uno stato «nessuna organizzazione»;
- l'harness dei test smette di costruire ogni inquilino come owner di qualcosa: è la
  prima cosa da cambiare, con la suite verde, **prima** del prodotto;
- una migration (`app.owner_invitations`, con la sua policy e i privilegi di colonna
  nella forma della `0019`), uno script in `ops/`, un runbook in [07](07-operazioni.md);
- `R9` esce da [12](12-da-fare.md) con l'esito della sezione 8; `R41` — chiedere di
  rientrare — resta dov'è: questa decisione non la tocca.

<a id="adr-054"></a>

## ADR-054 — Chi è uscito non legge più: la membership nel ramo dell'inquilino, al livello della RLS

**18 settembre 2026**, sezione 10. Chiude `R46`.

### Il fatto

Chi esce da un'organizzazione continua a leggerne i dati — e a scriverci — per i
quindici minuti del suo access token. `middleware::context` si fida del ruolo nel token
quando `X-Org-Id` coincide con l'`org_id` dei claim, senza rileggere la membership: è il
baratto del JWT non revocabile di [ADR-012](#adr-012), che parla di *validità* del token
e non di *membership*. Per lo staff quei minuti sono già vuoti: ogni policy passa da
`member_role`, che filtra `status = 'active'`
([poscritto della 6](11-piani-conclusi.md#poscritto-sezione-6)). Per l'inquilino no: il
ramo suo in `app.can_access_lease` e `app.can_access_property` (`0014`) passava da
`lease_parties.user_id = current_user_id()` **senza guardare `org_members`**, e uscire
(`0039`) tocca `org_members` e non le parti. `GET /leases` rispondeva `200` **con il
contratto disdetto nel corpo**, e con lo stesso token si apriva una segnalazione
nell'organizzazione lasciata.

Prima di [ADR-053](#adr-053) il caso era raro — il token di un inquilino portava la sua
organizzazione personale, mai quella di chi lo ospitava, quindi la rilettura scattava
sempre. Dopo, porta quella di chi lo ospita: è il caso di **ogni** inquilino che esce.

### Decisione

**`AND app.is_org_member(<org>)` nel ramo dell'inquilino delle due funzioni**, con una
migration che le riscrive per intero (`0041`, `CREATE OR REPLACE` con la stessa firma:
le policy referenziano per oid e non si toccano). Il difetto sta nel solo ramo che non
guardava la membership, e si chiude lì, al livello che una lettura non può aggirare.

Il costo: una lettura in più di `org_members` per riga valutata, sull'indice che
`member_role` usa già. Le funzioni sono `SECURITY DEFINER`, e `is_org_member` anche.

`member_role` **non si tocca**: è la funzione di tutte le policy dello staff, e un
cambiamento lì cambia tutto.

### Alternativa scartata

**Rileggere la membership in `Authenticated` a ogni richiesta.** Chiuderebbe il caso
per tutte le rotte, ma cambia la forma del baratto di ADR-012 per tutti — staff
compreso, dove non c'è nulla da chiudere — e sposta il confine dalla riga alla
richiesta: una query in più per ogni chiamata, per un difetto che sta in un ramo di due
funzioni. Resta la via da riconsiderare se un giorno un altro ramo si rivelasse cieco
alla membership; oggi non ce n'è.

### Conseguenze

- il token vecchio, dopo l'uscita, vede un elenco **vuoto** dove prima vedeva il
  contratto: `200` con zero elementi, come per lo staff, non `403`. È coerente con il
  cambio di organizzazione, che non distingue «non esiste» da «non ne fai parte»;
- aprire una segnalazione nell'organizzazione lasciata risponde `404
  property.not_found`: l'immobile non si legge, e il bordo lo traduce come traduce ogni
  risorsa che «non è tua» — non `403`, che direbbe che la risorsa c'è;
- il test `a_tenant_leaves_an_organization_unless_a_lease_is_in_force_and_can_come_back`
  smette di fare un login nuovo dopo l'uscita e afferma con il **token vecchio**;
- il rientro ([ADR-051](#adr-051)) non cambia: riattivata la membership, la parte del
  contratto torna a valere.


---

<a id="adr-055"></a>

## ADR-055 — Il totale incassato si somma oltre la RLS: una funzione, non una vista più larga

**Contesto, e il difetto misurato.** `app.v_charge_balances` è `security_invoker = true`
dalla `0015`: la vista applica la Row Level Security **di chi legge**, ed è la scelta
giusta per l'addebito — chi non può vedere quella riga non ne legge il saldo. Ma il
`paid_amount` di quella vista è una somma su `app.payments`, e `payments_select`
(`0014:299-302`) mostra un pagamento a tre soggetti: lo staff, chi ha pagato
(`payer_user_id`), chi ha incassato (`payee_user_id`). Un incasso registrato **senza**
`payer_user_id` — il bonifico che arriva da un conto che non si sa a chi ricondurre, che
`submit.rs` dichiara voluto e che resta tale — è invisibile all'inquilino, e quindi **non
entra nella sua somma**.

Misurato seminando i dati del collaudo: il proprietario registra `150,00` su un addebito
da `420,00` senza attribuirlo; l'inquilino, sulla **stessa riga**, legge `status:
partially_paid` con `paid_amount: 0.00` e `outstanding_amount: 420.00`. La riga si
contraddice da sola — si dichiara pagata in parte e mostra il residuo intero — e chi la
legge pagherebbe due volte. È la voce `R29`.

Lo stato è scritto dal service layer ([ADR-015](#adr-015)) e quindi è lo stesso per
tutti; la somma è derivata sotto la RLS e quindi cambia con chi guarda. **La
contraddizione nasce da qui**, e non da un difetto di una delle due scelte.

### Decisione

`app.charge_paid_amount(p_charge_id uuid) RETURNS app.money_amount`, `SECURITY DEFINER`,
somma i pagamenti di un addebito **senza la Row Level Security di chi chiama**;
`app.v_charge_balances` la chiama al posto della `LATERAL` che sommava in linea
(`migrations/0042_charge_paid_amount_reads_past_rls.sql`).

Il predicato sommato è **lo stesso di prima, parola per parola**: `status = ANY
(ARRAY['succeeded','partially_refunded'])` e `SUM(amount - refunded_amount)`. Non sono i
tre stati di `payments_payable_ix`, che comprende anche `authorized` — denaro autorizzato
e non incassato, che farebbe scendere il saldo su un incasso mai arrivato. Spostare la
somma dentro una funzione non è l'occasione per cambiare che cosa si somma: cambierebbero
due cose insieme, e il giorno in cui un saldo risultasse sbagliato non si saprebbe quale.

**Che cosa rivela, e a chi.** Un **aggregato** — quanto è stato incassato su un addebito —
a chi può vedere **quell'addebito**, cioè a una parte del suo contratto o allo staff
(`charges_select`, `0014:291`, via `app.can_access_lease`). La funzione riceve un
`charge_id` e restituisce un `numeric`: **non** chi ha pagato, **non** quando, **non**
quante righe, **non** il metodo. `payments_select` non si tocca, e l'elenco dei pagamenti
continua a mostrare all'inquilino i soli suoi.

È la forma di [ADR-036](#adr-036): una funzione che **proietta** invece di una policy che
si allarga. Là erano tre colonne dei residenti, qui è un solo numero.

### Perché non le altre due vie

La voce `R29` ne registrava tre, e le altre due sono state scartate con ragioni diverse.

**Rendere `payer_user_id` obbligatorio** quando l'addebito ha un contratto con un solo
inquilino registrato. È la più piccola, e non chiude il caso: il bonifico anonimo — un
incasso che non si sa a chi ricondurre — esiste e continuerebbe a esistere sui contratti
con più inquilini, che sono esattamente quelli del co-living. Chiuderebbe il caso
frequente lasciando aperto quello che si nota di meno, che è il modo peggiore di chiudere
un difetto di correttezza.

**Non mostrare affatto il totale** a chi non vede tutte le righe. È l'unica che non
richiedeva di decidere subito, ed è la ragione per cui la voce la registrava. Toglie però
a chi abita **l'unico numero per cui apre la prima schermata** — *quanto resta da pagare*
— e lo sostituisce con un'assenza che non spiega niente. Un residuo falso è peggio di un
residuo assente, ma un residuo vero è meglio di entrambi, e costa una funzione.

### Conseguenze

- **`v_charge_balances` non è più interamente derivabile da ciò che il chiamante vede.**
  Resta `security_invoker`, quindi **quali righe** si leggono non cambia: cambia che una
  colonna di ciascuna è sommata oltre la policy. È una proprietà che
  [04](04-sicurezza.md) ora descrive, ed è la ragione per cui questa decisione è un ADR e
  non una riga di migration;
- `v_owner_dashboard` legge da `v_charge_balances` e ne eredita il `paid_amount`: per lo
  staff non cambia nulla, perché `payments_select` gli mostrava già tutto;
- il `CREATE OR REPLACE VIEW` conserva **le stesse colonne, nello stesso ordine e con gli
  stessi tipi**, che è la condizione perché il `REPLACE` sia ammesso e perché
  `v_owner_dashboard` non venga portata via da un `DROP`;
- la funzione ha `SET search_path = app, pg_temp` — l'invariante 6 di
  `ops/sql/checks/schema_invariants.sql` ferma un `SECURITY DEFINER` che ne sia privo — e
  in più `REVOKE ALL ... FROM PUBLIC` seguito dai `GRANT EXECUTE` ai ruoli. **Il `REVOKE`
  è nuovo in questo repository**: `app.property_residents` (`0030`) concede per esplicito
  ma non toglie il default di `PUBLIC`, e il controllo delle invarianti non se ne accorge.
  Qui si fa, perché una funzione che legge oltre la RLS eseguibile da chiunque abbia una
  connessione è la cosa che il `DEFINER` doveva evitare;
- lo stato dell'addebito e il suo saldo **tornano a dire la stessa cosa a tutti**, che è
  l'effetto per cui la decisione esiste.
