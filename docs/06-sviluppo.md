# 06 — Sviluppo

## Requisiti

| Strumento | Versione | Note |
|---|---|---|
| Rust | 1.95.0 | fissata in `rust-toolchain.toml`, rustup la installa da sé |
| Docker | recente | per il PostgreSQL di sviluppo e di test |
| OpenSSL | 3.x | per generare le chiavi Ed25519. Su macOS `brew install openssl@3`: l'`openssl` di sistema è LibreSSL e non conosce Ed25519 |
| `psql` | opzionale | gli script usano il client dell'immagine `postgres:16-alpine` se non lo trovano |

## Primo avvio

```bash
# MinIO è lo storage oggetti dei documenti; `minio-setup` crea il bucket una volta.
docker compose up -d postgres minio minio-setup
./ops/dev/generate-jwt-keys.sh        # crea .secrets/, che è ignorato da git
cp .env.example .env                  # adattare se serve
set -a; source .env; set +a           # il file lo carica la shell, non il codice

# I ruoli si creano una volta per cluster
docker compose exec -T postgres psql -U postgres -d postgres \
  -v db_name=propflow \
  -v migrator_password=migrator -v app_password=app -v worker_password=worker \
  -f /dev/stdin < ops/sql/roles.sql

cargo run -p propflow-migrator        # applica lo schema
cargo run -p propflow-api             # http://localhost:8080/docs
```

In alternativa, `docker compose up` avvia lo stack completo: `migrate` gira come
job e `api` con `worker` partono solo dopo che è completato.

## Comandi

```bash
# Il comando da usare prima di ogni consegna: container effimero, ruoli, migration,
# invarianti dello schema, chiavi, tutti i test.
./ops/ci/test-integration.sh

# Varianti utili
PROPFLOW_TEST_KEEP=1 ./ops/ci/test-integration.sh   # non distrugge il container
PROPFLOW_TEST_PORT=55499 ./ops/ci/test-integration.sh

# Controlli rapidi, senza database
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace           # i test che richiedono il DB si dichiarano saltati
./ops/ci/check-domain-purity.sh

# Solo le invarianti dello schema, su un database già migrato
./ops/ci/check-schema.sh "postgres://propflow_migrator:migrator@127.0.0.1:5432/propflow"
```

## Strategia di test

Quattro livelli sul codice, più una guardia sulla documentazione.

| Livello | Dove | Verifica | Database |
|---|---|---|---|
| Unitari di dominio | `crates/domain/src/**` | invarianti, validazioni, transizioni | no |
| Unitari applicativi | `crates/application/src/**` | autorizzazione, policy sulle password | no |
| Integrazione persistenza | `crates/infrastructure/tests/persistence.rs` | query, contesto RLS, atomicità di comando ed evento | **sì** |
| Integrazione storage | `crates/infrastructure/tests/object_storage.rs` | firma SigV4, URL pre-firmati, digest, contro un MinIO vero | no, ma serve MinIO |
| Contratto HTTP | `crates/api/tests/` | codici di stato, formato degli errori, autorizzazione | **sì** |
| Guardie sulla documentazione | `crates/api/tests/docs_config_reference.rs` | che le tabelle di [07 — Operazioni](07-operazioni.md#configurazione) non restino indietro rispetto al codice | no |

Attualmente **402 test**.

L'ultimo livello non verifica il codice ma la documentazione: deriva le
impostazioni dai sorgenti di `AppConfig` e del migratore e fallisce se una manca
dalla tabella, se una riga della tabella non esiste più, o se un default
documentato non è quello che il progetto spedisce. È l'unico controllo che una
riscrittura della documentazione può rompere, ed è voluto ([ADR-023](08-decisioni.md#adr-023)).

### Perché i test di integrazione usano un database reale

La Row Level Security e i vincoli dello schema *sono* comportamento di PostgreSQL.
Verificarli con un doppio in memoria non proverebbe nulla: proverebbe che il doppio
si comporta come crediamo che si comporti il database.

I test passano dai **repository** e non dal SQL diretto: è il percorso che usa il
codice applicativo, e testare la RLS scrivendo query a mano proverebbe che il
database funziona, non che il nostro layer lo usa correttamente.

### Test saltati

Senza `PROPFLOW_TEST_APP_URL` e `PROPFLOW_TEST_PRIVILEGED_URL`, i test che
richiedono il database stampano `SALTATO` e ritornano, così `cargo test` resta
eseguibile su una macchina senza Docker. Lo stesso vale per i test dell'adattatore S3
senza `PROPFLOW_TEST_S3_ENDPOINT` e affini.

Il rischio è un verde silenzioso, e la CI lo chiude: un passo cerca `SALTATO`
nell'output e fallisce se lo trova. **Ne segue che ogni servizio richiesto da un test
deve essere disponibile in CI**, non che il marcatore possa essere tollerato: sono
`ops/ci/prepare-database.sh` e `ops/ci/prepare-object-storage.sh` a fornirli, ed
entrambi girano anche in locale da `ops/ci/test-integration.sh`, così le due procedure
non divergono.

### Convenzioni

- I dati di test usano email irripetibili (`unique_email`): i test girano sullo
  stesso database e un vincolo unique non deve far fallire il secondo test perché
  il primo è passato.
- Nei test `unwrap` ed `expect` sono ammessi e corretti: un panic segnala una
  precondizione non soddisfatta. I crate di test hanno l'`allow` in testa.
- Le assert includono il corpo della risposta nel messaggio: `assert_eq!(status,
  OK, "corpo: {body}")`. Un test che dice solo "atteso 204, ottenuto 403" costringe
  a riprodurre a mano.

## Ricette

### Aggiungere una migration

```bash
# 1. Nuovo file, numerazione progressiva
$EDITOR migrations/0017_descrizione_breve.sql
```

- **Mai modificare una migration già mergiata.** Lo schema è append-only.
- Qualificare tutto con `app.`.
- Se aggiunge una tabella di business: `org_id NOT NULL`, i tre timestamp, RLS
  attiva con le policy, indici con `org_id` in testa.
- Se aggiunge una policy di SELECT: **non** metterci `deleted_at IS NULL` (vedi
  [ADR-008](08-decisioni.md#adr-008)).
- Se aggiunge una foreign key: un indice sulla colonna referenziante, altrimenti il
  controllo 4 fallisce.

```bash
# 2. Applicare e verificare
cargo run -p propflow-migrator
./ops/ci/check-schema.sh "$DATABASE_URL"
./ops/ci/test-integration.sh
```

Il migratore legge le migration a runtime, quindi un file nuovo viene visto senza
ricompilare.

### Aggiungere un caso d'uso

1. **Dominio**: entità o metodo sull'aggregato, con test unitari. Nuovo evento →
   variante in `DomainEventPayload`.
2. **Porta**: se serve una capacità nuova, un metodo su un trait esistente in
   `domain::ports`, oppure un trait nuovo. Se è un servizio tecnico (cifratura,
   storage, provider AI) va in `application::ports`.
3. **Use case** in `crates/application/src/use_cases/`, con la sequenza canonica:
   autorizza → costruisci → apri la transazione → persisti → accoda l'evento →
   commit.
4. **Infrastruttura**: implementazione della porta. Query come letterali statici
   composti con `concat!`, cast espliciti sui tipi dominio e sugli enum.
5. **Bordo HTTP**: vedi [05 — API](05-api.md#aggiungere-un-endpoint).
6. **Test**: unitari, di integrazione, di contratto.

### Aggiungere un provider esterno

Il vincolo è "nessun vendor lock-in", e si rispetta decidendo la forma
dell'astrazione **prima** di scrivere il primo adattatore.

1. Trait in `application::ports`, con errori propri (`CryptoError`,
   `StorageError`): niente tipi della libreria nella firma.
2. Implementazione in `crates/infrastructure/`.
3. Costruzione in `crates/api/src/main.rs` e iniezione come `Arc<dyn Port>`.
4. Configurazione in `AppConfig`, con il nome del provider come **dato**, non come
   ramo del codice.

`ObjectStorage` è l'esempio completo: porta dichiarata dal primo giorno, adattatore
scritto molto dopo in `crates/infrastructure/src/storage/`, e nessuna riga della porta
riscritta per accomodarlo — che è il punto di deciderne la forma prima. `Notifier` è
ancora dichiarata e senza adattatore: è la prossima.

### Aggiungere un enum al database

1. `ALTER TYPE app.x ADD VALUE 'y'` in una migration nuova (non blocca).
2. Variante corrispondente nell'enum di dominio.
3. Riga nella macro `string_enum!` in `crates/infrastructure/src/db/enums.rs`.
4. Variante nel DTO al bordo, se esposto.

Il valore sconosciuto in lettura dà **errore** e non un default: un enum esteso nel
database e non nel codice deve essere rumoroso, non silenzioso.

## Errori comuni

| Sintomo | Causa probabile |
|---|---|
| `403` con `accesso negato dalle policy di riga` su una scrittura | La query modifica una riga che la policy di SELECT non lascia rileggere. Vedi [ADR-008](08-decisioni.md#adr-008) |
| Una query restituisce zero righe che dovrebbero esserci | Contesto RLS mancante o `org_id` sbagliato; oppure manca `deleted_at IS NULL` e la riga è archiviata |
| `dynamic SQL string` non compila | sqlx 0.9 rifiuta le stringhe non statiche. Comporre con `concat!`, non con `format!` |
| `cargo --locked` fallisce | `Cargo.lock` non corrisponde ai manifest: rigenerarlo con `cargo check` e committarlo |
| Il migratore riapplica tutto | Bookkeeping duplicato: controllo 10 delle invarianti, [ADR-009](08-decisioni.md#adr-009) |
| `expect_used` fallisce in un test | Manca l'`allow` in testa al crate di test |
| Il test di integrazione si salta | Variabili d'ambiente assenti: usare `./ops/ci/test-integration.sh` |
| `missing field private_key_path` all'avvio | `.env` non lo carica nessuno: `set -a; source .env; set +a` ([ADR-023](08-decisioni.md#adr-023)) |
| L'API parte, ma con bind, timeout o TTL diversi da quelli documentati | `config/default.toml` si risolve rispetto alla directory di lavoro: avviarla dalla radice del repository ([ADR-023](08-decisioni.md#adr-023)) |
