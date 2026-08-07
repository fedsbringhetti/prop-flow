-- =============================================================================
-- Patrimonio: immobili e stanze (co-living)
-- =============================================================================

CREATE TABLE app.properties (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id            UUID NOT NULL REFERENCES app.organizations (id) ON DELETE RESTRICT,
    owner_id          UUID NOT NULL REFERENCES app.users (id) ON DELETE RESTRICT,
    code              TEXT CHECK (code IS NULL OR length(btrim(code)) BETWEEN 1 AND 32),
    label             TEXT NOT NULL CHECK (length(btrim(label)) BETWEEN 1 AND 200),
    property_type     app.property_type NOT NULL DEFAULT 'apartment',
    status            app.property_status NOT NULL DEFAULT 'draft',
    is_coliving       BOOLEAN NOT NULL DEFAULT false,

    -- Indirizzo (colonne piatte: filtrabili e indicizzabili, a differenza di un JSONB)
    address_street    TEXT NOT NULL,
    address_extra     TEXT,
    city              TEXT NOT NULL,
    province          TEXT,
    postal_code       TEXT,
    country           app.country_code NOT NULL DEFAULT 'IT',
    latitude          NUMERIC(9,6)  CHECK (latitude  IS NULL OR latitude  BETWEEN -90 AND 90),
    longitude         NUMERIC(9,6)  CHECK (longitude IS NULL OR longitude BETWEEN -180 AND 180),

    -- Caratteristiche
    size_sqm          NUMERIC(8,2) CHECK (size_sqm IS NULL OR size_sqm > 0),
    rooms_count       SMALLINT CHECK (rooms_count IS NULL OR rooms_count >= 0),
    bathrooms_count   SMALLINT CHECK (bathrooms_count IS NULL OR bathrooms_count >= 0),
    floor             SMALLINT,
    has_elevator      BOOLEAN,
    energy_class      app.energy_class,

    -- Predisposizione V2/V3 (nessuna logica MVP): analytics ROI, catasto, APE
    purchase_price    app.money_amount CHECK (purchase_price IS NULL OR purchase_price >= 0),
    purchase_date     DATE,
    current_value     app.money_amount CHECK (current_value IS NULL OR current_value >= 0),
    currency          app.currency_code NOT NULL DEFAULT 'EUR',
    cadastral_data    JSONB NOT NULL DEFAULT '{}'::jsonb,

    notes             TEXT,
    metadata          JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at        TIMESTAMPTZ,

    -- ricerca full-text MVP; V2 sostituirà con Meilisearch alimentato dall'outbox
    search_vector     TSVECTOR GENERATED ALWAYS AS (
        to_tsvector('italian',
            coalesce(label, '') || ' ' || coalesce(code, '') || ' ' ||
            coalesce(address_street, '') || ' ' || coalesce(city, '') || ' ' ||
            coalesce(province, '') || ' ' || coalesce(postal_code, '') || ' ' ||
            coalesce(notes, ''))
    ) STORED
);
-- org_id in testa a ogni indice: la RLS filtra per org, il planner sfrutta il prefisso
CREATE INDEX properties_org_status_ix ON app.properties (org_id, status)
    WHERE deleted_at IS NULL;
CREATE INDEX properties_org_owner_ix  ON app.properties (org_id, owner_id)
    WHERE deleted_at IS NULL;
-- lookup per owner_id da solo: serve alla verifica della FK ON DELETE RESTRICT
CREATE INDEX properties_owner_ix ON app.properties (owner_id);
CREATE UNIQUE INDEX properties_org_code_uk ON app.properties (org_id, code)
    WHERE code IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX properties_city_ix ON app.properties (org_id, city) WHERE deleted_at IS NULL;
CREATE INDEX properties_search_ix ON app.properties USING gin (search_vector);
CREATE INDEX properties_metadata_ix ON app.properties USING gin (metadata jsonb_path_ops);
CREATE INDEX properties_coliving_ix ON app.properties (org_id)
    WHERE is_coliving AND deleted_at IS NULL;

CREATE TABLE app.rooms (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id                UUID NOT NULL REFERENCES app.organizations (id) ON DELETE RESTRICT,
    property_id           UUID NOT NULL REFERENCES app.properties (id) ON DELETE CASCADE,
    name                  TEXT NOT NULL CHECK (length(btrim(name)) BETWEEN 1 AND 100),
    status                app.room_status NOT NULL DEFAULT 'available',
    size_sqm              NUMERIC(8,2) CHECK (size_sqm IS NULL OR size_sqm > 0),
    floor                 SMALLINT,
    has_private_bathroom  BOOLEAN NOT NULL DEFAULT false,
    is_furnished          BOOLEAN,
    monthly_rent          app.money_amount CHECK (monthly_rent IS NULL OR monthly_rent >= 0),
    currency              app.currency_code NOT NULL DEFAULT 'EUR',
    -- quota di ripartizione utenze del co-living (default: equal se NULL)
    utility_share_percent app.percentage,
    metadata              JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at            TIMESTAMPTZ
);
CREATE UNIQUE INDEX rooms_property_name_uk ON app.rooms (property_id, name)
    WHERE deleted_at IS NULL;
CREATE INDEX rooms_org_property_ix ON app.rooms (org_id, property_id) WHERE deleted_at IS NULL;
CREATE INDEX rooms_available_ix ON app.rooms (org_id, status) WHERE deleted_at IS NULL;

-- Coerenza dell'org fra padre e figlio: impedisce il "cross-org stitching"
-- (riga con org_id A che punta a un immobile dell'org B, invisibile alla RLS).
CREATE FUNCTION app.tg_assert_org_matches_property() RETURNS trigger
  LANGUAGE plpgsql SECURITY DEFINER SET search_path = app, pg_temp AS $$
DECLARE v_org uuid;
BEGIN
  SELECT org_id INTO v_org FROM app.properties WHERE id = NEW.property_id;
  IF v_org IS NULL THEN
    RAISE EXCEPTION 'property % not found', NEW.property_id USING ERRCODE = 'foreign_key_violation';
  END IF;
  IF v_org <> NEW.org_id THEN
    RAISE EXCEPTION 'org_id mismatch: % belongs to org %, row declares %',
      NEW.property_id, v_org, NEW.org_id USING ERRCODE = 'check_violation';
  END IF;
  RETURN NEW;
END $$;
