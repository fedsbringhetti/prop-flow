-- =============================================================================
-- Identity, organizzazioni, RBAC, sessioni
-- =============================================================================

CREATE TABLE app.organizations (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name              TEXT NOT NULL CHECK (length(btrim(name)) BETWEEN 1 AND 200),
    slug              CITEXT NOT NULL CHECK (slug ~ '^[a-z0-9][a-z0-9-]{1,62}$'),
    kind              app.org_kind NOT NULL DEFAULT 'personal',
    country           app.country_code NOT NULL DEFAULT 'IT',
    timezone          TEXT NOT NULL DEFAULT 'Europe/Rome',
    default_currency  app.currency_code NOT NULL DEFAULT 'EUR',
    vat_number        TEXT,
    -- settings applicative (feature flag, preferenze notifiche): evita ALTER frequenti
    settings          JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at        TIMESTAMPTZ
);
-- unique "vivo": lo slug si libera dopo un soft delete
CREATE UNIQUE INDEX organizations_slug_uk ON app.organizations (slug) WHERE deleted_at IS NULL;
CREATE INDEX organizations_kind_ix ON app.organizations (kind) WHERE deleted_at IS NULL;

COMMENT ON TABLE app.organizations IS
  'Confine di isolamento dati (org_id in ogni tabella di business). Nell''MVP ogni
   proprietario ha una personal org; kind=condominium/business abilita la V3 senza migrazione.';

CREATE TABLE app.users (
    id                       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email                    app.email_address NOT NULL,
    email_verified_at        TIMESTAMPTZ,
    password_hash            TEXT,               -- Argon2id; NULL = solo invito/SSO futuro
    full_name                TEXT NOT NULL CHECK (length(btrim(full_name)) BETWEEN 1 AND 200),
    phone                    TEXT CHECK (phone IS NULL OR phone ~ '^\+?[0-9 ().-]{6,32}$'),
    locale                   TEXT NOT NULL DEFAULT 'it-IT',
    platform_role            app.platform_role NOT NULL DEFAULT 'user',
    status                   app.user_status NOT NULL DEFAULT 'pending',
    avatar_document_id       UUID,               -- FK aggiunta in 0006 (dipendenza circolare)
    -- PII cifrata lato applicazione (envelope encryption). Il DB non vede il plaintext
    -- e non conserva la chiave: solo il riferimento logico alla key version.
    fiscal_code_encrypted    BYTEA,
    fiscal_code_fingerprint  BYTEA,              -- HMAC-SHA256 per lookup/dedup senza decifrare
    encryption_key_ref       TEXT,
    mfa_secret_encrypted     BYTEA,
    terms_accepted_at        TIMESTAMPTZ,
    privacy_accepted_at      TIMESTAMPTZ,
    last_login_at            TIMESTAMPTZ,
    failed_login_count       SMALLINT NOT NULL DEFAULT 0 CHECK (failed_login_count >= 0),
    locked_until             TIMESTAMPTZ,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at               TIMESTAMPTZ
);
CREATE UNIQUE INDEX users_email_uk ON app.users (email) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX users_fiscal_fingerprint_uk ON app.users (fiscal_code_fingerprint)
    WHERE fiscal_code_fingerprint IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX users_full_name_trgm_ix ON app.users USING gin (full_name gin_trgm_ops);

COMMENT ON COLUMN app.users.encryption_key_ref IS
  'Riferimento logico alla chiave (es. "kms:v3"). Nessun materiale crittografico nel DB:
   il provider di cifratura è astratto dietro interfaccia (vincolo no-vendor-lock-in).';

CREATE TABLE app.org_members (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL REFERENCES app.organizations (id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES app.users (id) ON DELETE CASCADE,
    role        app.org_role NOT NULL,
    status      app.member_status NOT NULL DEFAULT 'active',
    invited_by  UUID REFERENCES app.users (id) ON DELETE SET NULL,
    joined_at   TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at  TIMESTAMPTZ,
    CONSTRAINT org_members_org_user_uk UNIQUE (org_id, user_id)
);
-- percorso caldo: app.member_role() a ogni valutazione di policy
CREATE INDEX org_members_lookup_ix ON app.org_members (user_id, org_id)
    WHERE status = 'active' AND deleted_at IS NULL;
CREATE INDEX org_members_org_role_ix ON app.org_members (org_id, role) WHERE deleted_at IS NULL;
CREATE INDEX org_members_invited_by_ix ON app.org_members (invited_by) WHERE invited_by IS NOT NULL;
-- almeno un owner per org: invariante applicata dal Service Layer, non dal DB
-- (richiederebbe un trigger cross-row = workflow, escluso dai vincoli Parte 4)

CREATE TABLE app.org_invitations (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id       UUID NOT NULL REFERENCES app.organizations (id) ON DELETE CASCADE,
    email        app.email_address NOT NULL,
    role         app.org_role NOT NULL DEFAULT 'tenant',
    token_hash   BYTEA NOT NULL,                 -- SHA-256 del token: mai il token in chiaro
    lease_id     UUID,                           -- FK aggiunta in 0005: invito legato a un contratto
    invited_by   UUID REFERENCES app.users (id) ON DELETE SET NULL,
    expires_at   TIMESTAMPTZ NOT NULL,
    accepted_at  TIMESTAMPTZ,
    accepted_by  UUID REFERENCES app.users (id) ON DELETE SET NULL,
    revoked_at   TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT org_invitations_expiry_ck CHECK (expires_at > created_at)
);
CREATE UNIQUE INDEX org_invitations_token_uk ON app.org_invitations (token_hash);
CREATE UNIQUE INDEX org_invitations_pending_uk ON app.org_invitations (org_id, email)
    WHERE accepted_at IS NULL AND revoked_at IS NULL;
CREATE INDEX org_invitations_org_ix ON app.org_invitations (org_id, created_at DESC);
CREATE INDEX org_invitations_invited_by_ix ON app.org_invitations (invited_by)
    WHERE invited_by IS NOT NULL;
CREATE INDEX org_invitations_accepted_by_ix ON app.org_invitations (accepted_by)
    WHERE accepted_by IS NOT NULL;

CREATE TABLE app.user_sessions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id             UUID NOT NULL REFERENCES app.users (id) ON DELETE CASCADE,
    refresh_token_hash  BYTEA NOT NULL,
    -- rotazione: la sessione ruotata punta alla successiva (rilevamento riuso token)
    rotated_to          UUID REFERENCES app.user_sessions (id) ON DELETE SET NULL,
    issued_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at          TIMESTAMPTZ NOT NULL,
    revoked_at          TIMESTAMPTZ,
    revoked_reason      TEXT,
    client_ip           INET,
    user_agent          TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX user_sessions_token_uk ON app.user_sessions (refresh_token_hash);
CREATE INDEX user_sessions_active_ix ON app.user_sessions (user_id, expires_at DESC)
    WHERE revoked_at IS NULL;
CREATE INDEX user_sessions_rotated_to_ix ON app.user_sessions (rotated_to)
    WHERE rotated_to IS NOT NULL;
