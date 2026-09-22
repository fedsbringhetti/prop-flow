# PropFlow

A two-sided PropTech platform for property management: owners and tenants use the
same system through different views and permissions. Beyond accounting, two levers
traditional tools do not have — intelligent automation (OCR of utility bills, triage
of maintenance issues from a photograph) and co-living management with expense
splitting.

Backend in **Rust** (Axum, SQLx) on **PostgreSQL 16** with Row Level Security, ports
and adapters architecture. Web client in **React and TypeScript**, served from the
same origin behind a reverse proxy.

## Quick start

Requirements: Rust (`rust-toolchain.toml` pins the version, rustup installs it), Node
24, Docker, OpenSSL 3 and [`just`](https://github.com/casey/just) (`brew install just`
or `cargo install just`).

```bash
just setup                              # keys, .env, containers, roles, migrations, npm ci
```

Then, in three terminals:

```bash
just api                                # http://localhost:8080/docs
just worker                             # outbox dispatcher, also reads the bills
just web                                # http://localhost:5173
```

`just` (alone) lists every recipe. What `setup` does step by step, and how to do it
by hand without `just`, is in [`docs/06-sviluppo.md`](docs/06-sviluppo.md#primo-avvio).
Outgoing mail lands in mailpit at `http://localhost:8025`: it accepts everything
and delivers nothing.

Vite forwards `/api/v1` to the API, so the browser sees a **single origin** — the
same contract a reverse proxy provides in production, and the reason there is no
CORS to configure. `docker compose up` brings up the whole stack, client included,
behind that proxy on `http://localhost:8081`.

The interface is in Italian: its users are Italian owners and tenants. English is
deferred, not declined, which is why no visible string is written inside a
component — see [ADR-039](docs/08-decisioni.md#adr-039).

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
apps/web/               React + TypeScript client, Italian interface
ops/                    CI and development scripts, role provisioning, reverse proxy
docs/                   project documentation
```

## Status

The database schema and the backend cover the whole MVP: authentication, properties,
rooms, leases, invitations, documents, maintenance issues, utility bills with
automatic reading, charges, payments and co-living expenses.

The web client covers the **owner's path** end to end — dashboard, properties and
rooms, leases and parties, bills with OCR review, charges, invitations. The tenant's
path is the next slice, and it is next by dependency rather than priority: a tenant
does not exist until an owner invites one. What is missing, and why, is in
[status and roadmap](docs/09-stato-e-roadmap.md).
