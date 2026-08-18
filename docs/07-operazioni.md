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
| `PROPFLOW__SERVER__BODY_LIMIT_BYTES` | `1048576` | |
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
| `PROPFLOW__TELEMETRY__SERVICE_NAME` | `propflow-api` | nome del servizio nelle tracce e negli span |
| `PROPFLOW__TELEMETRY__OTLP_ENDPOINT` | — | assente: solo stdout |
| `PROPFLOW__TELEMETRY__FILTER` | `info,propflow_api=debug,propflow_application=debug` | direttiva `EnvFilter` |
| `PROPFLOW__TELEMETRY__JSON_LOGS` | `false` | `true` in produzione |

La colonna dei default riporta ciò che il progetto **spedisce**, cioè i valori di
`config/default.toml`. Dove il file non viene trovato valgono i default del codice,
che possono essere più conservativi: il filtro di log, per esempio, ripiega su
`info,propflow_api=debug`.

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
- `X-Request-Id` propagato e presente in ogni riga di log della richiesta, e nel
  corpo di ogni errore: un utente che segnala un problema porta con sé la chiave di
  ricerca.
- `ai_jobs` registra token, costo e latenza per ogni chiamata a un provider AI, con
  `trace_id` per correlare allo span.
- PostgreSQL con `log_min_duration_statement=200` e `log_statement=none`: si vedono
  le query lente senza registrare i parametri, che contengono dati personali.

## Pipeline

`.github/workflows/ci.yml`, su push e pull request verso `main` e `dev`.

| Job | Cosa fa |
|---|---|
| `lint` | `cargo fmt --check`, `clippy` con `-D warnings` |
| `architecture` | `check-domain-purity.sh`: il dominio non dipende dall'infrastruttura, nessun SQL negli handler |
| `test` | PostgreSQL 16 come service, ruoli, migrazioni dal binario, invarianti dello schema, idempotenza del migratore, suite completa, e un controllo che nessun test si sia dichiarato saltato |
| `image` | Costruisce l'immagine con cache GHA. Non pubblica: la pubblicazione è una decisione di release, e la fa il workflow di rilascio ([10 — Rilasci](10-rilasci.md#cosa-fa-il-workflow-al-tag)) |
| `dependencies` | `cargo-deny`: vulnerabilità note, crate ritirati, versioni duplicate, provenienza |
| `ci` | Riepilogo. È l'unico da richiedere nella protezione del branch, così aggiungere un job non richiede di riconfigurare GitHub |

Concorrenza: un push che arriva mentre la pipeline precedente sullo stesso ref è in
corso la annulla.

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
