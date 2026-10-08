#!/usr/bin/env bash
# Prepara uno storage oggetti per i test: avvia SeaweedFS e crea il bucket.
#
# Condiviso fra la CI e il runner locale, come `prepare-database.sh` e per la
# stessa ragione: due procedure di preparazione divergono al primo cambiamento, e
# poi "in locale funziona".
#
# **Perché serve anche in CI.** I test dell'adattatore S3 si dichiarano `SALTATO`
# senza queste variabili, e la pipeline fallisce se trova quella parola: il verde
# deve significare "tutto è stato eseguito", non "qualcosa non c'era". Rendere lo
# storage disponibile è quindi l'unico modo di tenere il marcatore onesto.
#
# **SeaweedFS e non MinIO** (ADR-057): le immagini di MinIO non si scaricano più,
# né da Docker Hub né da quay.io. Le due immagini sono **fissate per versione**:
# `latest` è ciò che ha nascosto la rottura finché la cache locale c'era.
#
# Non è un service container del runner perché il comando (`server -s3`) va
# passato, e i service di GitHub Actions non permettono di sovrascriverlo. Un
# `docker run` esplicito funziona uguale ed è la stessa riga in locale.
#
# Parametri dall'ambiente:
#   PROPFLOW_TEST_STORAGE_CONTAINER   nome del container (default: propflow-test-storage)
#   PROPFLOW_TEST_STORAGE_PORT        porta esposta (default: 55490)
#   PROPFLOW_TEST_S3_BUCKET           bucket da creare (default: propflow-test)
#
# In uscita stampa le variabili come righe KEY=value, così il chiamante può farne
# `eval` in locale o appenderle a $GITHUB_ENV in CI.
set -euo pipefail

: "${PROPFLOW_TEST_STORAGE_CONTAINER:=propflow-test-storage}"
: "${PROPFLOW_TEST_STORAGE_PORT:=55490}"
: "${PROPFLOW_TEST_S3_BUCKET:=propflow-test}"
# Credenziali di prova, con un container effimero e una porta locale: non sono un
# segreto, e averle qui le rende identiche in CI e in locale.
ACCESS_KEY=propflow
SECRET_KEY=propflow-test-secret
STORAGE_IMAGE=chrislusf/seaweedfs:4.48
CLI_IMAGE=amazon/aws-cli:2.37.7
ENDPOINT="http://127.0.0.1:${PROPFLOW_TEST_STORAGE_PORT}"

log() { printf '  %s\n' "$*" >&2; }

log "SeaweedFS su ${ENDPOINT}"
docker rm -f "$PROPFLOW_TEST_STORAGE_CONTAINER" >/dev/null 2>&1 || true
# Le credenziali dall'ambiente bastano: SeaweedFS ne fa l'identità con tutti i
# permessi, e una firma con un segreto diverso viene rifiutata.
docker run -d --name "$PROPFLOW_TEST_STORAGE_CONTAINER" \
  -e "AWS_ACCESS_KEY_ID=${ACCESS_KEY}" \
  -e "AWS_SECRET_ACCESS_KEY=${SECRET_KEY}" \
  -p "${PROPFLOW_TEST_STORAGE_PORT}:8333" \
  "$STORAGE_IMAGE" server -s3 -dir=/data >/dev/null

for _ in $(seq 1 60); do
  if curl -fsS "${ENDPOINT}/healthz" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
curl -fsS "${ENDPOINT}/healthz" >/dev/null

log "bucket ${PROPFLOW_TEST_S3_BUCKET}"
# `BucketAlreadyOwnedByYou` non è un errore: è il `--ignore-existing` di `mc mb`, e
# senza tollerarlo un secondo giro fallirebbe su un bucket che c'è già.
if ! out=$(docker run --rm --network host \
    -e "AWS_ACCESS_KEY_ID=${ACCESS_KEY}" \
    -e "AWS_SECRET_ACCESS_KEY=${SECRET_KEY}" \
    -e AWS_DEFAULT_REGION=us-east-1 \
    "$CLI_IMAGE" --endpoint-url "$ENDPOINT" \
    s3api create-bucket --bucket "$PROPFLOW_TEST_S3_BUCKET" 2>&1); then
  if [[ "$out" != *BucketAlreadyOwnedByYou* ]]; then
    printf '%s\n' "$out" >&2
    exit 1
  fi
fi

cat <<VARS
PROPFLOW_TEST_S3_ENDPOINT=${ENDPOINT}
PROPFLOW_TEST_S3_BUCKET=${PROPFLOW_TEST_S3_BUCKET}
PROPFLOW_TEST_S3_ACCESS_KEY_ID=${ACCESS_KEY}
PROPFLOW_TEST_S3_SECRET_ACCESS_KEY=${SECRET_KEY}
VARS
