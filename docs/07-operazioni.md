# 07 — Operazioni

## Componenti

| Componente | Ruolo database | Funzione |
|---|---|---|
| `propflow-api` | `propflow_app` + `propflow_worker` | servizio HTTP |
| `propflow-worker` | `propflow_worker` | dispatcher dell'outbox |
| `propflow-migrator` | `propflow_migrator` | migrazioni dello schema |

L'API usa due pool: quello applicativo per tutto, quello privilegiato per i flussi
pre-autenticazione. Vedi [04 — Sicurezza](04-sicurezza.md#il-pool-privilegiato).

## Configurazione

`config/default.toml` più variabili d'ambiente con prefisso `PROPFLOW__`, dove il
doppio underscore separa i livelli. L'ambiente vince sul file, così un deploy non
deve riscriverlo. Nessun segreto nel file.

Due avvertenze che nascono da come il file viene caricato, entrambe in
[ADR-023](08-decisioni.md#adr-023):

- **`config/default.toml` si risolve rispetto alla directory di lavoro.** L'API va
  avviata dalla radice del repository, o dall'immagine, dove il file sta in
  `/app/config`. Avviata altrove il file semplicemente non c'è: le quattro
  impostazioni obbligatorie falliscono rumorosamente, tutte le altre ripiegano in
  silenzio sui default del codice, che non sono per forza quelli di questa tabella.
- **`.env` non lo legge nessun processo.** Nessun crate dipende da `dotenvy` e
  `docker-compose.yml` dichiara le variabili inline. Lo carica chi avvia: `just`, che ha
  `set dotenv-load`, o la shell con `set -a; source .env; set +a` per chi lancia un
  binario a mano. Resta un file per l'ambiente, non una sorgente di configurazione.

### Impostazioni dell'API e del worker

Sono i campi di `AppConfig` (`crates/api/src/config.rs`), lette da figment.

| Variabile | Default | Note |
|---|---|---|
| `PROPFLOW__SERVER__BIND` | `0.0.0.0:8080` | |
| `PROPFLOW__SERVER__REQUEST_TIMEOUT_SECONDS` | `30` | risposta `503` allo scadere |
| `PROPFLOW__SERVER__BODY_LIMIT_BYTES` | `1048576` | limite generale, montato sull'intero router |
| `PROPFLOW__SERVER__UPLOAD_BODY_LIMIT_BYTES` | `26214400` | solo le rotte di caricamento dei documenti |
| `PROPFLOW__SERVER__SECURE_COOKIES` | `true` | `Secure` sul cookie del refresh token. `false` **solo** in sviluppo su `http://localhost` |
| `PROPFLOW__DATABASE__APPLICATION_URL` | — | **obbligatoria**, ruolo `propflow_app` |
| `PROPFLOW__DATABASE__PRIVILEGED_URL` | — | **obbligatoria**, ruolo `propflow_worker` |
| `PROPFLOW__DATABASE__MAX_CONNECTIONS` | `16` | per pool |
| `PROPFLOW__DATABASE__STATEMENT_TIMEOUT_SECONDS` | `10` | per connessione |
| `PROPFLOW__AUTH__PRIVATE_KEY_PATH` | — | **obbligatoria**, PEM PKCS#8 Ed25519 |
| `PROPFLOW__AUTH__PUBLIC_KEY_PATH` | — | **obbligatoria**, PEM SPKI |
| `PROPFLOW__AUTH__ISSUER` | `propflow` | |
| `PROPFLOW__AUTH__AUDIENCE` | `propflow-api` | |
| `PROPFLOW__AUTH__ACCESS_TOKEN_TTL_SECONDS` | `900` | |
| `PROPFLOW__AUTH__REFRESH_TOKEN_TTL_DAYS` | `30` | |
| `PROPFLOW__STORAGE__ENDPOINT` | `http://localhost:9000` | SeaweedFS del compose in sviluppo, `http://storage:8333` nel compose di produzione ([ADR-057](08-decisioni.md#adr-057)); qualunque S3 compatibile |
| `PROPFLOW__STORAGE__REGION` | `us-east-1` | |
| `PROPFLOW__STORAGE__BUCKET` | `propflow-documents` | |
| `PROPFLOW__STORAGE__ACCESS_KEY_ID` | — | **obbligatoria** |
| `PROPFLOW__STORAGE__SECRET_ACCESS_KEY` | — | **obbligatoria**, dal gestore dei segreti |
| `PROPFLOW__STORAGE__PATH_STYLE` | `true` | bucket nel percorso; `false` solo con un DNS `<bucket>.<host>` |
| `PROPFLOW__STORAGE__PRESIGN_TTL_SECONDS` | `300` | vita dell'URL di download pre-firmato |
| `PROPFLOW__STORAGE__REQUEST_TIMEOUT_SECONDS` | `30` | per richiesta verso lo storage |
| `PROPFLOW__MAIL__HOST` | — | **obbligatoria**, host del relay SMTP |
| `PROPFLOW__MAIL__PORT` | `587` | `1025` per il raccoglitore locale, `465` con TLS implicito |
| `PROPFLOW__MAIL__USERNAME` | — | assente sul raccoglitore locale; con la password, o nessuna delle due |
| `PROPFLOW__MAIL__PASSWORD` | — | dal gestore dei segreti |
| `PROPFLOW__MAIL__FROM_ADDRESS` | — | **obbligatoria**, il mittente. Su un dominio senza SPF, DKIM e DMARC finisce nello spam senza dare errore |
| `PROPFLOW__MAIL__FROM_NAME` | `PropFlow` | il nome accanto all'indirizzo |
| `PROPFLOW__MAIL__ENCRYPTION` | `starttls` | `tls` per la 465, `none` **solo** per il raccoglitore locale |
| `PROPFLOW__MAIL__TIMEOUT_SECONDS` | `10` | per invio verso il relay |
| `PROPFLOW__TELEMETRY__SERVICE_NAME` | `propflow-api` | nome del servizio nelle tracce e negli span |
| `PROPFLOW__TELEMETRY__OTLP_ENDPOINT` | — | assente: solo stdout |
| `PROPFLOW__TELEMETRY__FILTER` | `info,propflow_api=debug,propflow_application=debug` | direttiva `EnvFilter` |
| `PROPFLOW__TELEMETRY__JSON_LOGS` | `false` | `true` in produzione |

**Un'impostazione obbligatoria che manca ferma l'avvio, e lo dice.** `AppConfig::load`
distingue un'impostazione **mancante** da una **sbagliata**: sulla prima il messaggio
ricorda il prefisso `PROPFLOW__`, rimanda a `.env.example` per l'elenco completo, e
ricorda che quel file il processo non lo carica da solo — lo fa `just`, o la shell con
`set -a; source .env; set +a` per chi lancia il binario a mano. Il messaggio di figment,
che dice quale campo e sotto quale chiave, resta nella catena delle cause.

**Le due obbligatorie della posta sono `HOST` e `FROM_ADDRESS`**, e non hanno un default
perché non ne esiste uno sensato. La ragione è più stringente di quella dello storage: da
quando esiste il recupero della password la posta è l'**unico canale** verso una persona
chiusa fuori dal proprio account, e un'API che parte senza saper spedire risponderebbe
`202` a una richiesta di recupero senza mandare niente — su chi non ha nessun altro modo
di rientrare.

**La configurazione ferma un relay mal configurato, non un relay caduto.** Quello lo dice
`/ready`, che interroga SMTP insieme al database: l'invio è best-effort dopo il commit,
quindi un relay fermo non produce nessun errore visibile a nessuno. Dirlo al chiamante non
si può — si tenta l'invio solo per un indirizzo che ha un account, quindi qualunque cosa
distingua i due rami sarebbe un oracolo sugli account — e il controllo di prontezza è il
rimedio: rende la posta caduta nota **prima** che qualcuno la incontri. Prova che il
server risponde, **non** che il messaggio verrebbe recapitato.

Non c'è un default per le credenziali dello storage, ed è deliberato: uno storage
configurato a caso sposterebbe il fallimento al primo caricamento di un documento, dove
somiglierebbe a un problema di rete invece che a una configurazione incompleta. Il caso
tipico è **un `.env` copiato prima della fetta dei documenti**, che non ha le quattro
`PROPFLOW__STORAGE__*`: si confronta con `.env.example`.

Le due impostazioni di limite del corpo sono **due** perché
`RequestBodyLimitLayer` è montato sull'intero router: un limite più largo dichiarato
sulla sola rotta di upload non verrebbe mai raggiunto, perché il layer esterno
rifiuta per primo. Il router monta quindi il limite generale sul gruppo di rotte che
**non** comprende l'upload, e a quella dà il proprio. Alzare il limite generale
funzionerebbe ed è la via da non prendere: indebolisce ogni endpoint per servirne uno.

**Cifratura a riposo.** Su uno S3 gestito, o su qualunque fornitore che dichiari la
cifratura sulla `PUT`, il bucket va configurato con la cifratura predefinita (SSE):
l'applicazione non la richiede a ogni oggetto, **legge** dalla risposta del fornitore
se l'oggetto è stato cifrato e registra la modalità in `documents.encryption_key_ref`.
**Sullo storage che il progetto spedisce oggi questo passo non c'è**, e il paragrafo
dopo dice perché: non esiste un comando da eseguire né in `storage-setup` né nel
runbook, perché non cambierebbe niente di osservabile.
Su un bucket non configurato la colonna `documents.is_encrypted` risulta `false`, ed è
il comportamento voluto — dice il vero invece di dire ciò che si spera. Con SSE la
chiave è del fornitore: non protegge da un accesso amministrativo al bucket.

**Su SeaweedFS l'header non torna** ([ADR-057](08-decisioni.md#adr-057)): accetta la
cifratura predefinita sul bucket e non la dichiara sulla `PUT`, come MinIO senza un KMS.
`is_encrypted` resta quindi `false` anche configurandola, che è il vero dal punto di vista
dell'applicazione. La cifratura a riposo dei documenti è oggi **quella del disco** della
macchina — da attivare sul volume della VPS — e la colonna non lo può sapere.

La colonna dei default riporta ciò che il progetto **spedisce**, cioè i valori di
`config/default.toml`. Dove il file non viene trovato valgono i default del codice,
che possono essere più conservativi: il filtro di log, per esempio, ripiega su
`info,propflow_api=debug`.

Le due tabelle di questa sezione sono verificate da un test
(`crates/api/tests/docs_config_reference.rs`), che deriva l'elenco dal sorgente:
un'impostazione nuova non documentata, o una riga qui che nel codice non esiste più,
fanno fallire la suite.

### Impostazioni del migratore

Non passano da `AppConfig`: `crates/migrator/src/main.rs` le legge direttamente con
`std::env::var`. Il migratore è un binario di deploy che gira come un altro ruolo e
non ha bisogno del resto della configurazione — pretendere l'intero `AppConfig` gli
chiederebbe le URL dei pool applicativi e i percorsi delle chiavi di firma, che non
usa.

| Variabile | Default | Note |
|---|---|---|
| `PROPFLOW__DATABASE__MIGRATOR_URL` | — | **obbligatoria**, ruolo `propflow_migrator` |
| `PROPFLOW__DATABASE__MIGRATIONS_DIR` | `migrations` | nell'immagine `/app/migrations` |

Le chiavi sono **percorsi**, non contenuti: una chiave privata in una variabile
d'ambiente è leggibile da `/proc`, dai dump di crash e da `docker inspect`.

### Impostazioni del worker

Stessa scelta del migratore e stessa ragione: `crates/worker/src/main.rs` le legge con
`std::env::var`. Il worker non ha un bind né chiavi di firma, e pretendere l'intero
`AppConfig` gliele chiederebbe.

| Variabile | Default | Note |
|---|---|---|
| `PROPFLOW__DATABASE__PRIVILEGED_URL` | — | **obbligatoria**, ruolo `propflow_worker` (`BYPASSRLS`) |
| `PROPFLOW__STORAGE__ENDPOINT` | — | **obbligatoria** dalla fetta dell'OCR |
| `PROPFLOW__STORAGE__ACCESS_KEY_ID` | — | **obbligatoria** |
| `PROPFLOW__STORAGE__SECRET_ACCESS_KEY` | — | **obbligatoria** |
| `PROPFLOW__STORAGE__REGION` | `us-east-1` | |
| `PROPFLOW__STORAGE__BUCKET` | `propflow-documents` | |
| `PROPFLOW__STORAGE__PATH_STYLE` | `true` | |
| `PROPFLOW__STORAGE__REQUEST_TIMEOUT_SECONDS` | `30` | |
| `RUST_LOG` | `info` | il worker non ha una sezione `telemetry` |

**Le tre obbligatorie dello storage sono nuove**, e vanno aggiunte al deployment del
worker *prima* di rilasciare l'OCR: senza, il worker non parte. È deliberato — fallisce
all'avvio invece di accumulare job `failed` che sembrano colpa del fornitore di lettura.

**Devono puntare allo stesso bucket dell'API.** Il worker legge gli oggetti che l'API ha
scritto: due configurazioni che divergono danno il sintomo peggiore possibile, cioè job
che falliscono `document_unavailable` su documenti che l'API mostra benissimo.

Il worker esegue **due cicli in un processo solo** — il dispatcher dell'outbox e la
lettura automatica delle bollette — e se uno dei due esce, il processo esce: un worker
vivo che fa metà del lavoro non verrebbe riavviato da nessun orchestratore. Non ci sono
impostazioni per la frequenza dei cicli: sono costanti nel binario, e diventeranno
configurazione quando ci sarà una ragione per cambiarle in esercizio.

## Immagine

Multi-stage: `rust:1.95-slim-bookworm` per la build, `debian:bookworm-slim` per il
runtime. Circa 196 MB, tre binari, utente non privilegiato `uid 10001`.

Le dipendenze si compilano prima del codice applicativo, così una modifica a un
file `.rs` non invalida la compilazione dell'intero albero. La build è **offline**:
gli asset di Swagger UI arrivano dal crate `vendored` invece di essere scaricati.

La cartella `migrations/` è nell'immagine perché il migratore le legge a runtime.

```bash
docker build --target runtime -t propflow:dev .
```

## Prerequisiti del rollout

Due cose devono esistere **prima** che l'API parta, e nessuna delle due la crea
l'applicazione.

**Lo schema**, applicato dal migratore: sotto, in [Migrazioni in
produzione](#migrazioni-in-produzione).

**Il bucket dello storage oggetti.** `ObjectStorage` non espone una creazione e non
deve esporla: darla all'applicazione significherebbe darle in produzione il
privilegio di creare bucket, che un servizio HTTP non deve avere
([ADR-024](08-decisioni.md#adr-024)). Il bucket si crea con il provisioning — in
sviluppo e in produzione lo fa il servizio `storage-setup` dei due compose, in CI
`ops/ci/prepare-object-storage.sh`. In produzione **prima non lo faceva nessuno**: il
runbook faceva `up -d`, e il bucket mancante dava il sintomo muto descritto sotto.
`storage-setup` è un one-shot da cui API e worker dipendono, e al secondo avvio
`BucketAlreadyOwnedByYou` non è un errore.

Le tre impostazioni `PROPFLOW__STORAGE__ENDPOINT`, `__ACCESS_KEY_ID` e
`__SECRET_ACCESS_KEY` non hanno un default e sono **obbligatorie**: senza, l'API
non parte e dice `configurazione non valida`. È deliberato — un servizio che entra
in esercizio e fallisce al primo caricamento di un utente è peggio di uno che non
parte. **Dalla fetta dell'OCR valgono anche per il worker**, che legge le scansioni
dallo stesso bucket: vedi [Impostazioni del worker](#impostazioni-del-worker).

Il fallimento che invece **non** è rumoroso è il bucket che non esiste o non è
raggiungibile: `S3ObjectStorage::new` valida endpoint e nome, non l'esistenza
remota. Verificato puntando l'API a un bucket inesistente: l'avvio riesce, `/ready`
risponde `200`, e il primo `POST .../documents` risponde `500 internal.error`. Il
motivo resta nei log come `richiesta terminata con errore`, correlato dal
`request_id` della risposta — nel corpo non c'è, perché un messaggio del fornitore
di storage in una risposta HTTP è ricognizione gratuita.

Ne segue che **il caricamento di un documento va nel giro di collaudo**: è l'unica
operazione che tocca lo storage, e nessuna sonda la copre. `/ready` interroga solo
il database (`pools.ping()`), perché è stata scritta quando lo storage non
esisteva; estenderla non è ovvio e per ora resta com'è, dato che una sonda che
fallisse per il bucket toglierebbe dal servizio un'istanza che continua a servire
correttamente ogni altro endpoint. Il collaudo copre quindi ciò che la sonda non
copre.

Se il bucket non ha la cifratura predefinita, nulla fallisce e
`documents.is_encrypted` resta `false`: vedi la nota in
[Configurazione](#configurazione).

## Migrazioni in produzione

Job separato, **prima** del rollout. In Kubernetes un `Job` o un initContainer:

```yaml
initContainers:
  - name: migrate
    image: propflow:<tag>
    command: ["propflow-migrator"]
    env:
      - name: PROPFLOW__DATABASE__MIGRATOR_URL
        valueFrom: { secretKeyRef: { name: propflow-db, key: migrator-url } }
      - name: PROPFLOW__DATABASE__MIGRATIONS_DIR
        value: /app/migrations
```

Perché non dall'API: si connette come `propflow_app`, che non ha privilegi DDL, e
in un deploy multi-replica partirebbero N migrazioni concorrenti. Il migratore usa
un advisory lock, quindi più esecuzioni contemporanee si serializzano.

È **idempotente**, verificato in CI: una seconda esecuzione su un database già
migrato non riapplica nulla.

### Manutenzione periodica

`audit_log` è partizionata per mese. Un cron mensile deve creare la partizione
successiva:

```sql
SELECT app.ensure_audit_partition((CURRENT_DATE + INTERVAL '1 month')::date);
```

La funzione è idempotente e revoca la scrittura sulle nuove partizioni. Esiste una
partizione `DEFAULT`, quindi un cron mancato non causa errori di inserimento — solo
una partizione che cresce.

### Emettere un invito da proprietario

ADR-053: chi diventa proprietario non si registra da una rotta aperta a
chiunque, ma da un invito emesso qui, dietro un token. Come si verifichi che
una persona sia un proprietario vero — un'email, una telefonata, un documento —
è una decisione di chi amministra la piattaforma e non vive nel prodotto: nel
prodotto vive solo il fatto che il token esiste.

**Emissione**, con il ruolo del migratore, come per `password_reset_codes`:

```bash
set -a; source .env; set +a   # carica PROPFLOW__DATABASE__MIGRATOR_URL
./ops/dev/issue-owner-invitation.sh mario.rossi@example.it
# ./ops/dev/issue-owner-invitation.sh mario.rossi@example.it 3   # scadenza a 3 giorni
```

Stampa il token in chiaro **una volta sola**, sull'ultima riga di stdout: non si
rilegge più da nessuna parte, perché in database ne resta solo lo hash SHA-256
(la stessa forma di ogni credenziale opaca del progetto). Va consegnato fuori
banda — non per email non cifrata, che il perimetro di questa decisione lascia
esplicitamente fuori — a chi accetterà l'invito su `POST
/api/v1/auth/accept-owner-invitation`.

**Consegna.** Chi lo riceve incolla il token in `/invito` — la stessa porta
dell'inquilino: il form chiede nome e password, e il nome dell'organizzazione lo
chiede solo dopo che il server ha risposto `409 invitation.owner_token`, cioè solo a
chi possiede un token da proprietario valido. Nessun link, per la stessa ragione del codice di recupero della
password e dell'invito all'inquilino ([ADR-045](08-decisioni.md#adr-045),
[ADR-052](08-decisioni.md#adr-052)) — un token in un URL finisce nella
cronologia, nel `Referer` e nei log di ogni proxy davanti al client.

**Revoca**, con il ruolo del migratore:

```sql
UPDATE app.owner_invitations
   SET revoked_at = now()
 WHERE email = 'mario.rossi@example.it'
   AND accepted_at IS NULL
   AND revoked_at IS NULL;
```

**Potatura degli scaduti**, periodica e non obbligatoria: una riga scaduta,
accettata o revocata resta innocua — il ruolo applicativo non ha comunque il
privilegio `DELETE` ([ADR-003](08-decisioni.md#adr-003)) — e la cancellazione
è un'operazione del runbook, con il ruolo del migratore, non un trigger che
nessuno guarda:

```sql
DELETE FROM app.owner_invitations
 WHERE expires_at < now() - INTERVAL '30 days'
   AND accepted_at IS NULL;
```

## Sonde

| Endpoint | Uso | Comportamento |
|---|---|---|
| `/health`, `/api/v1/health` | liveness | Risponde sempre, non tocca il database. Se lo facesse, un'indisponibilità di PostgreSQL riavvierebbe in loop tutti i pod |
| `/ready`, `/api/v1/ready` | readiness | Verifica il database **e la posta**. `503` se una delle due non risponde: il pod non riceve traffico ma non viene riavviato |

Le due forme sono **equivalenti** — stesso handler, stesso corpo — ed esistono entrambe
perché hanno due chiamanti diversi. La radice è per l'`healthcheck` del container, che
parla all'API direttamente. Il prefisso è per chiunque passi dal reverse proxy, cioè dal
dominio: il proxy inoltra all'API solo `/api/v1/*`, e `https://<dominio>/ready` non
arriverebbe mai all'API — risponderebbe il file server, con `index.html` e `200`, una
sonda che «riesce» senza aver provato niente. Il [runbook](#il-primo-deploy--runbook) usa
la forma con il prefisso per questo.

All'avvio l'API verifica il database e **esce** se non risponde: meglio non entrare
mai in servizio che accettare traffico e restituire `500`.

**La posta sta in `/ready` e non nell'avvio**, e la differenza è deliberata. La
*configurazione* della posta è obbligatoria e ferma l'avvio, perché un'API che non sa
spedire non deve entrare in servizio. Il *relay*, invece, può cadere in qualunque
momento dopo, e un'API che uscisse a ogni disservizio SMTP smetterebbe di servire tutto
il resto — contratti, addebiti, segnalazioni — per una funzione che serve a chi non
riesce a entrare.

Ma un relay caduto **non produce nessun errore visibile**: l'invio è best-effort dopo il
commit, e la richiesta di recupero risponde `202` a chiunque per non fare da oracolo
sugli account. Se non lo dicesse `/ready`, lo scoprirebbe una persona chiusa fuori dal
proprio account. Il controllo prova che il relay risponde, **non** che il messaggio
verrebbe recapitato: quello lo dicono SPF, DKIM e DMARC.

Lo shutdown ascolta `SIGTERM` oltre a `SIGINT` — `SIGTERM` è quello che manda
Kubernetes, e ignorarlo significa farsi terminare a forza dopo il grace period
interrompendo le richieste in volo. Le connessioni vengono chiuse ordinatamente.

## Osservabilità

- Log strutturati, JSON con `PROPFLOW__TELEMETRY__JSON_LOGS=true`.
- Tracce OTLP su gRPC quando è configurato un endpoint. L'handle di telemetria
  svuota la coda al `Drop`: perderla allo shutdown significa non avere le tracce
  degli errori che hanno causato lo shutdown.
- **Una riga di log per richiesta**, a livello `INFO`, con metodo, URI, stato,
  latenza e `request_id`. Lo span è costruito a mano in `crates/api/src/lib.rs`:
  `DefaultMakeSpan` di `TraceLayer` registra metodo, URI e versione ma non l'id, e
  la risposta la logga a `DEBUG`, che con il filtro spedito dal progetto non
  produce **alcuna** riga per nessuna richiesta. Un test unitario fallisce se il
  campo `request_id` sparisce dallo span.
- `X-Request-Id` è propagato in risposta e compare nel corpo di ogni errore: un
  utente che segnala un problema porta con sé la chiave di ricerca. Se il client
  non lo fornisce ne viene generato uno.
- `ai_jobs` registra token, costo e latenza per ogni chiamata a un provider AI, con
  `trace_id` per correlare allo span.
- PostgreSQL con `log_min_duration_statement=200` e `log_statement=none`: si vedono
  le query lente senza registrare i parametri, che contengono dati personali.

## Pipeline

`.github/workflows/ci.yml`, su push e pull request verso `main` e `dev`.

| Job | Cosa fa |
|---|---|
| `changes` | Decide se la revisione tocca il **backend**, il **client web**, o nessuno dei due. Chi non è toccato viene **saltato** |
| `lint` | `cargo fmt --check`, `clippy` con `-D warnings` |
| `architecture` | `check-domain-purity.sh`: il dominio non dipende dall'infrastruttura, nessun SQL negli handler |
| `test` | PostgreSQL 16 come service, ruoli, migrazioni dal binario, invarianti dello schema, idempotenza del migratore, suite completa, e un controllo che nessun test si sia dichiarato saltato |
| `web` | Il client in `apps/web/`: `npm ci`, typecheck, lint, test, build |
| `image` | Costruisce l'immagine con cache GHA. Gira **anche per una modifica al solo client**, che l'immagine contiene. Non pubblica: la pubblicazione è una decisione di release, e la fa il workflow di rilascio ([10 — Rilasci](10-rilasci.md#cosa-fa-il-workflow-al-tag)) |
| `dependencies` | `cargo-deny`: vulnerabilità note, crate ritirati, versioni duplicate, provenienza |
| `ci` | Riepilogo. È l'unico da richiedere nella protezione del branch, così aggiungere un job non richiede di riconfigurare GitHub |

Concorrenza: un push che arriva mentre la pipeline precedente sullo stesso ref è in
corso la annulla.

### Le modifiche di sola documentazione non eseguono la pipeline, e il client si verifica da sé

Una revisione che tocca solo prosa salta `lint`, `architecture`, `test`, `web`,
`image` e `dependencies`. Vale sia sulle pull request sia sui push verso `main` e `dev`: se il
merge non porta codice nuovo, dopo il merge non c'è niente di nuovo da verificare.

**Il salto si fa con un `if:` sui job, non con `paths-ignore` sul trigger**, ed è la
cosa da sapere prima di «semplificare». `paths-ignore` impedisce al workflow di
partire: il check richiesto `CI` non verrebbe mai riportato, e la protezione del branch
terrebbe la pull request bloccata su *«Expected — waiting for status»* finché qualcuno
non tocca un file di codice. Saltando i job invece questi restano in stato `skipped`,
che il riepilogo tratta come successo — la sua condizione guarda `failure` e
`cancelled`, non `skipped`.

Conta come documentazione: tutto sotto `docs/`, ogni file `.md`, e `LICENSE`,
`.gitignore`, `.gitattributes`, `.editorconfig`.

**Il job `changes` ha due uscite e non una.** `apps/web/*` accende soltanto `web`,
e i percorsi Rust — `crates/`, `migrations/`, `ops/`, `config/`, `tests/`, i
manifest — soltanto `code`. Senza la distinzione un cambio di CSS pagherebbe sette
minuti di test su PostgreSQL, e con il tempo si smette di aspettare la pipeline:
che è il modo in cui una pipeline diventa inutile. Tutto il resto — un Dockerfile,
il compose, il workflow stesso — accende **entrambe**, perché l'immagine contiene
tutti e due e un elenco che sbaglia in difetto manda su `dev` codice che nessuno ha
verificato.

**`docs/07-operazioni.md` — questo documento — è l'eccezione, e non è pedanteria:**
`crates/api/tests/docs_config_reference.rs` deriva le impostazioni da `AppConfig` e dal
migratore e **fa fallire `cargo test`** se manca la riga corrispondente nelle tabelle
qui sopra. È l'unico file di sola prosa che può rompere la suite, quindi è l'unico che
non si salta. Modificarlo esegue la pipeline intera.

Nel dubbio si esegue tutto: un intervallo di commit che non si riesce a calcolare, un
primo push su un branch, un evento non previsto e la chiamata da `release.yml` su un
tag danno tutti «c'è codice». L'asimmetria è voluta — un salto sbagliato manda su `dev`
codice che nessuno ha verificato, un'esecuzione di troppo costa minuti.

La preparazione del database è condivisa fra CI e locale
(`ops/ci/prepare-database.sh`): due procedure separate divergono al primo
cambiamento, e poi "in locale funziona".

## Branch e rilasci

- `dev` è il branch di integrazione, `main` la linea stabile. Nessun commit diretto
  su entrambi.
- PR in stack: la base è il branch padre. **Mai squash sul padre di uno stack**:
  sostituisce i commit che i figli contengono e li costringe a un rebase manuale.
- Merge commit o rebase, così la ripartizione in commit logici sopravvive.

## Il primo deploy — runbook

**Niente di ciò che segue è stato eseguito.** Gli artefatti esistono e sono validi
(`docker compose config` passa, gli script passano `bash -n`); la macchina, il dominio e
le credenziali no. Un runbook non provato è un elenco di intenzioni: la parte in fondo
dice come si smette di esserlo.

### 0. Che cosa serve prima di cominciare

- una **VPS** con Docker e il plugin `compose`, e le porte **80 e 443** aperte — la 80
  serve alla convalida del certificato anche se poi tutto va in HTTPS;
- un **dominio** il cui record `A` punta a quella macchina. Va fatto *prima*: Caddy
  chiede il certificato alla prima richiesta, e con il DNS non ancora propagato la
  richiesta fallisce e Let's Encrypt ha un limite che si tocca in fretta;
- un **relay SMTP** con utenza e password, e sul dominio **SPF, DKIM e DMARC**. Senza,
  la posta parte e finisce nello spam, e il sintomo è muto: nessun errore, il codice non
  arriva;
- `gpg` sulla macchina, per cifrare le copie di sicurezza.

### 1. Il codice e i segreti

```bash
git clone <repo> /srv/propflow && cd /srv/propflow
./ops/dev/generate-jwt-keys.sh /srv/propflow/.secrets   # richiede OpenSSL 3
chmod 700 /srv/propflow/.secrets
```

Poi `/srv/propflow/.env.prod`, che **non** sta nel repository. Le variabili che il
compose di produzione pretende sono dichiarate con `:?` in
`ops/deploy/docker-compose.prod.yml`: se ne manca una, `up` si ferma dicendo quale.

```
POSTGRES_SUPERUSER_PASSWORD=…
STORAGE_ACCESS_KEY=…         STORAGE_SECRET_KEY=…
PROPFLOW__DATABASE__MIGRATOR_URL=postgres://propflow_migrator:…@postgres:5432/propflow
PROPFLOW__DATABASE__APPLICATION_URL=postgres://propflow_app:…@postgres:5432/propflow
PROPFLOW__DATABASE__PRIVILEGED_URL=postgres://propflow_worker:…@postgres:5432/propflow
PROPFLOW__MAIL__HOST=…       PROPFLOW__MAIL__USERNAME=…       PROPFLOW__MAIL__PASSWORD=…
PROPFLOW__MAIL__FROM_ADDRESS=propflow@iltuodominio
PROPFLOW_DOMAIN=iltuodominio
PROPFLOW_SECRETS_DIR=/srv/propflow/.secrets
```

`chmod 600 .env.prod`. **`PROPFLOW__SERVER__SECURE_COOKIES` non si dichiara**: il
default è `true` ed è quello che serve su HTTPS. Ogni posto in cui compare è un posto da
cui qualcuno può metterlo a `false` per far funzionare una prova.

### 2. Le immagini e i ruoli del database

```bash
docker build -t propflow:latest .
docker build -t propflow-web:latest --target web .

docker compose -f ops/deploy/docker-compose.prod.yml --env-file .env.prod up -d postgres
docker compose -f ops/deploy/docker-compose.prod.yml --env-file .env.prod exec -T postgres   psql -U postgres -d postgres -v db_name=propflow     -v migrator_password=… -v app_password=… -v worker_password=…     -f - < ops/sql/roles.sql
```

I ruoli si creano **una volta per cluster** e stanno fuori dalle migration: il migratore
si connette con un ruolo che deve già esistere.

### 3. Su

```bash
docker compose -f ops/deploy/docker-compose.prod.yml --env-file .env.prod up -d
docker compose -f ops/deploy/docker-compose.prod.yml --env-file .env.prod logs -f migrate
```

`migrate` è un job che deve uscire `0`. L'API parte solo dopo.

### 4. Quando è deployato

**Non quando i container sono su**: quando succede questo, in quest'ordine.

1. `curl https://<dominio>/api/v1/ready` risponde `200`. Se dice
   `posta non raggiungibile`, il relay è sbagliato o irraggiungibile — e va sistemato
   adesso, perché dopo il sintomo diventa muto;
2. si emette il **primo invito da proprietario** ([§ sopra](#emettere-un-invito-da-proprietario)):
   dopo ADR-053 non c'è una registrazione aperta, e un database appena migrato non
   ha nessun account con cui entrare. Poi si apre il prodotto **dal telefono, in 4G**
   — non dalla stessa rete della macchina —, si incolla il token in `/invito` e si
   fa l'accesso. È il passo che trova i certificati a metà, i record DNS
   propagati solo in locale e i firewall aperti alla rete sbagliata;
3. si chiede un **recupero della password** con un indirizzo vero e il codice arriva
   **fuori dalla cartella dello spam**. Il raccoglitore locale non lo prova: accetta
   tutto e non verifica SPF, DKIM né i rimbalzi;
4. `https://<dominio>/docs` risponde `404`. In produzione Swagger UI non si espone.

Finché il punto 2 non è successo, non è deployato.

### 5. Il backup, e il ripristino provato

```bash
# Una volta a mano, per vedere che cosa produce.
PROPFLOW_BACKUP_PASSPHRASE=… PROPFLOW_BACKUP_REMOTE=… ops/deploy/backup.sh

# Poi da cron, di notte.
0 3 * * * PROPFLOW_BACKUP_PASSPHRASE=… PROPFLOW_BACKUP_REMOTE=…   /srv/propflow/ops/deploy/backup.sh >> /var/log/propflow-backup.log 2>&1
```

Due cose e non una: il **database e il bucket**. I documenti sono oggetti nello storage, non
righe — un dump del database li lascia fuori, e la riga che li cita resta senza byte. E
la copia **esce dalla macchina**: un backup sullo stesso disco del database è una seconda
copia dello stesso guasto.

**Poi il ripristino, una volta, prima che dentro ci siano i dati di qualcun altro**, su
una macchina di prova e su un database vuoto:

```bash
PROPFLOW_BACKUP_PASSPHRASE=… ops/deploy/restore.sh /var/backups/propflow/propflow-….tar.gz.gpg
```

E la parte che conta, che il codice di uscita dello script non dice: si avvia lo stack, si
entra con un account che **esisteva prima del dump**, si apre un contratto e si **scarica
un documento**. Il terzo punto è la metà che un dump del solo database non porta, ed è
quella che si scopre mancante mesi dopo. **Finché non è successo non c'è un backup, c'è
un file.**

### 6. La privacy

`https://<dominio>/privacy` esiste e si legge **senza sessione**, perché la si deve poter
leggere prima di consegnare i propri dati. Il contatto del titolare arriva da
`VITE_PRIVACY_CONTACT` a build time: se la pagina mostra il nome della variabile invece di
un indirizzo, è quella che manca.

La cancellazione dichiarata è una **richiesta scritta con risposta entro trenta giorni**,
non un bottone, e la pagina lo dice. Non è una promessa al ribasso: il ruolo applicativo
non ha il privilegio `DELETE`, tutto passa da `deleted_at`, e `audit_log` e gli eventi
continuano a citare le righe. La capacità vera è
[`R37`](12-da-fare.md#cancellare-in-generale).

## Cosa manca

- Manifest Kubernetes: non ci sono ancora, e restano fuori finché la scala è una VPS.
- **Il runbook qui sopra non è mai stato eseguito**: gli artefatti sono validi, la
  macchina non esiste.
- Metriche (esistono log e tracce, non metriche).
- Alerting — compreso quello sul `/ready` che dice `posta non raggiungibile`, che oggi
  lo vede solo chi guarda.
- La potatura dei codici di recupero scaduti: restano in tabella, inerti. Il ruolo
  applicativo non ha `DELETE`, quindi è un'operazione del runbook con il ruolo del
  migratore, non un trigger.
