-- =============================================================================
-- Splitwise integrato: gruppi di spesa co-living, quote, saldi
-- =============================================================================

CREATE TABLE app.expense_groups (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id                UUID NOT NULL REFERENCES app.organizations (id) ON DELETE RESTRICT,
    property_id           UUID REFERENCES app.properties (id) ON DELETE SET NULL,
    kind                  app.expense_group_kind NOT NULL DEFAULT 'coliving',
    name                  TEXT NOT NULL CHECK (length(btrim(name)) BETWEEN 1 AND 120),
    currency              app.currency_code NOT NULL DEFAULT 'EUR',
    default_split_method  app.expense_split_method NOT NULL DEFAULT 'equal',
    created_by            UUID REFERENCES app.users (id) ON DELETE SET NULL,
    is_archived           BOOLEAN NOT NULL DEFAULT false,
    metadata              JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at            TIMESTAMPTZ
);
CREATE INDEX expense_groups_org_ix ON app.expense_groups (org_id, is_archived)
    WHERE deleted_at IS NULL;
CREATE INDEX expense_groups_property_ix ON app.expense_groups (property_id)
    WHERE property_id IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX expense_groups_created_by_ix ON app.expense_groups (created_by)
    WHERE created_by IS NOT NULL;

CREATE TABLE app.expense_group_members (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id        UUID NOT NULL REFERENCES app.organizations (id) ON DELETE RESTRICT,
    group_id      UUID NOT NULL REFERENCES app.expense_groups (id) ON DELETE CASCADE,
    user_id       UUID NOT NULL REFERENCES app.users (id) ON DELETE RESTRICT,
    room_id       UUID REFERENCES app.rooms (id) ON DELETE SET NULL,
    display_name  TEXT,
    -- peso per split 'weighted' (es. stanza doppia = 2)
    weight        NUMERIC(6,3) NOT NULL DEFAULT 1 CHECK (weight > 0),
    joined_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    left_at       TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT expense_group_members_group_user_uk UNIQUE (group_id, user_id),
    CONSTRAINT expense_group_members_dates_ck CHECK (left_at IS NULL OR left_at >= joined_at)
);
-- percorso caldo RLS: "di quali gruppi sono membro attivo?"
CREATE INDEX expense_group_members_user_ix ON app.expense_group_members (user_id, group_id)
    WHERE left_at IS NULL;
CREATE INDEX expense_group_members_org_ix ON app.expense_group_members (org_id);
CREATE INDEX expense_group_members_room_ix ON app.expense_group_members (room_id)
    WHERE room_id IS NOT NULL;

CREATE TABLE app.expenses (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id            UUID NOT NULL REFERENCES app.organizations (id) ON DELETE RESTRICT,
    group_id          UUID NOT NULL REFERENCES app.expense_groups (id) ON DELETE RESTRICT,
    -- chi ha anticipato la spesa
    paid_by_user_id   UUID NOT NULL REFERENCES app.users (id) ON DELETE RESTRICT,
    category          app.expense_category NOT NULL DEFAULT 'other',
    description       TEXT NOT NULL CHECK (length(btrim(description)) BETWEEN 1 AND 300),
    amount            app.money_amount NOT NULL CHECK (amount > 0),
    currency          app.currency_code NOT NULL DEFAULT 'EUR',
    expense_date      DATE NOT NULL DEFAULT CURRENT_DATE,
    split_method      app.expense_split_method NOT NULL DEFAULT 'equal',
    -- bolletta OCR ripartita automaticamente fra i coinquilini
    utility_bill_id   UUID REFERENCES app.utility_bills (id) ON DELETE SET NULL,
    document_id       UUID REFERENCES app.documents (id) ON DELETE SET NULL,
    created_by        UUID REFERENCES app.users (id) ON DELETE SET NULL,
    notes             TEXT,
    metadata          JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at        TIMESTAMPTZ
);
CREATE INDEX expenses_group_date_ix ON app.expenses (group_id, expense_date DESC)
    WHERE deleted_at IS NULL;
CREATE INDEX expenses_org_ix ON app.expenses (org_id, expense_date DESC) WHERE deleted_at IS NULL;
CREATE INDEX expenses_payer_ix ON app.expenses (paid_by_user_id, expense_date DESC)
    WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX expenses_bill_uk ON app.expenses (utility_bill_id)
    WHERE utility_bill_id IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX expenses_document_ix ON app.expenses (document_id) WHERE document_id IS NOT NULL;
CREATE INDEX expenses_created_by_ix ON app.expenses (created_by) WHERE created_by IS NOT NULL;

CREATE TABLE app.expense_shares (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id        UUID NOT NULL REFERENCES app.organizations (id) ON DELETE RESTRICT,
    expense_id    UUID NOT NULL REFERENCES app.expenses (id) ON DELETE CASCADE,
    user_id       UUID NOT NULL REFERENCES app.users (id) ON DELETE RESTRICT,
    share_amount  app.money_amount NOT NULL CHECK (share_amount >= 0),
    share_weight  NUMERIC(6,3),
    is_settled    BOOLEAN NOT NULL DEFAULT false,
    settled_at    TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT expense_shares_expense_user_uk UNIQUE (expense_id, user_id),
    CONSTRAINT expense_shares_settled_ck CHECK (NOT is_settled OR settled_at IS NOT NULL)
);
CREATE INDEX expense_shares_user_open_ix ON app.expense_shares (user_id, expense_id)
    WHERE NOT is_settled;
CREATE INDEX expense_shares_org_ix ON app.expense_shares (org_id);

-- Invariante contabile: la somma delle quote deve essere identica all'importo speso.
-- DEFERRABLE INITIALLY DEFERRED: l'insert di spesa + quote è un'unica transazione.
CREATE FUNCTION app.tg_expense_shares_balanced() RETURNS trigger
  LANGUAGE plpgsql SECURITY DEFINER SET search_path = app, pg_temp AS $$
DECLARE
  v_expense_id uuid := COALESCE(NEW.expense_id, OLD.expense_id);
  v_total app.money_amount;
  v_sum   app.money_amount;
BEGIN
  SELECT amount INTO v_total FROM app.expenses
   WHERE id = v_expense_id AND deleted_at IS NULL;

  IF v_total IS NULL THEN
    RETURN NULL;  -- spesa cancellata/soft-deleted nella stessa transazione
  END IF;

  SELECT COALESCE(SUM(share_amount), 0) INTO v_sum
    FROM app.expense_shares WHERE expense_id = v_expense_id;

  IF v_sum <> v_total THEN
    RAISE EXCEPTION 'expense % shares total % does not match expense amount %',
      v_expense_id, v_sum, v_total USING ERRCODE = 'check_violation';
  END IF;
  RETURN NULL;
END $$;

CREATE CONSTRAINT TRIGGER expense_shares_balanced
    AFTER INSERT OR UPDATE OR DELETE ON app.expense_shares
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION app.tg_expense_shares_balanced();

CREATE TABLE app.settlements (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id         UUID NOT NULL REFERENCES app.organizations (id) ON DELETE RESTRICT,
    group_id       UUID NOT NULL REFERENCES app.expense_groups (id) ON DELETE RESTRICT,
    from_user_id   UUID NOT NULL REFERENCES app.users (id) ON DELETE RESTRICT,
    to_user_id     UUID NOT NULL REFERENCES app.users (id) ON DELETE RESTRICT,
    amount         app.money_amount NOT NULL CHECK (amount > 0),
    currency       app.currency_code NOT NULL DEFAULT 'EUR',
    status         app.settlement_status NOT NULL DEFAULT 'pending',
    method         app.payment_method NOT NULL DEFAULT 'bank_transfer',
    payment_id     UUID REFERENCES app.payments (id) ON DELETE SET NULL,
    note           TEXT,
    settled_at     TIMESTAMPTZ,
    confirmed_at   TIMESTAMPTZ,
    confirmed_by   UUID REFERENCES app.users (id) ON DELETE SET NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT settlements_distinct_parties_ck CHECK (from_user_id <> to_user_id),
    CONSTRAINT settlements_confirmed_ck CHECK (status <> 'confirmed' OR confirmed_at IS NOT NULL)
);
CREATE INDEX settlements_group_ix ON app.settlements (group_id, created_at DESC);
CREATE INDEX settlements_pending_ix ON app.settlements (to_user_id, created_at DESC)
    WHERE status = 'pending';
CREATE INDEX settlements_from_ix ON app.settlements (from_user_id, created_at DESC);
CREATE INDEX settlements_org_ix ON app.settlements (org_id, status);
CREATE INDEX settlements_payment_ix ON app.settlements (payment_id) WHERE payment_id IS NOT NULL;
CREATE INDEX settlements_confirmed_by_ix ON app.settlements (confirmed_by)
    WHERE confirmed_by IS NOT NULL;
