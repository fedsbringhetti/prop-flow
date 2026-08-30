-- =============================================================================
-- Ruoli PropFlow — eseguito una volta per cluster, come superuser.
-- NON è una migration di schema: CREATE ROLE è cluster-wide.
--
--   psql -v db_name=propflow \
--        -v migrator_password=... -v app_password=... -v worker_password=... \
--        -f ops/sql/roles.sql "postgresql://postgres@localhost/postgres"
-- =============================================================================

\set ON_ERROR_STOP on

-- I ruoli sono del cluster, i privilegi no: `GRANT CONNECT` e `ALTER DATABASE`
-- valgono su **un** database, e un cluster può ospitarne più di uno — uno di
-- staging accanto a uno di sviluppo, due giri in parallelo. Il nome arriva quindi
-- da chi invoca lo script.
--
-- Con un default, e non obbligatorio: i comandi di primo avvio documentati in
-- README.md e in docs/06-sviluppo.md sono copiati altrove, e renderlo obbligatorio
-- li farebbe fallire con `variable "db_name" not specified` — cioè fermerebbe chi
-- segue la documentazione. Il default sta scritto qui una volta sola invece che in
-- ogni comando che lo invoca.
\if :{?db_name}
\else
  \set db_name propflow
\endif

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_migrator') THEN
    CREATE ROLE propflow_migrator LOGIN;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_app') THEN
    CREATE ROLE propflow_app LOGIN;
  END IF;
  -- BYPASSRLS: dispatcher outbox, worker AI/OCR, bootstrap registrazione (pre-auth)
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_worker') THEN
    CREATE ROLE propflow_worker LOGIN BYPASSRLS;
  END IF;
END $$;

ALTER ROLE propflow_migrator PASSWORD :'migrator_password';
ALTER ROLE propflow_app      PASSWORD :'app_password';
ALTER ROLE propflow_worker   PASSWORD :'worker_password';

-- Il database va creato prima (CREATE DATABASE non è transazionale).
--
-- `:"db_name"` con le virgolette doppie, non `:'db_name'`: la prima forma è quella
-- che psql quota come **identificatore**, la seconda produrrebbe una stringa
-- letterale e la query non compilerebbe.
\echo 'ruoli e privilegi sul database' :db_name
GRANT CONNECT ON DATABASE :"db_name" TO propflow_migrator, propflow_app, propflow_worker;
ALTER DATABASE :"db_name" OWNER TO propflow_migrator;

-- search_path: SQLx e il codice applicativo usano nomi non qualificati
ALTER ROLE propflow_migrator SET search_path = app, public;
ALTER ROLE propflow_app      SET search_path = app, public;
ALTER ROLE propflow_worker   SET search_path = app, public;

-- IMPORTANTE: propflow_app NON deve essere owner di alcuna tabella —
-- l'owner di una tabella è esente dalla Row Level Security.
