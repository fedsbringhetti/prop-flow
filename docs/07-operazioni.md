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
- **`.env` non lo legge nessuno.** Nessun crate dipende da `dotenvy` e
  `docker-compose.yml` dichiara le variabili inline. È un file da dare alla shell —
  `set -a; source .env; set +a` — non una sorgente di configurazione.

### Impostazioni dell'API e del worker

Sono i campi di `AppConfig` (`crates/api/src/config.rs`), lette da figment.

| Variabile | Default | Note |
|---|---|---|
| `PROPFLOW__SERVER__BIND` | `0.0.0.0:8080` | |
| `PROPFLOW__SERVER__REQUEST_TIMEOUT_SECONDS` | `30` | risposta `503` allo scadere |
| `PROPFLOW__SERVER__BODY_LIMIT_BYTES` | `1048576` | limite generale, montato sull'intero router |
| `PROPFLOW__SERVER__UPLOAD_BODY_LIMIT_BYTES` | `26214400` | solo le rotte di caricamento dei documenti |
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
| `PROPFLOW__STORAGE__ENDPOINT` | `http://localhost:9000` | MinIO in sviluppo, `https://s3.<regione>.amazonaws.com` in produzione |
| `PROPFLOW__STORAGE__REGION` | `us-east-1` | |
| `PROPFLOW__STORAGE__BUCKET` | `propflow-documents` | |
| `PROPFLOW__STORAGE__ACCESS_KEY_ID` | — | **obbligatoria** |
| `PROPFLOW__STORAGE__SECRET_ACCESS_KEY` | — | **obbligatoria**, dal gestore dei segreti |
| `PROPFLOW__STORAGE__PATH_STYLE` | `true` | bucket nel percorso; `false` solo con un DNS `<bucket>.<host>` |
| `PROPFLOW__STORAGE__PRESIGN_TTL_SECONDS` | `300` | vita dell'URL di download pre-firmato |
| `PROPFLOW__STORAGE__REQUEST_TIMEOUT_SECONDS` | `30` | per richiesta verso lo storage |
| `PROPFLOW__TELEMETRY__SERVICE_NAME` | `propflow-api` | nome del servizio nelle tracce e negli span |
| `PROPFLOW__TELEMETRY__OTLP_ENDPOINT` | — | assente: solo stdout |
| `PROPFLOW__TELEMETRY__FILTER` | `info,propflow_api=debug,propflow_application=debug` | direttiva `EnvFilter` |
| `PROPFLOW__TELEMETRY__JSON_LOGS` | `false` | `true` in produzione |

Le due impostazioni di limite del corpo sono **due** perché
`RequestBodyLimitLayer` è montato sull'intero router: un limite più largo dichiarato
sulla sola rotta di upload non verrebbe mai raggiunto, perché il layer esterno
rifiuta per primo. Il router monta quindi il limite generale sul gruppo di rotte che
**non** comprende l'upload, e a quella dà il proprio. Alzare il limite generale
funzionerebbe ed è la via da non prendere: indebolisce ogni endpoint per servirne uno.

**Cifratura a riposo.** Il bucket va configurato con la cifratura predefinita (SSE):
l'applicazione non la richiede a ogni oggetto, **legge** dalla risposta del fornitore
se l'oggetto è stato cifrato e registra la modalità in `documents.encryption_key_ref`.
Su un bucket non configurato la colonna `documents.is_encrypted` risulta `false`, ed è
il comportamento voluto — dice il vero invece di dire ciò che si spera. Con SSE la
chiave è del fornitore: non protegge da un accesso amministrativo al bucket.

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
sviluppo lo fa il servizio `minio-setup` di `docker-compose.yml`, in CI
`ops/ci/prepare-object-storage.sh`.

Le tre impostazioni `PROPFLOW__STORAGE__ENDPOINT`, `__ACCESS_KEY_ID` e
`__SECRET_ACCESS_KEY` non hanno un default e sono **obbligatorie**: senza, l'API
non parte e dice `configurazione non valida`. È deliberato — un servizio che entra
in esercizio e fallisce al primo caricamento di un utente è peggio di uno che non
parte.

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

## Sonde

| Endpoint | Uso | Comportamento |
|---|---|---|
| `/health` | liveness | Risponde sempre, non tocca il database. Se lo facesse, un'indisponibilità di PostgreSQL riavvierebbe in loop tutti i pod |
| `/ready` | readiness | Verifica il database. `503` se non raggiungibile: il pod non riceve traffico ma non viene riavviato |

All'avvio l'API verifica il database e **esce** se non risponde: meglio non entrare
mai in servizio che accettare traffico e restituire `500`.

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
| `changes` | Decide se la revisione tocca il codice. Se no, i cinque job che seguono vengono **saltati** |
| `lint` | `cargo fmt --check`, `clippy` con `-D warnings` |
| `architecture` | `check-domain-purity.sh`: il dominio non dipende dall'infrastruttura, nessun SQL negli handler |
| `test` | PostgreSQL 16 come service, ruoli, migrazioni dal binario, invarianti dello schema, idempotenza del migratore, suite completa, e un controllo che nessun test si sia dichiarato saltato |
| `image` | Costruisce l'immagine con cache GHA. Non pubblica: la pubblicazione è una decisione di release, e la fa il workflow di rilascio ([10 — Rilasci](10-rilasci.md#cosa-fa-il-workflow-al-tag)) |
| `dependencies` | `cargo-deny`: vulnerabilità note, crate ritirati, versioni duplicate, provenienza |
| `ci` | Riepilogo. È l'unico da richiedere nella protezione del branch, così aggiungere un job non richiede di riconfigurare GitHub |

Concorrenza: un push che arriva mentre la pipeline precedente sullo stesso ref è in
corso la annulla.

### Le modifiche di sola documentazione non eseguono la pipeline

Una revisione che tocca solo prosa salta `lint`, `architecture`, `test`, `image` e
`dependencies`. Vale sia sulle pull request sia sui push verso `main` e `dev`: se il
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

## Cosa manca

- Manifest Kubernetes: non ci sono ancora.
- Backup e restore del database.
- Metriche (esistono log e tracce, non metriche).
- Alerting.
