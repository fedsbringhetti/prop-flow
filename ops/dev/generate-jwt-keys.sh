#!/usr/bin/env bash
# Genera una coppia di chiavi Ed25519 per la firma degli access token.
#
# Le chiavi finiscono in .secrets/, che è in .gitignore: una chiave privata non
# entra mai nel repository, nemmeno per lo sviluppo. In produzione le fornisce il
# gestore dei segreti come file montato.
set -euo pipefail

DEST="${1:-.secrets}"

# LibreSSL, che è l'`openssl` di sistema su macOS, non conosce Ed25519: serve un
# OpenSSL 3. Si cerca fra i percorsi noti invece di dare per scontato il PATH.
find_openssl() {
  local candidates=(
    "${OPENSSL_BIN:-}"
    openssl
    /opt/homebrew/opt/openssl@3/bin/openssl
    /opt/homebrew/opt/openssl/bin/openssl
    /usr/local/opt/openssl@3/bin/openssl
    /usr/bin/openssl
  )
  for candidate in "${candidates[@]}"; do
    [[ -z "$candidate" ]] && continue
    if command -v "$candidate" >/dev/null 2>&1 \
       && "$candidate" genpkey -algorithm ED25519 -out /dev/null 2>/dev/null; then
      command -v "$candidate"
      return 0
    fi
  done
  return 1
}

if ! OPENSSL=$(find_openssl); then
  echo "errore: nessun openssl con supporto Ed25519 trovato." >&2
  echo "        su macOS: brew install openssl@3" >&2
  exit 1
fi

mkdir -p "$DEST"
if [[ -f "$DEST/jwt_ed25519.pem" && -f "$DEST/jwt_ed25519.pub.pem" ]]; then
  echo "chiavi già presenti in $DEST, nulla da fare"
  exit 0
fi

"$OPENSSL" genpkey -algorithm ED25519 -out "$DEST/jwt_ed25519.pem"
"$OPENSSL" pkey -in "$DEST/jwt_ed25519.pem" -pubout -out "$DEST/jwt_ed25519.pub.pem"
chmod 600 "$DEST/jwt_ed25519.pem"

echo "generate con $($OPENSSL version):"
echo "  $DEST/jwt_ed25519.pem      (privata, non committare)"
echo "  $DEST/jwt_ed25519.pub.pem  (pubblica)"
