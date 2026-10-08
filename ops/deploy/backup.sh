#!/usr/bin/env bash
# Copia di sicurezza: il **database e il bucket**, cifrati, fuori dalla macchina.
#
# DUE COSE E NON UNA
# I documenti sono oggetti nello storage, non righe: un dump del database li lascia
# fuori, e la riga che li cita resta senza byte. È la stessa constatazione che
# `just down-hard` ha già prodotto una volta in sviluppo, e in produzione sarebbe
# un contratto senza il suo PDF.
#
# CIFRATE, E FUORI DALLA MACCHINA
# Un backup sullo stesso disco del database non è un backup: è una seconda copia
# dello stesso guasto. E un dump non cifrato è l'anagrafica di tutti gli inquilini
# in chiaro sul primo disco che finisce da qualche parte.
#
# NON È UN BACKUP FINCHÉ NON È STATO RIPRISTINATO
# Un `pg_dump` che gira e non dà errori prova che il comando esiste. La prova è
# `ops/deploy/restore.sh`, e va eseguita **una volta prima** che dentro ci siano i
# dati di qualcun altro.
#
# COME SI USA
#   PROPFLOW_BACKUP_PASSPHRASE=... ops/deploy/backup.sh
#
# Da un cron, di notte:
#   0 3 * * * PROPFLOW_BACKUP_PASSPHRASE=... /srv/propflow/ops/deploy/backup.sh >> /var/log/propflow-backup.log 2>&1
set -euo pipefail

COMPOSE="${PROPFLOW_COMPOSE:-/srv/propflow/ops/deploy/docker-compose.prod.yml}"
ENV_FILE="${PROPFLOW_ENV_FILE:-/srv/propflow/.env.prod}"
STAGING="${PROPFLOW_BACKUP_DIR:-/var/backups/propflow}"
# Dove finisce la copia: un bucket remoto, una macchina in rsync, qualunque cosa
# **non sia questo disco**. Vuoto significa «non spedirla», ed è la configurazione
# di chi sta ancora provando: lo script lo dice invece di fingere.
REMOTE="${PROPFLOW_BACKUP_REMOTE:-}"
RETENTION_DAYS="${PROPFLOW_BACKUP_RETENTION_DAYS:-30}"

: "${PROPFLOW_BACKUP_PASSPHRASE:?serve PROPFLOW_BACKUP_PASSPHRASE per cifrare la copia}"

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
work="${STAGING}/${stamp}"
mkdir -p "$work"

echo "== dump del database =="
# `--format=custom`: comprime e permette il ripristino selettivo. Il dump lo fa il
# ruolo del **migratore**, che vede tutto: quello applicativo è soggetto alla Row
# Level Security e produrrebbe un dump con dentro zero righe di business — un
# backup vuoto che non dà errori, che è il modo peggiore di sbagliare.
docker compose -f "$COMPOSE" --env-file "$ENV_FILE" exec -T postgres \
  pg_dump --format=custom --no-owner --dbname="propflow" --username="postgres" \
  > "${work}/propflow.dump"

echo "== copia del bucket dei documenti =="
# `aws s3 sync` nel servizio `storage-cli`, sulla stessa rete del compose: non serve
# installare la CLI sulla macchina. `--delete` è il `--remove` di `mc mirror`, che
# c'era prima (ADR-057): la copia è lo stato del bucket, non un accumulo.
docker compose -f "$COMPOSE" --env-file "$ENV_FILE" --profile tools run --rm --no-deps \
  -v "${work}:/backup" storage-cli \
  s3 sync s3://propflow-documents /backup/documents --delete --only-show-errors

echo "== archivio cifrato =="
archive="${STAGING}/propflow-${stamp}.tar.gz.gpg"
tar -C "$work" -czf - . \
  | gpg --batch --yes --symmetric --cipher-algo AES256 \
        --passphrase "$PROPFLOW_BACKUP_PASSPHRASE" \
        --output "$archive"
rm -rf "$work"
echo "  $(du -h "$archive" | cut -f1)  $archive"

if [[ -n "$REMOTE" ]]; then
  echo "== copia fuori dalla macchina =="
  # `rclone` o `rsync`, a scelta di chi installa: qui si usa quello che c'è.
  if command -v rclone >/dev/null 2>&1; then
    rclone copy "$archive" "$REMOTE"
  else
    rsync -a "$archive" "$REMOTE"
  fi
else
  echo "== PROPFLOW_BACKUP_REMOTE non è impostata: la copia resta su questa macchina =="
  echo "   Un backup sullo stesso disco del database non è un backup." >&2
fi

echo "== ritenzione: $RETENTION_DAYS giorni =="
find "$STAGING" -name 'propflow-*.tar.gz.gpg' -mtime "+${RETENTION_DAYS}" -print -delete
