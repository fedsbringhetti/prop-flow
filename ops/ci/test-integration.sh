#!/usr/bin/env bash
# Esegue la suite completa contro un PostgreSQL reale.
#
# La Row Level Security e i vincoli dello schema sono comportamento del database:
# testarli con un doppio in memoria non proverebbe nulla. Il container è
# effimero e viene ricreato a ogni esecuzione, così i test partono sempre da uno
# stato noto.
set -euo pipefail

CONTAINER="${PROPFLOW_TEST_CONTAINER:-propflow-test-db}"
PORT="${PROPFLOW_TEST_PORT:-55433}"
IMAGE="postgres:16-alpine"
KEEP="${PROPFLOW_TEST_KEEP:-0}"

cleanup() {
  if [[ "$KEEP" != "1" ]]; then
    docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
docker run -d --name "$CONTAINER" \
  -e POSTGRES_DB=postgres \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_INITDB_ARGS="--locale=C --encoding=UTF8" \
  -p "${PORT}:5432" "$IMAGE" >/dev/null

echo "== attesa del database =="
for _ in $(seq 1 60); do
  docker exec "$CONTAINER" pg_isready -U postgres >/dev/null 2>&1 && break
  sleep 1
done
docker exec "$CONTAINER" pg_isready -U postgres

echo "== ruoli e database =="
docker cp ops/sql/roles.sql "$CONTAINER:/tmp/roles.sql" >/dev/null
docker exec "$CONTAINER" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -q \
  -c "CREATE DATABASE propflow" >/dev/null
docker exec "$CONTAINER" psql -v ON_ERROR_STOP=1 -U postgres -d postgres -q \
  -v migrator_password=migrator -v app_password=app -v worker_password=worker \
  -f /tmp/roles.sql >/dev/null

BASE="postgres://%s:%s@127.0.0.1:${PORT}/propflow"
MIGRATOR_URL=$(printf "$BASE" propflow_migrator migrator)
APP_URL=$(printf "$BASE" propflow_app app)
PRIVILEGED_URL=$(printf "$BASE" propflow_worker worker)

echo "== migration (tramite il binario propflow-migrator) =="
PROPFLOW__DATABASE__MIGRATOR_URL="$MIGRATOR_URL" cargo run --quiet -p propflow-migrator

# Percorsi assoluti: i test di integrazione girano con la working directory nella
# cartella del crate, non nella radice del workspace.
echo "== chiavi di firma per i test =="
./ops/dev/generate-jwt-keys.sh .secrets >/dev/null

echo "== suite =="
PROPFLOW_TEST_APP_URL="$APP_URL" \
PROPFLOW_TEST_PRIVILEGED_URL="$PRIVILEGED_URL" \
PROPFLOW_TEST_MIGRATOR_URL="$MIGRATOR_URL" \
PROPFLOW_TEST_JWT_PRIVATE_KEY="$PWD/.secrets/jwt_ed25519.pem" \
PROPFLOW_TEST_JWT_PUBLIC_KEY="$PWD/.secrets/jwt_ed25519.pub.pem" \
  cargo test --workspace -- --test-threads=4
