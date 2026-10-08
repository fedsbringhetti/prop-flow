#!/usr/bin/env bash
# Ripristino di una copia di sicurezza. **È ciò che rende un backup un backup.**
#
# Un `pg_dump` che gira e non dà errori prova che il comando esiste. Questo script
# va eseguito **una volta prima** che nel prodotto ci siano i dati di qualcun altro,
# su un database vuoto, e il prodotto deve partirci sopra. Finché non è successo,
# non c'è un backup: c'è un file.
#
# COME SI USA — su una macchina di prova, non su quella di produzione
#   PROPFLOW_BACKUP_PASSPHRASE=... ops/deploy/restore.sh /var/backups/propflow/propflow-....tar.gz.gpg
#
# CHE COSA VERIFICARE DOPO, e non è il codice di uscita di questo script:
#   1. `docker compose ... up -d` e l'API risponde su `/ready`;
#   2. si entra con un account che esisteva **prima** del dump;
#   3. si apre un contratto e si **scarica un documento**: è la metà che un dump
#      del solo database non porta, ed è quella che si scopre mancante mesi dopo.
set -euo pipefail

# Il controllo è un `if` e non `${1:?...}`, e non è pignoleria: dentro
# `${var:?parola}` bash interpreta l'apostrofo come apertura di una stringa, e
# «dell'archivio» rompe lo script prima ancora che venga eseguito.
ARCHIVE="${1:-}"
if [[ -z "$ARCHIVE" ]]; then
  echo "serve il percorso dell'archivio cifrato" >&2
  exit 1
fi
COMPOSE="${PROPFLOW_COMPOSE:-/srv/propflow/ops/deploy/docker-compose.prod.yml}"
ENV_FILE="${PROPFLOW_ENV_FILE:-/srv/propflow/.env.prod}"

: "${PROPFLOW_BACKUP_PASSPHRASE:?serve PROPFLOW_BACKUP_PASSPHRASE per decifrare la copia}"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "== apertura dell'archivio =="
gpg --batch --yes --decrypt --passphrase "$PROPFLOW_BACKUP_PASSPHRASE" "$ARCHIVE" \
  | tar -C "$work" -xzf -
test -f "${work}/propflow.dump" || { echo "l'archivio non contiene propflow.dump" >&2; exit 1; }

echo "== il database deve essere vuoto =="
# Ripristinare sopra dati esistenti non è un ripristino, è una fusione con esiti
# imprevedibili: `pg_restore` fallirebbe a metà lasciando uno schema misto. Meglio
# fermarsi qui e dire che cosa fare.
tables="$(docker compose -f "$COMPOSE" --env-file "$ENV_FILE" exec -T postgres \
  psql -U postgres -d propflow -tAc \
  "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'app'" 2>/dev/null || echo 0)"
if [[ "${tables//[[:space:]]/}" != "0" ]]; then
  echo "lo schema 'app' esiste già ($tables tabelle): ripristina su un database vuoto." >&2
  exit 1
fi

echo "== il bucket dei documenti deve essere vuoto =="
# Stessa ragione della guardia sul database, e **sta qui**, prima di scrivere
# qualunque cosa: il caso normale non è la macchina nuova, è il database buttato per
# rifarlo dal dump mentre il volume dei documenti sopravvive. Accorgersene dopo aver
# ripristinato il database lascia un sistema a metà, che è peggio di un rifiuto.
# Un bucket che non esiste conta come vuoto: lo crea il passo del ripristino.
objects="$(docker compose -f "$COMPOSE" --env-file "$ENV_FILE" --profile tools \
  run --rm --no-deps --entrypoint sh storage-cli -c '
    out=$(aws s3api list-objects-v2 --bucket propflow-documents --max-keys 1 \
            --output text 2>&1) || out=""
    case "$out" in *CONTENTS*) echo pieno ;; *) echo vuoto ;; esac
  ' | tr -d '[:space:]')"
# Il confronto è un `case` e non un `=`: `docker compose run` può stampare righe sue
# sullo stdout, e un'uguaglianza stretta le leggerebbe come «non vuoto». Lo stato che
# non si riconosce si tratta come un errore, non come un via libera.
case "$objects" in
  *pieno*) echo "il bucket 'propflow-documents' non è vuoto: ripristina su uno storage vuoto." >&2
           exit 1 ;;
  *vuoto*) ;;
  *)       echo "non si è potuto leggere il bucket dei documenti: $objects" >&2
           exit 1 ;;
esac

echo "== ripristino del database =="
docker compose -f "$COMPOSE" --env-file "$ENV_FILE" exec -T postgres \
  pg_restore --no-owner --dbname="propflow" --username="postgres" < "${work}/propflow.dump"

echo "== ripristino del bucket dei documenti =="
# Il bucket lo crea `storage-setup` all'avvio dello stack; qui si ricrea solo per non
# dipendere dall'ordine in cui i due sono stati eseguiti, e «esiste già» è il caso
# normale, non un errore.
#
# **`cp --recursive` e non `sync`**, ed è la riga che conta: `sync` trasferisce solo se
# la dimensione differisce **oppure** se la sorgente è più recente della destinazione, e
# `tar` conserva le mtime dell'archivio — cioè i file che arrivano sono più vecchi degli
# oggetti che trovano, e `sync` li salterebbe uscendo `0` senza stampare niente. Prima
# era `mc mirror --overwrite`, e `--overwrite` in `sync` non ha un equivalente (ADR-057).
# La guardia sopra rende il bucket vuoto una precondizione; la copia resta incondizionata
# per non dipendere da quella guardia.
docker compose -f "$COMPOSE" --env-file "$ENV_FILE" --profile tools run --rm --no-deps \
  --entrypoint sh -v "${work}:/backup" storage-cli -c '
    out=$(aws s3api create-bucket --bucket propflow-documents 2>&1) \
      || case "$out" in *BucketAlreadyOwnedByYou*) ;; *) echo "$out" >&2; exit 1 ;; esac
    aws s3 cp /backup/documents s3://propflow-documents --recursive --only-show-errors
  '

echo ""
echo "== ripristinato. Adesso la parte che conta =="
echo "   1. avvia lo stack e chiedi /ready"
echo "   2. entra con un account che esisteva prima del dump"
echo "   3. apri un contratto e **scarica un documento**"
echo "   Finché il punto 3 non è successo, il backup copre metà del prodotto."
