-- =============================================================================
-- PropFlow — bootstrap: estensioni, schema, domini, contesto RLS, helper
-- Richiede PostgreSQL 16+
-- =============================================================================

CREATE EXTENSION IF NOT EXISTS pgcrypto;    -- gen_random_uuid(), digest()
CREATE EXTENSION IF NOT EXISTS citext;      -- email/slug case-insensitive
CREATE EXTENSION IF NOT EXISTS btree_gist;  -- EXCLUDE (uuid WITH =, daterange WITH &&)
CREATE EXTENSION IF NOT EXISTS pg_trgm;     -- ricerca fuzzy su indirizzi/nomi

CREATE SCHEMA IF NOT EXISTS app;
COMMENT ON SCHEMA app IS 'PropFlow business schema. Runtime role: propflow_app (RLS enforced).';

-- ---------------------------------------------------------------------------
-- Domini: un solo posto dove cambiare precisione monetaria e validazioni
-- ---------------------------------------------------------------------------
CREATE DOMAIN app.money_amount   AS NUMERIC(14,2);
CREATE DOMAIN app.currency_code  AS CHAR(3) CHECK (VALUE ~ '^[A-Z]{3}$');
CREATE DOMAIN app.percentage     AS NUMERIC(5,2) CHECK (VALUE >= 0 AND VALUE <= 100);
CREATE DOMAIN app.confidence     AS NUMERIC(5,4) CHECK (VALUE >= 0 AND VALUE <= 1);
CREATE DOMAIN app.email_address  AS CITEXT
  CHECK (VALUE ~ '^[^@[:space:]]+@[^@[:space:]]+\.[A-Za-z]{2,}$');
CREATE DOMAIN app.country_code   AS CHAR(2) CHECK (VALUE ~ '^[A-Z]{2}$');

-- ---------------------------------------------------------------------------
-- Contesto di richiesta: unico canale tra applicazione e RLS.
-- Il repository Rust apre una transazione e chiama app.set_request_context().
-- ---------------------------------------------------------------------------
CREATE FUNCTION app.current_user_id() RETURNS uuid
  LANGUAGE sql STABLE PARALLEL SAFE AS
$$ SELECT NULLIF(current_setting('app.current_user_id', true), '')::uuid $$;

CREATE FUNCTION app.current_org_id() RETURNS uuid
  LANGUAGE sql STABLE PARALLEL SAFE AS
$$ SELECT NULLIF(current_setting('app.current_org_id', true), '')::uuid $$;

CREATE FUNCTION app.is_platform_admin() RETURNS boolean
  LANGUAGE sql STABLE PARALLEL SAFE AS
$$ SELECT COALESCE(NULLIF(current_setting('app.platform_admin', true), ''), 'off') = 'on' $$;

CREATE FUNCTION app.request_id() RETURNS text
  LANGUAGE sql STABLE PARALLEL SAFE AS
$$ SELECT NULLIF(current_setting('app.request_id', true), '') $$;

CREATE FUNCTION app.client_ip() RETURNS inet
  LANGUAGE sql STABLE PARALLEL SAFE AS
$$ SELECT NULLIF(current_setting('app.client_ip', true), '')::inet $$;

CREATE FUNCTION app.user_agent() RETURNS text
  LANGUAGE sql STABLE PARALLEL SAFE AS
$$ SELECT NULLIF(current_setting('app.user_agent', true), '') $$;

-- is_local = true: il contesto muore con la transazione (nessuna perdita fra
-- richieste che riusano la stessa connessione del pool).
CREATE FUNCTION app.set_request_context(
    p_user_id        uuid,
    p_org_id         uuid    DEFAULT NULL,
    p_platform_admin boolean DEFAULT false,
    p_request_id     text    DEFAULT NULL,
    p_client_ip      inet    DEFAULT NULL,
    p_user_agent     text    DEFAULT NULL,
    p_actor_role     text    DEFAULT NULL
) RETURNS void LANGUAGE plpgsql AS $$
BEGIN
  PERFORM set_config('app.current_user_id', COALESCE(p_user_id::text, ''), true);
  PERFORM set_config('app.current_org_id',  COALESCE(p_org_id::text, ''),  true);
  PERFORM set_config('app.platform_admin',  CASE WHEN p_platform_admin THEN 'on' ELSE 'off' END, true);
  PERFORM set_config('app.request_id',      COALESCE(p_request_id, ''),    true);
  PERFORM set_config('app.client_ip',       COALESCE(p_client_ip::text, ''), true);
  PERFORM set_config('app.user_agent',      COALESCE(left(p_user_agent, 512), ''), true);
  PERFORM set_config('app.actor_role',      COALESCE(p_actor_role, ''),     true);
END $$;

COMMENT ON FUNCTION app.set_request_context IS
  'Da chiamare come primo statement di ogni transazione applicativa. Le policy RLS
   ri-verificano sempre la membership nel DB: una GUC falsificata non concede accesso.';

-- ---------------------------------------------------------------------------
-- Helper generici
-- ---------------------------------------------------------------------------
CREATE FUNCTION app.tg_set_updated_at() RETURNS trigger
  LANGUAGE plpgsql AS $$
BEGIN
  NEW.updated_at := now();
  RETURN NEW;
END $$;

-- Redazione: usata dal trigger di audit per non persistere segreti/PII.
CREATE FUNCTION app.jsonb_redact(p_doc jsonb, p_keys text[]) RETURNS jsonb
  LANGUAGE sql IMMUTABLE PARALLEL SAFE AS
$$
  SELECT CASE
    WHEN p_doc IS NULL THEN NULL
    ELSE COALESCE(
      (SELECT jsonb_object_agg(e.k,
                CASE WHEN e.k = ANY (p_keys) THEN '"[REDACTED]"'::jsonb ELSE e.v END)
       FROM jsonb_each(p_doc) AS e(k, v)),
      '{}'::jsonb)
  END
$$;

-- Append-only guard, riusata da audit_log.
CREATE FUNCTION app.tg_forbid_mutation() RETURNS trigger
  LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION '% is append-only: % is not allowed', TG_TABLE_NAME, TG_OP
    USING ERRCODE = 'insufficient_privilege';
END $$;
