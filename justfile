# Comandi del progetto.
#
# I nomi delle ricette sono in **inglese** come i nomi degli script in `ops/`, che
# è la superficie che si digita; i commenti sono in italiano come il resto della
# prosa interna. Vale il criterio di `CLAUDE.md` §3: qui non c'è niente che si
# incontri senza aprire il repository — `just --list` lo si legge stando dentro.
#
# **Questo file non è la fonte della verità, lo è `CLAUDE.md` §4.** Serve a non
# doverli ridigitare e a non sbagliarne i flag: `--locked` su clippy, `-D warnings`,
# `npm ci` invece di `npm install`. Una ricetta che diverga da §4 è un difetto di
# questo file, non una variante.
#
# **La riga di commento attaccata a una ricetta è la sua voce in `just --list`, ed
# è solo l'ultima**: le spiegazioni lunghe vanno in un blocco staccato da una riga
# vuota, altrimenti l'elenco mostra una frase tagliata a metà. È il motivo per cui
# qui sotto la prosa sta sopra e il sommario è una riga sola.

# `.env` **si carica da sé**, ed è la ragione principale per cui questo file esiste.
#
# Nessuno lo caricava: lo doveva fare la shell con `set -a; source .env; set +a`, e
# dimenticarlo dava un'API che non parte o — peggio — un worker che parte e non
# trova lo storage. `just` lo legge da sé prima di ogni ricetta, e se il file non
# c'è non fallisce: valgono i default di `config/default.toml`.
set dotenv-load

# Ogni ricetta è **una sola shell**: senza questo, ogni riga girerebbe per conto
# suo e un `cd` non sopravvivrebbe alla riga dopo.
set shell := ["bash", "-euo", "pipefail", "-c"]

WEB := "apps/web"

# L'elenco, quando si scrive `just` e basta.
default:
    @just --list --unsorted

# ---------------------------------------------------------------------------
# Preparare l'ambiente

# Da zero a pronto: chiavi, `.env`, servizi, database, dipendenze del client.
setup: keys env up db install
    @echo ""
    @echo "Pronto. 'just api', 'just worker' e 'just web' in tre terminali."

# Genera le chiavi di firma EdDSA in `.secrets/`. Richiede OpenSSL 3.
keys:
    ./ops/dev/generate-jwt-keys.sh

# Copia `.env.example` senza sovrascrivere, e **fallisce** se mancano impostazioni.
#
# Non sovrascrive di proposito: quel file contiene le scelte locali di chi lavora.
# Ma un `.env` più vecchio di una fetta che ha aggiunto impostazioni non le ha —
# è successo tre volte con le quattro `PROPFLOW__STORAGE__*`, ogni volta scoprendolo
# all'avvio — quindi qui si confronta prima.
#
# **Esce diverso da zero quando qualcosa manca**, e non è pedanteria: la prima
# stesura si limitava ad avvisare, `just setup` proseguiva, e l'errore vero
# arrivava tre passi dopo sotto forma di un'autenticazione fallita — cioè lontano
# dalla sua causa. Un avviso che non ferma niente è un avviso che si scorre.

# Crea `.env` da `.env.example`, o elenca che cosa manca a quello che c'è.
env:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ ! -f .env ]]; then
      cp .env.example .env
      echo ".env creato da .env.example."
      exit 0
    fi
    mancanti="$(comm -23 \
      <(grep -oE '^[A-Z][A-Z0-9_]*' .env.example | sort -u) \
      <(grep -oE '^[A-Z][A-Z0-9_]*' .env | sort -u))"
    if [[ -z "$mancanti" ]]; then
      echo ".env c'è ed è allineato a .env.example."
      exit 0
    fi
    echo "Il tuo .env non ha queste impostazioni, che .env.example porta:" >&2
    echo "$mancanti" | sed 's/^/  /' >&2
    echo "" >&2
    echo "Quelle obbligatorie impediscono l'avvio dell'API e del worker." >&2
    echo "Copiale da .env.example, poi riesegui." >&2
    exit 1

# `--wait` non è un dettaglio: senza, `up` torna appena i container sono
# *avviati*, e su un volume nuovo PostgreSQL sta ancora facendo `initdb`. Il passo
# successivo trovava la porta aperta e il server non pronto.
#
# **`minio-setup` resta fuori dall'attesa**, ed è la seconda cosa che si scopre
# provando: è un one-shot che crea il bucket ed esce `0`, e per `--wait` un
# servizio che termina è un fallimento. Aspettarlo faceva fallire `up` con tutti i
# container sani. Parte dopo, e `depends_on: service_healthy` gli garantisce
# comunque il MinIO pronto.

# Avvia PostgreSQL e MinIO aspettando che siano sani, poi crea il bucket.
up:
    docker compose up -d --wait postgres minio
    docker compose up -d minio-setup

# I volumi restano: i dati di un giro a mano non si buttano via per aver fermato
# un container. Per cancellarli davvero c'è `down-hard`.

# Ferma i servizi, lasciando i dati dove sono.
down:
    docker compose down

# Ferma i servizi e cancella i volumi. Si perde tutto quello che c'è nel database.
down-hard:
    docker compose down --volumes

# **Il passo che a `setup` mancava.** Su un database vuoto i tre ruoli non
# esistono, e il migratore si presenta come `propflow_migrator` con l'esito che si
# vede: `password authentication failed`. `ops/ci/prepare-database.sh` è lo stesso
# script che usa la CI — apposta, perché due procedure di preparazione divergono al
# primo cambiamento e poi «in locale funziona» — e i suoi default combaciano con
# gli URL di `.env.example`. Crea il database, i ruoli, e applica le migration.

# Crea database e ruoli, poi applica le migration. Su un database vuoto è questo.
db:
    ./ops/ci/prepare-database.sh >/dev/null

# Applica le sole migration, su un database che ha già i suoi ruoli.
migrate:
    cargo run -p propflow-migrator

# Installa le dipendenze del client esattamente dal lockfile, come `--locked`.
install:
    cd {{WEB}} && npm ci

# ---------------------------------------------------------------------------
# Eseguire

# Avvia l'API. Swagger UI su http://localhost:8080/docs
api:
    cargo run -p propflow-api

# Il worker è anche ciò che **legge le bollette**: senza, una scansione caricata
# resta in coda e la bolletta non arriva mai in revisione, e il sintomo è muto —
# niente va in errore, semplicemente non succede niente.

# Avvia il dispatcher dell'outbox, che legge anche le bollette.
worker:
    cargo run -p propflow-worker

# Avvia il client su http://localhost:5173, con `/api/v1` inoltrato all'API.
web:
    cd {{WEB}} && npm run dev

# Avvia lo stack intero dietro il reverse proxy, su http://localhost:8081.
stack:
    docker compose up

# ---------------------------------------------------------------------------
# Verificare

# Container PostgreSQL effimero, ruoli, migration, invarianti, chiavi, tutti i
# test. È il comando da usare prima di ogni consegna.

# La suite completa, con il database. Da eseguire prima di ogni consegna.
test:
    ./ops/ci/test-integration.sh

# I test che richiedono il database si dichiarano saltati invece di fallire.

# I controlli rapidi senza database: formattazione, clippy, purezza, test.
check: fmt clippy purity
    cargo test --workspace

# Verifica la formattazione senza riscriverla.
fmt:
    cargo fmt --all --check

# Riscrive la formattazione invece di limitarsi a verificarla.
fmt-fix:
    cargo fmt --all

# Clippy con zero warning tollerati, come in CI.
clippy:
    cargo clippy --workspace --all-targets --locked -- -D warnings

# Verifica che il dominio non conosca l'infrastruttura e che negli handler non ci sia SQL.
purity:
    ./ops/ci/check-domain-purity.sh

# Senza argomento usa l'URL del migratore, che `.env` porta.

# Verifica le invarianti dello schema su un database già migrato.
schema url=env_var_or_default("PROPFLOW__DATABASE__MIGRATOR_URL", ""):
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -z "{{url}}" ]]; then
      echo "Serve un URL: 'just schema postgres://...', oppure PROPFLOW__DATABASE__MIGRATOR_URL in .env" >&2
      exit 1
    fi
    ./ops/ci/check-schema.sh "{{url}}"

# Il client per intero: typecheck, lint, test, build.
web-check:
    cd {{WEB}} && npm run typecheck && npm run lint && npm test && npm run build

# I soli test del client.
web-test:
    cd {{WEB}} && npm test

# Nell'ordine in cui conviene incontrare un errore: prima i controlli rapidi, poi
# il client, e per ultima la suite con il database, che è la più lenta.

# Tutto ciò che verifica la CI.
ci: check web-check test
