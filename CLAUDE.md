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

Partenza: **`docs/README.md`**, che è la mappa. Poi, in base al compito:

| Se il compito riguarda | Leggi prima |
|---|---|
| qualunque cosa | `docs/README.md`, questo file |
| un nuovo endpoint o un DTO | `docs/05-api.md`, `docs/02-architettura.md` |
| un nuovo use case o regola di dominio | `docs/02-architettura.md`, `docs/06-sviluppo.md` |
| lo schema, una migration, la RLS | `docs/03-modello-dati.md`, `docs/08-decisioni.md` |
| autenticazione, token, permessi, PII | `docs/04-sicurezza.md` |
| deploy, configurazione, CI | `docs/07-operazioni.md` |
| capire perché una cosa è fatta così | `docs/08-decisioni.md` |
| sapere cosa manca | `docs/09-stato-e-roadmap.md` |

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

### Sicurezza

18. **Niente dati personali nei log, nelle tracce e negli eventi di dominio.**
    Email, token, password, corpi di richiesta: mai. Gli eventi trasportano
    identificatori e nomi di campo, non valori.
19. **`Debug` non deve mostrare segreti.** `PasswordHash` e `TokenHash` hanno
    implementazioni che stampano `<redatto>`; farlo anche per i tipi nuovi.
20. **Il pool privilegiato (`begin_privileged`) si usa solo dove un contesto di
    sicurezza non può esistere**: registrazione, login, rotazione del refresh
    token. Ogni altro uso è un errore di progetto.
21. **Nessun segreto nel repository.** Chiavi e password arrivano da file montati
    o variabili d'ambiente. `.secrets/` è ignorato.

---

## 3. Metodo di lavoro

- **Si verifica prima di committare, e i commit si concordano.** Implementa,
  esegui la suite completa, presenta il risultato, *poi* prepara i commit.
- **Conventional Commits, scritti in inglese.** Messaggi di commit, titoli e corpi
  delle PR vanno in inglese. Il codice, i commenti e `docs/` restano in italiano:
  la cronologia git è la parte del progetto che sopravvive ai contributori e che
  un lettore esterno incontra per prima. Vedi `git log` per il registro esistente.
  Il corpo spiega il **perché**, non il cosa: il diff dice già cosa è cambiato.
- **Nessuna autoreferenza dell'assistente** nei messaggi di commit, nei corpi
  delle PR, nei commenti: né firme, né `Co-Authored-By`, né riferimenti a come il
  cambiamento è stato prodotto o a documenti di pianificazione esterni al
  repository. Un commit si spiega da sé a chi lo leggerà fra un anno.
- **Branch:** `<tipo>/<descrizione-kebab>`. `dev` è il branch di integrazione,
  `main` la linea stabile. Mai committare direttamente su `main` o `dev`.
- **PR in stack:** la base è il branch padre, non `dev`. Non usare *squash* sul
  padre di uno stack: rompe i figli.
- **I commenti spiegano il perché.** Un commento che riformula il codice è
  rumore; uno che spiega perché una via apparentemente più semplice non funziona
  vale il suo spazio.
- **La documentazione si aggiorna insieme al codice.** Una decisione nuova o
  cambiata è una voce in `docs/08-decisioni.md`.

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
docker compose up -d postgres
./ops/dev/generate-jwt-keys.sh          # crea .secrets/, richiede OpenSSL 3
cp .env.example .env                    # poi adattare
cargo run -p propflow-migrator
cargo run -p propflow-api               # Swagger UI su http://localhost:8080/docs
cargo run -p propflow-worker            # dispatcher dell'outbox
```

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
ops/ci/                 script usati dalla pipeline e in locale
ops/sql/                provisioning dei ruoli, invarianti dello schema
docs/                   documentazione di progetto — lettura obbligatoria
```

Direzione delle dipendenze, unica ammessa:

```
api ──► application ──► domain
 │                        ▲
 └──► infrastructure ─────┘
```

`api` conosce `infrastructure` solo in `main.rs`, per costruire le
implementazioni concrete e iniettarle come `Arc<dyn Port>`.
