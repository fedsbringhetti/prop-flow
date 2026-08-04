#!/usr/bin/env bash
# Verifica il vincolo "il dominio non dipende dall'infrastruttura".
#
# Non è una convenzione da controllare in code review: se `propflow-domain`
# acquisisce sqlx, axum o un client HTTP, questo script fallisce e la pipeline si
# ferma. È l'unico modo perché il vincolo resti vero fra sei mesi.
set -euo pipefail

FORBIDDEN='^(sqlx|axum|tokio|hyper|reqwest|aws-sdk|opentelemetry|tower|jsonwebtoken|argon2)'

echo "== dipendenze proibite nell'albero di propflow-domain =="
if cargo tree --package propflow-domain --edges normal --prefix none --no-dedupe \
   | sed 's/ .*//' | sort -u | grep -Eq "$FORBIDDEN"; then
  echo "FALLITO: il dominio dipende da infrastruttura:" >&2
  cargo tree --package propflow-domain --edges normal --prefix none --no-dedupe \
    | sed 's/ .*//' | sort -u | grep -E "$FORBIDDEN" >&2
  exit 1
fi
echo "ok: nessuna dipendenza di infrastruttura"

echo "== application non conosce l'infrastruttura concreta =="
if cargo tree --package propflow-application --edges normal --prefix none --no-dedupe \
   | sed 's/ .*//' | sort -u | grep -Eq '^(sqlx|axum|propflow-infrastructure)'; then
  echo "FALLITO: l'application layer dipende da infrastruttura concreta" >&2
  exit 1
fi
echo "ok: solo porte astratte"

echo "== gli handler HTTP non contengono query SQL =="
if grep -rnE 'sqlx::|SELECT |INSERT INTO|UPDATE .* SET' crates/api/src/routes/ 2>/dev/null; then
  echo "FALLITO: SQL dentro un handler HTTP" >&2
  exit 1
fi
echo "ok: nessun SQL nei bordi HTTP"
