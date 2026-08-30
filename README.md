# PropFlow

A two-sided PropTech platform for property management: owners and tenants use the
same system through different views and permissions. Beyond accounting, two levers
traditional tools do not have — intelligent automation (OCR of utility bills, triage
of maintenance issues from a photograph) and co-living management with expense
splitting.

Backend in **Rust** (Axum, SQLx) on **PostgreSQL 16** with Row Level Security, ports
and adapters architecture.

## Quick start

```bash
# MinIO backs document storage; `minio-setup` creates the bucket once.
docker compose up -d postgres minio minio-setup
./ops/dev/generate-jwt-keys.sh          # requires OpenSSL 3

# Nothing loads .env on its own: copy it, adapt it, then hand it to the shell.
cp .env.example .env
set -a; source .env; set +a

# Database roles, once per cluster
docker compose exec -T postgres psql -U postgres -d postgres \
  -v db_name=propflow \
  -v migrator_password=migrator -v app_password=app -v worker_password=worker \
  -f /dev/stdin < ops/sql/roles.sql

cargo run -p propflow-migrator          # applies the schema
cargo run -p propflow-api               # http://localhost:8080/docs
```

Full suite (ephemeral PostgreSQL and MinIO containers, migrations, invariants, every
test):

```bash
./ops/ci/test-integration.sh
```

## Documentation

Project documentation lives in [`docs/`](docs/README.md) and **must be read before
touching the code**: several choices are counterintuitive and come from an observed
defect, with the reason recorded in [`docs/08-decisioni.md`](docs/08-decisioni.md).

The documentation is written in Italian, as are the comments in the code. Everything
that reaches GitHub — commits, branches, pull requests, releases — is in English.

| | |
|---|---|
| [Product](docs/01-prodotto.md) | vision, MVP scope, roadmap |
| [Architecture](docs/02-architettura.md) | crates, dependency rule, Unit of Work, outbox |
| [Data model](docs/03-modello-dati.md) | ER diagram, Row Level Security, audit, migrations |
| [Security](docs/04-sicurezza.md) | authentication, tokens, isolation, personal data |
| [API](docs/05-api.md) | HTTP conventions, error model, endpoints |
| [Development](docs/06-sviluppo.md) | requirements, commands, tests, recipes |
| [Operations](docs/07-operazioni.md) | configuration, deployment, observability, CI |
| [Decisions](docs/08-decisioni.md) | record of architectural choices |
| [Status and roadmap](docs/09-stato-e-roadmap.md) | what exists, what is missing, known debt |
| [Releases](docs/10-rilasci.md) | versioning, changelog, release flow |

[`CLAUDE.md`](CLAUDE.md) holds the operating rules in condensed form, for the people
and assistants working on the repository.

## Layout

```
crates/domain           entities, value objects, events, persistence ports
crates/application      use cases, authorization, technical ports
crates/infrastructure   PostgreSQL, Argon2id, EdDSA, telemetry
crates/api        [bin] Axum: router, DTOs, OpenAPI
crates/worker     [bin] outbox dispatcher
crates/migrator   [bin] schema migrations
migrations/             append-only SQL
ops/                    CI and development scripts, role provisioning
docs/                   project documentation
```

## Status

The database schema covers the whole MVP. The backend covers authentication,
properties, rooms, leases and invitations; the remaining modules are listed in
[status and roadmap](docs/09-stato-e-roadmap.md).
