#!/usr/bin/env bash
# Esegue la suite completa contro un PostgreSQL reale, in un container effimero.
#
# La Row Level Security e i vincoli dello schema sono comportamento del database:
# testarli con un doppio in memoria non proverebbe nulla.
#
# In CI il database è un service del runner e questa parte non serve: là si chiama
# direttamente ops/ci/prepare-database.sh. La preparazione è condivisa, così le due
# procedure non divergono.
set -euo pipefail

CONTAINER="${PROPFLOW_TEST_CONTAINER:-propflow-test-db}"
STORAGE_CONTAINER="${PROPFLOW_TEST_STORAGE_CONTAINER:-propflow-test-storage}"
PORT="${PROPFLOW_TEST_PORT:-55433}"
IMAGE="postgres:16-alpine"
KEEP="${PROPFLOW_TEST_KEEP:-0}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

cleanup() {
  if [[ "$KEEP" != "1" ]]; then
    docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
    docker rm -f "$STORAGE_CONTAINER" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

echo "== container di test =="
docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
docker run -d --name "$CONTAINER" \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_INITDB_ARGS="--locale=C --encoding=UTF8" \
  -p "${PORT}:5432" "$IMAGE" >/dev/null

for _ in $(seq 1 60); do
  docker exec "$CONTAINER" pg_isready -U postgres >/dev/null 2>&1 && break
  sleep 1
done
docker exec "$CONTAINER" pg_isready -U postgres

echo "== preparazione dello schema =="
eval "$(PGPORT="$PORT" "${ROOT}/ops/ci/prepare-database.sh")"

echo "== storage oggetti di test =="
# Anche lo storage è effimero, per la stessa ragione del database: i test
# dell'adattatore S3 si dichiarerebbero `SALTATO` senza, e un test saltato in
# silenzio è un verde che non prova nulla.
eval "$(PROPFLOW_TEST_STORAGE_CONTAINER="$STORAGE_CONTAINER" "${ROOT}/ops/ci/prepare-object-storage.sh")"

echo "== controlli statici sullo schema =="
"${ROOT}/ops/ci/check-schema.sh" "$PROPFLOW_TEST_MIGRATOR_URL"

echo "== chiavi di firma per i test =="
"${ROOT}/ops/dev/generate-jwt-keys.sh" "${ROOT}/.secrets" >/dev/null

echo "== suite =="
# Percorsi assoluti: i test di integrazione girano con la working directory nella
# cartella del crate, non nella radice del workspace.
PROPFLOW_TEST_APP_URL="$PROPFLOW_TEST_APP_URL" \
PROPFLOW_TEST_PRIVILEGED_URL="$PROPFLOW_TEST_PRIVILEGED_URL" \
PROPFLOW_TEST_JWT_PRIVATE_KEY="${ROOT}/.secrets/jwt_ed25519.pem" \
PROPFLOW_TEST_JWT_PUBLIC_KEY="${ROOT}/.secrets/jwt_ed25519.pub.pem" \
PROPFLOW_TEST_S3_ENDPOINT="$PROPFLOW_TEST_S3_ENDPOINT" \
PROPFLOW_TEST_S3_BUCKET="$PROPFLOW_TEST_S3_BUCKET" \
PROPFLOW_TEST_S3_ACCESS_KEY_ID="$PROPFLOW_TEST_S3_ACCESS_KEY_ID" \
PROPFLOW_TEST_S3_SECRET_ACCESS_KEY="$PROPFLOW_TEST_S3_SECRET_ACCESS_KEY" \
  cargo test --workspace --locked -- --test-threads=4
