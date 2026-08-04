# PropFlow

Piattaforma PropTech bidirezionale per la gestione immobiliare: proprietari e
inquilini usano lo stesso sistema con viste e permessi diversi. Oltre alla
contabilità, due leve che gli strumenti tradizionali non hanno — automazione
intelligente (OCR delle bollette, triage dei guasti da fotografia) e gestione del
co-living con divisione delle spese.

Backend in **Rust** (Axum, SQLx) su **PostgreSQL 16** con Row Level Security,
architettura a porte e adattatori.

## Avvio rapido

```bash
docker compose up -d postgres
./ops/dev/generate-jwt-keys.sh          # richiede OpenSSL 3
cp .env.example .env

# Ruoli del database, una volta per cluster
docker compose exec -T postgres psql -U postgres -d postgres \
  -v migrator_password=migrator -v app_password=app -v worker_password=worker \
  -f /dev/stdin < ops/sql/roles.sql

cargo run -p propflow-migrator          # applica lo schema
cargo run -p propflow-api               # http://localhost:8080/docs
```

Suite completa (container PostgreSQL effimero, migrazioni, invarianti, tutti i test):

```bash
./ops/ci/test-integration.sh
```

## Documentazione

La documentazione di progetto è in [`docs/`](docs/README.md) e **va letta prima di
intervenire sul codice**: diverse scelte sono controintuitive e nascono da un
difetto osservato, con il motivo registrato in
[`docs/08-decisioni.md`](docs/08-decisioni.md).

| | |
|---|---|
| [Prodotto](docs/01-prodotto.md) | visione, perimetro dell'MVP, roadmap |
| [Architettura](docs/02-architettura.md) | crate, regola delle dipendenze, Unit of Work, outbox |
| [Modello dati](docs/03-modello-dati.md) | diagramma ER, Row Level Security, audit, migrazioni |
| [Sicurezza](docs/04-sicurezza.md) | autenticazione, token, isolamento, dati personali |
| [API](docs/05-api.md) | convenzioni HTTP, modello di errore, endpoint |
| [Sviluppo](docs/06-sviluppo.md) | requisiti, comandi, test, ricette |
| [Operazioni](docs/07-operazioni.md) | configurazione, deploy, osservabilità, CI |
| [Decisioni](docs/08-decisioni.md) | registro delle scelte architetturali |
| [Stato e roadmap](docs/09-stato-e-roadmap.md) | cosa c'è, cosa manca, debiti noti |

[`CLAUDE.md`](CLAUDE.md) contiene le regole operative in forma sintetica, per
persone e assistenti che lavorano sul repository.

## Struttura

```
crates/domain           entità, value object, eventi, porte di persistenza
crates/application      use case, autorizzazione, porte tecniche
crates/infrastructure   PostgreSQL, Argon2id, EdDSA, telemetria
crates/api        [bin] Axum: router, DTO, OpenAPI
crates/worker     [bin] dispatcher dell'outbox
crates/migrator   [bin] migrazioni dello schema
migrations/             SQL append-only
ops/                    script di CI e sviluppo, provisioning dei ruoli
docs/                   documentazione di progetto
```

## Stato

Lo schema del database copre l'intero MVP. Il backend copre autenticazione e
gestione degli immobili; i moduli restanti sono elencati in
[stato e roadmap](docs/09-stato-e-roadmap.md).
