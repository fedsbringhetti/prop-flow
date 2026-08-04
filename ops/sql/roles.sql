-- =============================================================================
-- Ruoli PropFlow — eseguito una volta per cluster, come superuser.
-- NON è una migration di schema: CREATE ROLE è cluster-wide.
--
--   psql -v migrator_password=... -v app_password=... -v worker_password=... \
--        -f ops/sql/roles.sql "postgresql://postgres@localhost/postgres"
-- =============================================================================

\set ON_ERROR_STOP on

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
GRANT CONNECT ON DATABASE propflow TO propflow_migrator, propflow_app, propflow_worker;
ALTER DATABASE propflow OWNER TO propflow_migrator;

-- search_path: SQLx e il codice applicativo usano nomi non qualificati
ALTER ROLE propflow_migrator SET search_path = app, public;
ALTER ROLE propflow_app      SET search_path = app, public;
ALTER ROLE propflow_worker   SET search_path = app, public;

-- IMPORTANTE: propflow_app NON deve essere owner di alcuna tabella —
-- l'owner di una tabella è esente dalla Row Level Security.
