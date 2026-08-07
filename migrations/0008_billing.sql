-- =============================================================================
-- Bollette (OCR), addebiti verso l'inquilino, pagamenti polimorfici
-- =============================================================================

CREATE TABLE app.utility_bills (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id           UUID NOT NULL REFERENCES app.organizations (id) ON DELETE RESTRICT,
    property_id      UUID NOT NULL REFERENCES app.properties (id) ON DELETE RESTRICT,
    room_id          UUID REFERENCES app.rooms (id) ON DELETE SET NULL,
    lease_id         UUID REFERENCES app.leases (id) ON DELETE SET NULL,

    utility_type     app.utility_type NOT NULL,
    status           app.bill_status NOT NULL DEFAULT 'draft',
    supplier_name    TEXT,
    supplier_vat     TEXT,
    invoice_number   TEXT,
    issue_date       DATE,
    due_date         DATE,
    period_start     DATE,
    period_end       DATE,

    total_amount     app.money_amount NOT NULL CHECK (total_amount >= 0),
    taxable_amount   app.money_amount CHECK (taxable_amount IS NULL OR taxable_amount >= 0),
    vat_amount       app.money_amount CHECK (vat_amount IS NULL OR vat_amount >= 0),
    currency         app.currency_code NOT NULL DEFAULT 'EUR',
    consumption      NUMERIC(14,4) CHECK (consumption IS NULL OR consumption >= 0),
    consumption_unit TEXT,                        -- kWh, m3, Smc…

    document_id      UUID REFERENCES app.documents (id) ON DELETE SET NULL,
    -- Tracciamento OCR: confidenza + payload normalizzato per la review umana
    ai_job_id        UUID,                        -- FK aggiunta in 0010
    ocr_confidence   app.confidence,
    ocr_extracted    JSONB NOT NULL DEFAULT '{}'::jsonb,
    validated_by     UUID REFERENCES app.users (id) ON DELETE SET NULL,
    validated_at     TIMESTAMPTZ,

    notes            TEXT,
    metadata         JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at       TIMESTAMPTZ,

    CONSTRAINT utility_bills_period_ck CHECK (period_end IS NULL OR period_start IS NULL
                                              OR period_end >= period_start),
    CONSTRAINT utility_bills_validated_ck CHECK (
        (status <> ALL (ARRAY['validated','allocated','paid']::app.bill_status[]))
        OR validated_at IS NOT NULL
    )
);
-- deduplicazione OCR: la stessa fattura caricata due volte viene rifiutata dal DB
CREATE UNIQUE INDEX utility_bills_dedup_uk ON app.utility_bills
    (org_id, supplier_vat, invoice_number)
    WHERE supplier_vat IS NOT NULL AND invoice_number IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX utility_bills_org_status_ix ON app.utility_bills (org_id, status, due_date)
    WHERE deleted_at IS NULL;
CREATE INDEX utility_bills_property_period_ix ON app.utility_bills
    (property_id, utility_type, period_start DESC) WHERE deleted_at IS NULL;
CREATE INDEX utility_bills_review_ix ON app.utility_bills (org_id, created_at)
    WHERE status = ANY (ARRAY['draft','pending_review']::app.bill_status[]) AND deleted_at IS NULL;
CREATE INDEX utility_bills_ocr_ix ON app.utility_bills USING gin (ocr_extracted jsonb_path_ops);
CREATE INDEX utility_bills_lease_ix ON app.utility_bills (lease_id) WHERE lease_id IS NOT NULL;
CREATE INDEX utility_bills_room_ix ON app.utility_bills (room_id) WHERE room_id IS NOT NULL;
CREATE INDEX utility_bills_document_ix ON app.utility_bills (document_id) WHERE document_id IS NOT NULL;
CREATE INDEX utility_bills_validated_by_ix ON app.utility_bills (validated_by)
    WHERE validated_by IS NOT NULL;
CREATE INDEX utility_bills_ai_job_ix ON app.utility_bills (ai_job_id) WHERE ai_job_id IS NOT NULL;

CREATE TABLE app.charges (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id           UUID NOT NULL REFERENCES app.organizations (id) ON DELETE RESTRICT,
    lease_id         UUID NOT NULL REFERENCES app.leases (id) ON DELETE RESTRICT,
    charge_type      app.charge_type NOT NULL,
    status           app.charge_status NOT NULL DEFAULT 'draft',
    description      TEXT,
    period_start     DATE,
    period_end       DATE,
    amount           app.money_amount NOT NULL CHECK (amount >= 0),
    currency         app.currency_code NOT NULL DEFAULT 'EUR',
    due_date         DATE NOT NULL,
    issued_at        TIMESTAMPTZ,
    -- riaddebito di una bolletta all'inquilino
    utility_bill_id  UUID REFERENCES app.utility_bills (id) ON DELETE SET NULL,
    document_id      UUID REFERENCES app.documents (id) ON DELETE SET NULL,
    -- idempotenza del generatore canoni ricorrenti (una riga per periodo/tipo)
    generation_key   TEXT,
    notes            TEXT,
    metadata         JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at       TIMESTAMPTZ,
    CONSTRAINT charges_period_ck CHECK (period_end IS NULL OR period_start IS NULL
                                        OR period_end >= period_start),
    CONSTRAINT charges_issued_ck CHECK (status = 'draft' OR issued_at IS NOT NULL)
);
CREATE UNIQUE INDEX charges_generation_uk ON app.charges (lease_id, generation_key)
    WHERE generation_key IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX charges_org_due_ix ON app.charges (org_id, status, due_date) WHERE deleted_at IS NULL;
CREATE INDEX charges_lease_ix ON app.charges (lease_id, due_date DESC) WHERE deleted_at IS NULL;
-- scadenzario/solleciti: la dashboard legge solo gli addebiti aperti
CREATE INDEX charges_outstanding_ix ON app.charges (org_id, due_date)
    WHERE deleted_at IS NULL
      AND status = ANY (ARRAY['issued','partially_paid','overdue']::app.charge_status[]);
CREATE INDEX charges_bill_ix ON app.charges (utility_bill_id) WHERE utility_bill_id IS NOT NULL;
CREATE INDEX charges_document_ix ON app.charges (document_id) WHERE document_id IS NOT NULL;

COMMENT ON TABLE app.charges IS
  'Partita a debito dell''inquilino (canone, deposito, riaddebito utenze). Lo stato è
   calcolato e scritto dal Service Layer; il saldo residuo si legge da app.v_charge_balances.';

CREATE TABLE app.payments (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id               UUID NOT NULL REFERENCES app.organizations (id) ON DELETE RESTRICT,

    -- Polimorfico: un pagamento salda un addebito, una quota Splitwise o un settlement.
    -- Validato da app.tg_check_polymorphic_ref(); nessun ALTER TABLE per nuovi payable.
    payable_type         app.payable_type NOT NULL,
    payable_id           UUID NOT NULL,

    payer_user_id        UUID REFERENCES app.users (id) ON DELETE SET NULL,
    payee_user_id        UUID REFERENCES app.users (id) ON DELETE SET NULL,
    amount               app.money_amount NOT NULL CHECK (amount > 0),
    currency             app.currency_code NOT NULL DEFAULT 'EUR',
    method               app.payment_method NOT NULL DEFAULT 'bank_transfer',
    status               app.payment_status NOT NULL DEFAULT 'pending',

    -- Provider astratto: 'manual' nell'MVP, 'stripe'/'gocardless' in V2 senza migrazione
    provider             TEXT NOT NULL DEFAULT 'manual',
    provider_payment_id  TEXT,
    provider_payload     JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- idempotenza end-to-end (retry HTTP e webhook duplicati)
    idempotency_key      TEXT NOT NULL,

    reference            TEXT,                    -- causale / bank reference
    paid_at              TIMESTAMPTZ,
    reconciled_at        TIMESTAMPTZ,
    failure_code         TEXT,
    failure_message      TEXT,
    refunded_amount      app.money_amount NOT NULL DEFAULT 0 CHECK (refunded_amount >= 0),
    metadata             JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT payments_refund_ck CHECK (refunded_amount <= amount),
    CONSTRAINT payments_succeeded_ck CHECK (status <> 'succeeded' OR paid_at IS NOT NULL)
);
CREATE UNIQUE INDEX payments_idempotency_uk ON app.payments (org_id, idempotency_key);
CREATE UNIQUE INDEX payments_provider_uk ON app.payments (provider, provider_payment_id)
    WHERE provider_payment_id IS NOT NULL;
CREATE INDEX payments_payable_ix ON app.payments (payable_type, payable_id)
    WHERE status = ANY (ARRAY['authorized','succeeded','partially_refunded']::app.payment_status[]);
CREATE INDEX payments_org_status_ix ON app.payments (org_id, status, created_at DESC);
CREATE INDEX payments_payer_ix ON app.payments (payer_user_id, created_at DESC)
    WHERE payer_user_id IS NOT NULL;
CREATE INDEX payments_payee_ix ON app.payments (payee_user_id, created_at DESC)
    WHERE payee_user_id IS NOT NULL;
CREATE INDEX payments_unreconciled_ix ON app.payments (org_id, paid_at)
    WHERE status = 'succeeded' AND reconciled_at IS NULL;

-- Nessun trigger che aggiorni charges.status dai pagamenti: sarebbe workflow nel DB.
-- Il saldo si legge dalla vista v_charge_balances, lo stato lo scrive il Service Layer.
