#!/usr/bin/env bash
# Emette un invito da proprietario: genera un token opaco, ne scrive lo hash in
# app.owner_invitations con il ruolo del migratore, e stampa il token in chiaro
# una volta sola.
#
# ADR-053: chi diventa proprietario non si registra da una rotta aperta a
# chiunque, ma da un invito emesso qui dietro un token. Come si verifichi che
# una persona sia un proprietario vero — un'email, una telefonata, un
# documento — è una decisione di chi amministra la piattaforma e non vive nel
# prodotto: nel prodotto vive solo il fatto che il token esiste.
#
# Uso:
#   ./ops/dev/issue-owner-invitation.sh <email> [giorni-di-validità]
#
# Legge PROPFLOW__DATABASE__MIGRATOR_URL dall'ambiente (la stessa variabile che
# usa propflow-migrator): `set -a; source .env; set +a` prima di lanciarlo a
# mano, o farlo lanciare da `just`, che la carica da sé.
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "uso: $0 <email> [giorni-di-validità, default 7]" >&2
  exit 1
fi

EMAIL="$1"
# Sette giorni di default: la stessa scadenza dell'invito all'inquilino
# (Invitation::TTL_DAYS in crates/domain/src/identity/invitation.rs).
DAYS="${2:-7}"
# Un intero e basta: `:days` entra nel SQL senza virgolette (è un numero), quindi
# è l'unico argomento che lo script non può lasciare a psql da controllare.
if ! [[ "$DAYS" =~ ^[1-9][0-9]{0,2}$ ]]; then
  echo "giorni di validità non validi: «${DAYS}» (atteso un intero fra 1 e 999)" >&2
  exit 1
fi

: "${PROPFLOW__DATABASE__MIGRATOR_URL:?PROPFLOW__DATABASE__MIGRATOR_URL non impostata: vedi .env.example}"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INSERT_SQL="${ROOT}/ops/dev/issue-owner-invitation.sql"

log() { printf '  %s\n' "$*" >&2; }

# In CI e in molti ambienti di sviluppo `psql` è installato. In locale può non
# esserci: si usa allora il client dell'immagine Postgres, come fanno gli altri
# script di ops/ (vedi ops/ci/prepare-database.sh). Il repository va montato in
# sola lettura perché `-f` legge `issue-owner-invitation.sql` dal percorso
# reale — **non** `-c`: psql non interpola le variabili `:'nome'` nel testo
# passato a `-c`, solo leggendo da uno script.
if command -v psql >/dev/null 2>&1; then
  run_psql() { psql "$PROPFLOW__DATABASE__MIGRATOR_URL" "$@"; }
else
  log "psql non trovato: uso il client dell'immagine postgres:16-alpine"
  run_psql() {
    docker run --rm -i --network host \
      -v "${ROOT}:${ROOT}:ro" -w "${ROOT}" \
      postgres:16-alpine psql "$PROPFLOW__DATABASE__MIGRATOR_URL" "$@"
  }
fi

# 32 byte casuali, base64url senza padding: **esattamente** la codifica di
# `random_token` in crates/infrastructure/src/security/refresh.rs. Le due
# codifiche devono coincidere byte per byte, perché è quel modulo a verificare
# il token quando arriva all'accettazione — un test di contratto incolla un
# token emesso da questo script e verifica che il server lo accetti.
TOKEN="$(openssl rand 32 | openssl base64 -A | tr '+/' '-_' | tr -d '=')"

# SHA-256 del **testo** del token, non dei byte casuali: è ciò che il server
# calcola su ciò che il client rispedisce (hash_token in refresh.rs), quindi va
# calcolato sulla stringa base64url già codificata, non prima.
TOKEN_HASH_HEX="$(printf '%s' "$TOKEN" | openssl dgst -sha256 -binary | od -An -tx1 | tr -d ' \n')"

run_psql -q -v ON_ERROR_STOP=1 \
  -v email="$EMAIL" \
  -v token_hash="\\x${TOKEN_HASH_HEX}" \
  -v days="$DAYS" \
  -f "$INSERT_SQL" \
  >&2

log "invito emesso per ${EMAIL}, valido ${DAYS} giorni"
log "consegnalo fuori banda (non per email non cifrata): una volta stampato non si rilegge più"
echo "$TOKEN"
