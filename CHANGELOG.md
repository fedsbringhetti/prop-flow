# Changelog

Tutte le modifiche degne di nota. Il formato segue [Keep a Changelog](https://keepachangelog.com/),
le versioni seguono [Semantic Versioning](https://semver.org/) sul ramo 0.x: prima
di 1.0 le rotture di compatibilità alzano il *minor*.

Generato da [git-cliff](https://git-cliff.org/) a partire dai Conventional Commits.
Vedi `docs/10-rilasci.md`.

## [0.1.0] — 2026-08-07

### Build

- **docker**: Add postgres 16 service for local development
- Add cargo workspace with pinned toolchain and workspace lints
- **docker**: Add multi-stage image and full development stack

### Correzioni

- **db**: Stop select policies from filtering soft deleted rows
- **ci**: Unblock the dependency gate
- **db**: Stop the application role from rewriting isolation boundaries

### Documentazione

- Add operating instructions for work on the repository
- Add project documentation
- State that commit messages are written in English
- Require English in every git and GitHub artifact, and set a bar for PR bodies

### Funzionalità

- **db**: Add schema bootstrap, domains and rls request context
- **db**: Add domain enums
- **db**: Add identity, organization and rbac tables
- **db**: Add property and lease aggregates
- **db**: Add document storage with polymorphic attachments
- **db**: Add maintenance ticket aggregate
- **db**: Add billing and provider-agnostic payments
- **db**: Add co-living expense splitting
- **db**: Add ai job registry
- **db**: Add transactional domain event outbox
- **db**: Add partitioned audit log with field redaction
- **db**: Wire updated_at, integrity and audit triggers
- **db**: Enforce row level security policies
- **db**: Add dashboard views and least-privilege grants
- **domain**: Add typed identifiers and validated value objects
- **domain**: Add identity aggregates and request context
- **domain**: Add property aggregate with explicit patch semantics
- **domain**: Add domain events and persistence ports
- **application**: Add error model, password policy and authorization
- **application**: Add authentication use cases
- **application**: Add property use cases
- **infra**: Add postgres pools and rls-scoped unit of work
- **infra**: Add repositories and transactional outbox
- **infra**: Add argon2id hashing, eddsa tokens and opaque refresh tokens
- **infra**: Add structured logging and otlp tracing
- **migrator**: Add schema migration binary reading migrations at runtime
- **worker**: Add transactional outbox dispatcher
- **api**: Add configuration, di container and rfc 9457 error model
- **api**: Add bearer authentication and request context extractors
- **api**: Add auth and property endpoints with generated openapi
- **api**: Add router and process lifecycle
- **rooms**: Expose rooms as part of the property aggregate
- **leases**: Lease agreements with date-derived temporal states
- **invitations**: Tenant onboarding without the privileged pool

### Test

- **db**: Add deterministic rls seed fixture
- **db**: Add rls isolation and domain invariant suite
- **db**: Guard against soft delete filters in select policies
- **infra**: Add persistence and rls integration tests
- **api**: Add http contract tests over the in-process router


