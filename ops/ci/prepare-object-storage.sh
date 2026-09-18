#!/usr/bin/env bash
# Prepara uno storage oggetti per i test: avvia MinIO e crea il bucket.
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
# MinIO non è un service container del runner perché l'immagine vuole `server
# /data` come comando, e i service di GitHub Actions non permettono di
# sovrascriverlo. Un `docker run` esplicito funziona uguale ed è la stessa riga in
# locale.
#
# Parametri dall'ambiente:
#   PROPFLOW_TEST_MINIO_CONTAINER   nome del container (default: propflow-test-minio)
#   PROPFLOW_TEST_MINIO_PORT        porta esposta (default: 55490)
#   PROPFLOW_TEST_S3_BUCKET         bucket da creare (default: propflow-test)
#
# In uscita stampa le variabili come righe KEY=value, così il chiamante può farne
# `eval` in locale o appenderle a $GITHUB_ENV in CI.
set -euo pipefail

: "${PROPFLOW_TEST_MINIO_CONTAINER:=propflow-test-minio}"
: "${PROPFLOW_TEST_MINIO_PORT:=55490}"
: "${PROPFLOW_TEST_S3_BUCKET:=propflow-test}"
# Credenziali di prova, con un container effimero e una porta locale: non sono un
# segreto e non hanno un posto nel gestore dei segreti.
ACCESS_KEY=propflow
SECRET_KEY=propflow-test-secret
ENDPOINT="http://127.0.0.1:${PROPFLOW_TEST_MINIO_PORT}"

log() { printf '  %s\n' "$*" >&2; }

log "MinIO su ${ENDPOINT}"
docker rm -f "$PROPFLOW_TEST_MINIO_CONTAINER" >/dev/null 2>&1 || true
docker run -d --name "$PROPFLOW_TEST_MINIO_CONTAINER" \
  -e "MINIO_ROOT_USER=${ACCESS_KEY}" \
  -e "MINIO_ROOT_PASSWORD=${SECRET_KEY}" \
  -p "${PROPFLOW_TEST_MINIO_PORT}:9000" \
  quay.io/minio/minio:latest server /data >/dev/null

# La sonda è quella dichiarata da MinIO: risponde 200 quando accetta richieste.
for _ in $(seq 1 60); do
  if curl -fsS "${ENDPOINT}/minio/health/live" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
curl -fsS "${ENDPOINT}/minio/health/live" >/dev/null

# Il bucket lo crea il provisioning, non l'applicazione: `S3ObjectStorage` non ha
# un `create_bucket` e non deve averlo, perché in produzione significherebbe dare
# all'API il privilegio di crearne.
log "bucket ${PROPFLOW_TEST_S3_BUCKET}"
docker run --rm --network host --entrypoint sh quay.io/minio/mc:latest -c "
  mc alias set test '${ENDPOINT}' '${ACCESS_KEY}' '${SECRET_KEY}' >/dev/null
  mc mb --ignore-existing 'test/${PROPFLOW_TEST_S3_BUCKET}' >/dev/null
" >&2

cat <<VARS
PROPFLOW_TEST_S3_ENDPOINT=${ENDPOINT}
PROPFLOW_TEST_S3_BUCKET=${PROPFLOW_TEST_S3_BUCKET}
PROPFLOW_TEST_S3_ACCESS_KEY_ID=${ACCESS_KEY}
PROPFLOW_TEST_S3_SECRET_ACCESS_KEY=${SECRET_KEY}
VARS
