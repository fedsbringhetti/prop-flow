# PropFlow — istruzioni operative

Piattaforma PropTech bidirezionale (proprietari ↔ inquilini) in Rust. Backend
Axum, PostgreSQL con Row Level Security, architettura a porte e adattatori.

---

## 1. Leggere la documentazione è obbligatorio, non consigliato

**Prima di scrivere codice, progettare, o rispondere a domande di architettura,
leggi i documenti pertinenti in `docs/`.** Non è una formalità: diverse scelte di
questo progetto sono controintuitive e sono state prese dopo aver visto un bug
concreto. Riderivarle a intuito significa reintrodurre quel bug.

Esempi di cose che *sembrano* sbagliate e non lo sono:

- le policy RLS **non** filtrano il soft delete — vedi `docs/08-decisioni.md#adr-008`
- il migratore fissa `search_path=public` — `docs/08-decisioni.md#adr-009`
- le query SQL sono letterali statici composti con `concat!` — `docs/08-decisioni.md#adr-006`
- gli enum si leggono con cast a `text`, non con `#[derive(sqlx::Type)]` — `adr-007`
- esistono **due** pool di connessione e usarne uno al posto dell'altro è un buco di sicurezza — `docs/04-sicurezza.md`
- **CORS non è montato, e non è un debito**: client e API stanno sulla stessa origine, e non montarlo è metà dell'argomento per cui non serve una difesa CSRF — `docs/08-decisioni.md#adr-038`
- il refresh token **non è nel corpo della risposta**, è solo un cookie — `adr-038`

Partenza: **`docs/README.md`**, che è la mappa. Poi, in base al compito:

| Se il compito riguarda | Leggi prima |
|---|---|
| qualunque cosa | `docs/README.md`, questo file |
| un nuovo endpoint o un DTO | `docs/05-api.md`, `docs/02-architettura.md` |
| un nuovo use case o regola di dominio | `docs/02-architettura.md`, `docs/06-sviluppo.md` |
| lo schema, una migration, la RLS | `docs/03-modello-dati.md`, `docs/08-decisioni.md` |
| autenticazione, token, permessi, PII | `docs/04-sicurezza.md` |
| il client web, una schermata, una stringa | `docs/08-decisioni.md#adr-039`, §2 di questo file |
| deploy, configurazione, CI | `docs/07-operazioni.md` |
| capire perché una cosa è fatta così | `docs/08-decisioni.md` |
| sapere cosa manca | `docs/09-stato-e-roadmap.md` |
| sapere perché un modulo è fatto così | `docs/11-piani-conclusi.md`, l'esito della fetta che lo ha prodotto |

Se la domanda è «cosa faccio adesso», la risposta è `docs/09-stato-e-roadmap.md`:
stato attuale, prossimo passo suggerito, debiti noti e ciò che è deliberatamente
fuori perimetro. È anche il posto dove un piano deve atterrare prima di essere
implementato — vedi §3.

Se dopo aver letto resta un'ambiguità, chiedila invece di inventare una
convenzione nuova.

---

## 2. Regole vincolanti

Queste non si negoziano senza discuterne prima. Sono verificate da CI dove
possibile.

### Architettura

1. **`propflow-domain` non dipende dall'infrastruttura.** Niente `sqlx`, `axum`,
   `tokio`, client HTTP. Verificato da `ops/ci/check-domain-purity.sh`.
2. **`propflow-application` conosce solo porte astratte**, mai
   `propflow-infrastructure`. Stesso controllo.
3. **Nessuna logica di business negli handler HTTP.** Un handler deserializza,
   chiama uno use case, serializza. Niente `if` di dominio, niente SQL: il
   controllo grep in CI fallisce se compare SQL in `crates/api/src/routes/`.
4. **Un repository si ottiene solo da `UnitOfWorkFactory::begin(ctx)`.** È il
   punto dove viene installato il contesto RLS. Non aggiungere modi alternativi
   di raggiungere il database.
5. **I provider esterni stanno dietro un trait** dichiarato in
   `domain::ports` (persistenza) o `application::ports` (servizi tecnici).
   L'implementazione concreta vive solo in `propflow-infrastructure`.

### Database

6. **Le migration sono append-only.** Uno schema già mergiato non si modifica: si
   aggiunge una migration nuova. Numerazione `NNNN_descrizione.sql`.
7. **Cancellazione soft.** `deleted_at`, mai `DELETE`. Il ruolo applicativo non ha
   il privilegio, quindi un `DELETE` fallisce a runtime.
8. **Ogni tabella di business ha `org_id`** e una policy RLS che isola per
   organizzazione.
9. **Le policy RLS esprimono isolamento, non filtri di business.** Il filtro
   `deleted_at IS NULL` va nel `WHERE` delle query.
10. **Ogni query filtra esplicitamente `deleted_at IS NULL`** quando legge entità
    con soft delete.
11. **Gli oggetti sono sempre qualificati con `app.`** nelle migration.

### Rust

12. **`unwrap`, `expect`, `panic!`, `todo!` sono vietati** nel codice di
    produzione (lint di workspace). Ammessi nei test, dove un panic è il modo
    corretto di segnalare una precondizione non soddisfatta.
13. **`unsafe` è proibito** (`unsafe_code = "forbid"`).
14. **Zero warning.** La CI usa `-D warnings`.
15. **Le stringhe SQL sono letterali statici.** Comporre con `concat!` e macro,
    non con `format!`. Non usare `AssertSqlSafe` per aggirare il vincolo.
16. **Le versioni delle dipendenze stanno in `[workspace.dependencies]`**, i
    membri usano `dep.workspace = true`.
17. **Errori tipizzati con `thiserror`**, mai `anyhow` nelle librerie. `anyhow`
    solo nei binari.

### TypeScript e il client web

Il client vive in `apps/web/`. `CLAUDE.md` era interamente su Rust, e «niente
`unwrap`», «errori con `thiserror`», «query come letterali statici» non hanno una
traduzione in TypeScript: lasciare il frontend senza regole significherebbe
lasciarlo senza criterio. Sono poche e valgono tutte.

18. **L'interfaccia è in italiano, e nessuna stringa visibile sta dentro un
    componente.** Le stringhe vivono in `apps/web/src/i18n/it.ts`; i componenti ne
    usano le chiavi. Non contraddice la regola sull'inglese di §3, che riguarda ciò
    che si incontra **dal di fuori del repository**. La traduzione in inglese è
    decisa e solo rimandata: una fetta con i testi nel JSX la rende una riscrittura
    invece di un'aggiunta, e il livello di indirezione è la parte impossibile da
    aggiungere dopo ([ADR-039](docs/08-decisioni.md#adr-039)).
19. **Date, importi e numeri passano da `Intl`, con la lingua come parametro.**
    `@/lib/format`, mai una formattazione a mano: altrimenti `31/08/2026` e
    `184,20 €` restano italiani anche in inglese.
20. **Niente `any`, `strict` acceso.** È l'equivalente di «zero warning»: il
    typecheck e il lint girano in CI e negano `any` esplicitamente.
21. **Gli importi restano stringhe, e `parseFloat` è vietato.** L'API li manda come
    `"184.20"` apposta. Sommare o confrontare passa da `@/lib/decimal`, che lavora
    su interi. Vale anche per `confidence`, che ha quattro decimali.
22. **Nessun `fetch` fuori dal livello dei dati.** È l'equivalente di «nessun SQL
    negli handler»: si passa da `ApiClient` (`@/api`), che allega il token e
    l'organizzazione attiva, traduce gli errori e rinnova la sessione. Un componente
    che chiami l'API da sé aggira il **refresh a singolo volo**, e senza quello tre
    richieste parallele scadute insieme fanno revocare la catena delle sessioni.
23. **Ogni stato di una schermata è progettato**: caricamento, vuoto, errore, e non
    solo il caso felice. I tre stanno in `@/components/state` come componenti di
    prima classe, così il caso mancante si vede scrivendo la schermata e non
    usandola.
24. **L'organizzazione attiva è stato esplicito.** Si manda `X-Org-Id` sempre, anche
    quando coincide con quella del token: senza, gli elenchi rispondono `200` con
    zero elementi — una schermata vuota, nessun errore, nessun indizio.

### Sicurezza

25. **Niente dati personali nei log, nelle tracce e negli eventi di dominio.**
    Email, token, password, corpi di richiesta: mai. Gli eventi trasportano
    identificatori e nomi di campo, non valori.
26. **`Debug` non deve mostrare segreti.** `PasswordHash` e `TokenHash` hanno
    implementazioni che stampano `<redatto>`; farlo anche per i tipi nuovi.
27. **Il pool privilegiato (`begin_privileged`) si usa solo dove un contesto di
    sicurezza non può esistere**: registrazione, login, rotazione del refresh
    token. Ogni altro uso è un errore di progetto.
28. **Nessun segreto nel repository.** Chiavi e password arrivano da file montati
    o variabili d'ambiente. `.secrets/` è ignorato.

---

## 3. Metodo di lavoro

### Lingua: inglese su ciò che si vede da fuori

Il criterio, unico e da applicare anche ai casi non elencati qui: **ciò che un
estraneo incontra senza aprire un file è in inglese; la prosa interna del progetto
resta in italiano.** Non conta se il testo vive dentro o fuori dal repository: conta
se lo si legge dall'interfaccia di GitHub, da un changelog o da una pagina di
progetto senza mai entrare nel codice.

**In inglese, senza eccezioni:**

- messaggi di commit, nomi dei branch, messaggi di tag;
- titoli e corpi delle PR, commenti di review, titoli e descrizioni delle issue;
- `README.md` — è la prima pagina del repository;
- `cliff.toml` e tutto ciò che ne deriva: `CHANGELOG.md` e il corpo delle GitHub
  Release, che da lì sono generati. Sbagliare qui sbaglia due superfici in una;
- nei workflow in `.github/workflows/`: i nomi dei **job** e degli **step**, e le
  annotazioni `::error::` e `::warning::`. I job compaiono come nomi dei check nelle
  PR, gli step nel log di esecuzione, le annotazioni in cima al riepilogo.

**In italiano:** `docs/`, questo file, i commenti nel codice Rust, nelle migration,
negli script `ops/` e nei workflow. Sono prosa interna: in italiano è più precisa e
più veloce da scrivere, e tradurre diciotto ADR densi di ragionamento rischia di
perdere esattamente ciò per cui esistono. L'italiano resta anche nelle stringhe che
il codice produce per sé, come il marcatore `SALTATO` dell'harness dei test su cui
la CI fa `grep`: è un contratto interno, non un messaggio per chi legge la pipeline.

**I nomi dei job sono un contratto con la protezione dei branch.** Il job di
riepilogo si chiama `CI` ed è il check richiesto su `main` e `dev`: rinominarlo fa
cercare a GitHub un check che non esiste più e blocca ogni merge finché non si
aggiorna anche la protezione via `gh api`. Gli altri job si rinominano liberamente.

Prima di scrivere un titolo di PR o un messaggio di commit, verifica di essere in
inglese. È l'errore più facile da commettere e il più costoso da correggere dopo,
perché richiede di riscrivere la storia o di riaprire le PR.

### I piani atterrano nella roadmap, corti; il dettaglio si fa passo per passo

Quando in una sessione si progetta un lavoro, **quel piano si scrive in
`docs/09-stato-e-roadmap.md` prima di implementarlo**, e contiene **due cose sole**:

1. **una descrizione breve e generale, in apertura** — che cosa si sta per fare e
   perché, in un paragrafo. È ciò che si legge per capire di che lavoro si tratta
   senza leggere i passi;
2. **i passi, una o due righe l'uno** — che cosa quel passo deve *ottenere*, non come.
   Eseguibili singolarmente, e leggibili in fila per vedere dove va il lavoro.

**Il dettaglio non si scrive qui e non si scrive in anticipo.** Si produce un passo
alla volta, in una conversazione a parte e **in plan mode**, nel momento in cui quel
passo si prende in mano: è lì che le premesse si verificano contro il codice com'è
*adesso*, e non com'era quando la sezione è stata pianificata.

**È il cambiamento dell'8 ottobre 2026**, e sostituisce la regola precedente, che
chiedeva la decomposizione intera — passi, decisioni, alternative e trappole — scritta
in `09` prima di cominciare. Quella regola nasceva da un'osservazione vera, e resta
vera: un piano che vive solo nella conversazione in cui è nato non esiste per la
sessione dopo. Ma pianificare in dettaglio una sezione intera prima di toccarla ha
prodotto premesse sbagliate con regolarità, e lo dice il registro di questo progetto:
la **5-ter**, la **7**, la **9**, la **10**, la **12** e la **13** hanno ciascuna
almeno una premessa smentita dal codice che il piano pretendeva di descrivere — la 7
ne aveva tre, e una stava nella tabella intitolata «verificato e non dedotto».
Riscriverle è costato più di quanto il dettaglio anticipato abbia fatto risparmiare.

**Ciò che deve sopravvivere sopravvive nell'esito, non nel piano.** Quando un passo si
chiude, in `docs/11-piani-conclusi.md` vanno le decisioni **con il perché** e le
alternative scartate, e soprattutto le **trappole**: le vie che sembrano corrette e non
lo sono, che sono la conoscenza più cara da riacquistare, perché per riscoprirne una
bisogna prima sbagliare. Il piano dettagliato di plan mode può morire con la sua chat;
questo no, ed è la condizione che rende accettabile tenerlo fuori dai file.

Ciò che è rinviato si segna come rinviato, con la ragione, altrimenti si rilegge come
lavoro pendente e qualcuno lo prende in mano.

**A esito scritto si pianifica subito la sezione successiva, nello stesso giro di
lavoro.** Chi ha appena chiuso una sezione ha in testa le trappole incontrate e ciò che
il codice fa *adesso*: è materiale che la sessione dopo non avrà. Una sezione non è
conclusa finché la successiva non ha, in `docs/09-stato-e-roadmap.md`, il suo paragrafo
di apertura e i suoi passi. Dal cambiamento dell'8 ottobre costa poco — è un paragrafo e
un elenco, non più una decomposizione — e **le premesse non si verificano qui**: si
verificano in plan mode, passo per passo, che è il punto del cambiamento.

**A esito scritto, il piano si sposta in `docs/11-piani-conclusi.md`, nello stesso
commit.** I due file si dividono per **ciclo di vita** e non per argomento: `09` si
riscrive a ogni fetta, `11` non si riscrive mai — nemmeno per correggere una
previsione che l'esecuzione ha smentito, che è la parte più utile da rileggere.

Lo spostamento sta nello stesso commit dell'esito perché è l'unico momento in cui chi
lavora ha già entrambi i file aperti: una regola di migrazione che chieda un passaggio
a parte non viene rispettata, e dopo due fette i file tornano a essere due archivi
mescolati.

### La review di un agent, quando il lavoro la merita

**Non è sistematica, e non ha una soglia scritta.** Una review costa un agent intero, e
farla su tre stringhe e un ramo nel client è spendere senza comprare niente; non farla
mai su una migration o su un artefatto di deploy è il contrario.

La regola è quindi di procedura e non di contenuto: **a lavoro finito e verificato,
prima di proporre i commit, si dice se quel cambiamento merita una review e perché** —
e chi possiede il progetto decide. Non si lancia un agent senza averlo chiesto.

Il criterio da usare nel proporla, che resta un giudizio e per questo non è scritto come
regola: **quanto costerebbe un difetto che i test non vedono.** Lo meritano le modifiche
importanti e i blocchi critici — una migration o una policy RLS, l'autenticazione e i
permessi, un calcolo di importi o di quote, un artefatto di deploy che non è mai stato
eseguito. Non lo meritano le rifiniture del client, le stringhe del catalogo e la
documentazione.

### Commit

- **Si verifica prima di committare, e i commit si concordano.** Implementa,
  esegui la suite completa, presenta il risultato, *poi* prepara i commit.
- **Conventional Commits.** Il corpo spiega il **perché**, non il cosa: il diff
  dice già cosa è cambiato. Vedi `git log` per il registro esistente.
- **Un commit per unità logica.** Un commit da 4.000 righe che introduce un modulo
  intero non è rivedibile; si divide per livello (dominio, use case, infrastruttura,
  bordo HTTP, test) o per capacità.
- **Creare un branch non richiede permesso; committare sì.** Quando il lavoro è
  pronto, i commit si propongono come **tabella** — una riga per commit, con tipo,
  scope, oggetto e cosa copre — e poi ci si ferma ad aspettare. La tabella è il punto
  di revisione: rimodellare una proposta costa poco, riscrivere una storia già
  pubblicata no.
- **Il push è una conferma a parte.** Il via libera a committare non è il via libera
  a pubblicare, e quello dato per un push non vale per il successivo: si chiede ogni
  volta.

### Pull request

- **Il corpo di una PR è documentazione, non una nota di passaggio.** Deve bastare a
  chi la rilegge fra un anno senza aprire il diff: cosa cambia e perché, quali
  decisioni sono state prese e quali alternative scartate, quali invarianti e vincoli
  vengono introdotti, come è stato verificato, cosa resta fuori. Tre righe buttate là
  non sono accettabili.
- **Le PR non si aprono se non richieste.** Aprirne una è pubblicare: la decisione è
  di chi ha chiesto il lavoro, non di chi lo ha fatto.
- Il file `.github/pull_request_template.md` riporta le sezioni attese. Compilarle
  tutte; se una non si applica, dirlo esplicitamente invece di cancellarla. **Il
  template da solo non vincola nessuno**: `gh pr create --body` lo scavalca senza
  dire niente, e per questo la regola sta scritta anche qui e non solo lì.
- **Riferimenti puntuali**: numeri di migration, nomi di ADR, percorsi di file,
  identificatori di vincoli e policy. Un corpo che dice "sistemata la RLS" non serve
  a nessuno.
- **PR in stack:** la base è il branch padre, non `dev`. Non usare *squash* sul
  padre di uno stack: rompe i figli.
- **Su una PR in stack la CI non gira**, e non è un guasto: `.github/workflows/ci.yml`
  si attiva su `pull_request: branches: [main, dev]`, che filtra il branch di
  **destinazione**, e la base di una PR in stack è il padre. Il cancello vero resta la
  PR del padre verso `dev`, dove la CI gira e dove la protezione del branch esige il
  check `CI`.
  Ne segue un obbligo per chi apre una PR in stack: **la verifica è quella eseguita a
  mano**, e va scritta nel corpo con i numeri — `just ci`, quanti test, quanti falliti,
  quanti dichiarati `SALTATO`. Senza, quella PR non ha nessuna prova, né automatica né
  scritta.
  Allargare il filtro ai branch di lavoro è possibile e si è scelto di non farlo: il
  nome del job `CI` è un contratto con la protezione dei branch, e verificare due volte
  lo stesso albero — una nella PR figlia, una in quella del padre — costa minuti di
  runner per una prova che il padre rifà comunque.

### Altro

- **Nessuna autoreferenza dell'assistente** nei messaggi di commit, nei corpi
  delle PR, nei commenti: né firme, né `Co-Authored-By`, né riferimenti a come il
  cambiamento è stato prodotto o a documenti di pianificazione esterni al
  repository. Un commit si spiega da sé a chi lo leggerà fra un anno.
- **Branch:** `<tipo>/<descrizione-kebab>`, in inglese. `dev` è il branch di
  integrazione, `main` la linea stabile. Mai committare direttamente su `main` o
  `dev`.
- **Non si cancellano branch altrui** senza chiedere, nemmeno quando il contenuto è
  già altrove: chi non ha potuto verificare non ha modo di saperlo.
- **I commenti spiegano il perché.** Un commento che riformula il codice è
  rumore; uno che spiega perché una via apparentemente più semplice non funziona
  vale il suo spazio.
- **La documentazione si aggiorna insieme al codice.** Una decisione nuova o
  cambiata è una voce in `docs/08-decisioni.md`. Per la configurazione non è un
  invito: `crates/api/tests/docs_config_reference.rs` deriva le impostazioni da
  `AppConfig` e dal migratore, e un'impostazione aggiunta senza la riga
  corrispondente in `docs/07-operazioni.md` fa fallire `cargo test`.

---

## 4. Comandi

```bash
# Suite completa: container PostgreSQL effimero, ruoli, migration, invarianti,
# chiavi di firma, tutti i test. È il comando da usare prima di ogni consegna.
./ops/ci/test-integration.sh

# Controlli rapidi senza database
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace                 # i test che richiedono il DB si dichiarano saltati
./ops/ci/check-domain-purity.sh

# Solo le invarianti dello schema, su un database già migrato
./ops/ci/check-schema.sh "$DATABASE_URL"

# Ambiente di sviluppo
docker compose up -d postgres minio minio-setup mailpit   # minio: documenti; mailpit: posta
./ops/dev/generate-jwt-keys.sh          # crea .secrets/, richiede OpenSSL 3
cp .env.example .env                    # poi adattare
set -a; source .env; set +a             # solo se lanci a mano: `just` lo carica da sé
cargo run -p propflow-migrator
cargo run -p propflow-api               # Swagger UI su http://localhost:8080/docs
cargo run -p propflow-worker            # dispatcher dell'outbox

# La posta che parte si legge su http://localhost:8025 (mailpit). Accetta tutto e
# non recapita niente: prova che il messaggio parte e che cosa contiene, **non**
# che sarebbe recapitato.
```

Il client web, che vive in `apps/web/`:

```bash
cd apps/web
npm ci                 # esattamente il lockfile, come `cargo --locked`
npm run dev            # http://localhost:5173, con /api/v1 inoltrato all'API
npm run typecheck      # `strict` acceso: è l'equivalente di «zero warning»
npm run lint
npm test               # fra questi, il refresh a singolo volo riprodotto al contrario
npm run build
```

`npm run dev` presuppone l'API su `http://localhost:8080` e
`PROPFLOW__SERVER__SECURE_COOKIES=false`: su HTTP un browser non manda un cookie
`Secure`, e senza la sessione durerebbe i quindici minuti dell'access token.
`PROPFLOW_API_URL` sposta la destinazione del proxy.

In alternativa `docker compose up` avvia lo stack intero, client compreso, dietro
il reverse proxy su `http://localhost:8081` — che è la stessa forma che avrà in
produzione: `/` ai file statici, `/api/v1` all'API.

`psql` non è necessario in locale: gli script usano il client dell'immagine
`postgres:16-alpine` se non lo trovano.

---

## 5. Mappa del codice

```
crates/domain           entità, value object, eventi, porte di persistenza
crates/application      use case, autorizzazione, porte tecniche
crates/infrastructure   PostgreSQL, Argon2id, EdDSA, telemetria
crates/api        [bin] Axum: router, estrattori, DTO, OpenAPI
crates/worker     [bin] dispatcher dell'outbox
crates/migrator   [bin] migration dello schema
migrations/             SQL append-only, applicato dal migratore
apps/web/               client React + TypeScript, interfaccia in italiano
ops/ci/                 script usati dalla pipeline e in locale
ops/sql/                provisioning dei ruoli, invarianti dello schema
ops/web/                Caddyfile del reverse proxy: una sola origine
docs/                   documentazione di progetto — lettura obbligatoria
```

Dentro `apps/web/src/`:

```
i18n/        il catalogo delle stringhe. Nessun testo visibile fuori da qui
lib/         decimali (mai `parseFloat`), formattazione con `Intl`
api/         client, sessione con il refresh a singolo volo, tipi del filo
app/         guscio: sessione, query, router, navigazione
components/  primitive e i tre stati — caricamento, vuoto, errore
features/    una cartella per modulo: dashboard, immobili, contratti, bollette…
```

Direzione delle dipendenze, unica ammessa:

```
api ──► application ──► domain
 │                        ▲
 └──► infrastructure ─────┘
```

`api` conosce `infrastructure` solo in `main.rs`, per costruire le
implementazioni concrete e iniettarle come `Arc<dyn Port>`.
