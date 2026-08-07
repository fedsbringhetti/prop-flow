#!/usr/bin/env bash
# Esegue le invarianti dello schema e propaga l'esito.
#
# Lo script SQL solleva un'eccezione se trova violazioni, quindi con
# ON_ERROR_STOP psql esce diverso da zero e la pipeline si ferma.
#
# Uso: ops/ci/check-schema.sh <URL_DATABASE>
set -euo pipefail

URL="${1:?uso: check-schema.sh URL_DATABASE}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CHECKS="${ROOT}/ops/sql/checks/schema_invariants.sql"

if command -v psql >/dev/null 2>&1; then
  exec psql -v ON_ERROR_STOP=1 -q -f "$CHECKS" "$URL"
fi

# Nessun client locale: si usa quello dell'immagine Postgres.
exec docker run --rm -i --network host -v "${ROOT}:${ROOT}:ro" -w "${ROOT}" \
  postgres:16-alpine psql -v ON_ERROR_STOP=1 -q -f "$CHECKS" "$URL"
