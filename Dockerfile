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
# Build del client web
# =============================================================================
# Stage separato e non un passo dello stage Rust: le due build non condividono
# nulla e ricompilarne una perché è cambiata l'altra sarebbe tempo buttato. Con
# stage distinti Docker le tiene in cache separatamente e può eseguirle in
# parallelo.
FROM node:24-slim AS web-builder

WORKDIR /web

# Il lockfile prima del codice, per la stessa ragione dei manifest di cargo:
# `node_modules` cambia molto meno spesso di un componente.
COPY apps/web/package.json apps/web/package-lock.json ./
# `npm ci` e non `npm install`: installa esattamente il lockfile e fallisce se
# manifest e lock divergono. È l'equivalente di `cargo --locked`.
RUN npm ci

COPY apps/web/ ./
# `npm run build` esegue anche `tsc -b`: un errore di tipo ferma l'immagine
# invece di finirci dentro.
RUN npm run build

# =============================================================================
# Runtime del client web
# =============================================================================
# Un'immagine a sé e non i file statici dentro l'API. Costa un servizio in più e
# compra due cose: la build del frontend non entra nell'immagine dell'API, e i
# due processi HTTP non fanno lo stesso mestiere con profili di carico e di cache
# opposti — file immutabili con l'hash nel nome da un lato, risposte personali e
# non memorizzabili dall'altro.
FROM caddy:2-alpine AS web

COPY ops/web/Caddyfile /etc/caddy/Caddyfile
COPY --from=web-builder /web/dist /srv

EXPOSE 8080

# =============================================================================
# Runtime
# =============================================================================
FROM debian:bookworm-slim AS runtime

# Etichette OCI: permettono di risalire da un'immagine in esecuzione al commit che
# l'ha prodotta. Senza, l'unico modo di sapere cosa gira è fidarsi del tag.
ARG VERSION=0.0.0-dev
ARG REVISION=unknown
LABEL org.opencontainers.image.title="PropFlow" \
      org.opencontainers.image.description="Piattaforma PropTech bidirezionale" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}" \
      org.opencontainers.image.source="https://github.com/fedsbringhetti/prop-flow" \
      org.opencontainers.image.licenses="UNLICENSED"

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
