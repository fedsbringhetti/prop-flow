#!/usr/bin/env bash
# Prepara un database PropFlow: crea il database, i tre ruoli e applica le
# migration. Condiviso fra la CI e il runner locale, perché due procedure di
# preparazione divergono al primo cambiamento e poi "in locale funziona".
#
# Parametri dall'ambiente, con default adatti allo sviluppo:
#   PGHOST PGPORT                  host e porta di PostgreSQL
#   PGUSER PGPASSWORD              credenziali di un superuser
#   PROPFLOW_DB_NAME               database da creare (default: propflow)
#   PROPFLOW_{MIGRATOR,APP,WORKER}_PASSWORD   password dei tre ruoli
#
# In uscita stampa le tre URL di connessione come righe KEY=value, così il
# chiamante può farne `eval` in locale o appenderle a $GITHUB_ENV in CI.
set -euo pipefail

: "${PGHOST:=127.0.0.1}"
: "${PGPORT:=5432}"
: "${PGUSER:=postgres}"
: "${PGPASSWORD:=postgres}"
: "${PROPFLOW_DB_NAME:=propflow}"
: "${PROPFLOW_MIGRATOR_PASSWORD:=migrator}"
: "${PROPFLOW_APP_PASSWORD:=app}"
: "${PROPFLOW_WORKER_PASSWORD:=worker}"
export PGHOST PGPORT PGUSER PGPASSWORD

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

log() { printf '  %s\n' "$*" >&2; }

# In CI `psql` è installato sul runner. In locale può non esserci: si usa allora il
# client dell'immagine Postgres, con il repository montato in sola lettura perché
# gli script SQL vanno letti dal percorso reale.
if command -v psql >/dev/null 2>&1; then
  run_psql() { psql "$@"; }
else
  log "psql non trovato: uso il client dell'immagine postgres:16-alpine"
  run_psql() {
    docker run --rm -i --network host \
      -e PGHOST -e PGPORT -e PGUSER -e PGPASSWORD \
      -v "${ROOT}:${ROOT}:ro" -w "${ROOT}" \
      postgres:16-alpine psql "$@"
  }
fi

log "database ${PROPFLOW_DB_NAME} su ${PGHOST}:${PGPORT}"
if ! run_psql -d postgres -tAc "SELECT 1 FROM pg_database WHERE datname = '${PROPFLOW_DB_NAME}'" | grep -q 1; then
  run_psql -q -d postgres -c "CREATE DATABASE ${PROPFLOW_DB_NAME}"
fi

log "ruoli con privilegi minimi"
run_psql -q -v ON_ERROR_STOP=1 -d "${PROPFLOW_DB_NAME}" \
  -v migrator_password="${PROPFLOW_MIGRATOR_PASSWORD}" \
  -v app_password="${PROPFLOW_APP_PASSWORD}" \
  -v worker_password="${PROPFLOW_WORKER_PASSWORD}" \
  -f "${ROOT}/ops/sql/roles.sql" >/dev/null

url() { printf 'postgres://%s:%s@%s:%s/%s' "$1" "$2" "$PGHOST" "$PGPORT" "$PROPFLOW_DB_NAME"; }
MIGRATOR_URL="$(url propflow_migrator "$PROPFLOW_MIGRATOR_PASSWORD")"
APP_URL="$(url propflow_app "$PROPFLOW_APP_PASSWORD")"
WORKER_URL="$(url propflow_worker "$PROPFLOW_WORKER_PASSWORD")"

# Le migration passano dal binario, non da psql: è lo stesso percorso del deploy,
# quindi la CI verifica anche il migratore invece di aggirarlo.
log "migration tramite propflow-migrator"
PROPFLOW__DATABASE__MIGRATOR_URL="$MIGRATOR_URL" \
PROPFLOW__DATABASE__MIGRATIONS_DIR="${ROOT}/migrations" \
  cargo run --quiet --locked -p propflow-migrator >&2

cat <<VARS
PROPFLOW_TEST_MIGRATOR_URL=${MIGRATOR_URL}
PROPFLOW_TEST_APP_URL=${APP_URL}
PROPFLOW_TEST_PRIVILEGED_URL=${WORKER_URL}
VARS
