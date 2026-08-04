# syntax=docker/dockerfile:1

# =============================================================================
# Build
# =============================================================================
FROM rust:1.95-slim-bookworm AS builder

WORKDIR /build

# Le dipendenze cambiano molto meno spesso del codice: copiandole prima, una
# modifica a un file .rs non invalida la compilazione dell'intero albero.
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/domain/Cargo.toml         crates/domain/
COPY crates/application/Cargo.toml    crates/application/
COPY crates/infrastructure/Cargo.toml crates/infrastructure/
COPY crates/api/Cargo.toml            crates/api/
COPY crates/worker/Cargo.toml         crates/worker/
COPY crates/migrator/Cargo.toml       crates/migrator/

# Sorgenti minimi per far risolvere e compilare le sole dipendenze.
RUN set -eux; \
    for lib in domain application infrastructure; do \
      mkdir -p "crates/$lib/src" && echo "" > "crates/$lib/src/lib.rs"; \
    done; \
    for bin in api worker migrator; do \
      mkdir -p "crates/$bin/src" && echo "fn main() {}" > "crates/$bin/src/main.rs"; \
    done; \
    cargo build --release --locked --workspace

# Ora il codice vero. Si rimuovono gli artefatti dei crate del workspace per
# forzarne la ricompilazione: le dipendenze esterne restano in cache.
COPY crates crates
RUN set -eux; \
    find target/release -maxdepth 1 -name 'propflow*' -delete; \
    rm -rf target/release/.fingerprint/propflow*; \
    cargo build --release --locked --workspace

# =============================================================================
# Runtime
# =============================================================================
FROM debian:bookworm-slim AS runtime

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends ca-certificates; \
    rm -rf /var/lib/apt/lists/*; \
    # Utente non privilegiato: un processo che non ha bisogno di root non deve
    # averlo, e con un filesystem in sola lettura non può nemmeno scriversi addosso.
    useradd --system --uid 10001 --no-create-home --shell /usr/sbin/nologin propflow

WORKDIR /app
COPY --from=builder /build/target/release/propflow-api      /usr/local/bin/
COPY --from=builder /build/target/release/propflow-worker   /usr/local/bin/
COPY --from=builder /build/target/release/propflow-migrator /usr/local/bin/
# Il migratore legge le migration a runtime: vanno nell'immagine, non incorporate
# nel binario (vedi il commento in crates/migrator/src/main.rs).
COPY migrations ./migrations
COPY config ./config

USER propflow
EXPOSE 8080
ENV PROPFLOW__SERVER__BIND=0.0.0.0:8080

CMD ["propflow-api"]
