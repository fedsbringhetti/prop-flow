-- =============================================================================
-- Contratti di locazione (Contratto = Lease) e parti contrattuali
-- =============================================================================

CREATE TABLE app.leases (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id               UUID NOT NULL REFERENCES app.organizations (id) ON DELETE RESTRICT,
    property_id          UUID NOT NULL REFERENCES app.properties (id) ON DELETE RESTRICT,
    room_id              UUID REFERENCES app.rooms (id) ON DELETE RESTRICT,
    lease_type           app.lease_type NOT NULL,
    status               app.lease_status NOT NULL DEFAULT 'draft',

    start_date           DATE NOT NULL,
    end_date             DATE,                   -- NULL = a tempo indeterminato
    notice_months        SMALLINT CHECK (notice_months IS NULL OR notice_months BETWEEN 0 AND 24),
    auto_renew           BOOLEAN NOT NULL DEFAULT false,
    renewal_months       SMALLINT CHECK (renewal_months IS NULL OR renewal_months > 0),

    rent_amount          app.money_amount NOT NULL CHECK (rent_amount >= 0),
    currency             app.currency_code NOT NULL DEFAULT 'EUR',
    rent_frequency       app.rent_frequency NOT NULL DEFAULT 'monthly',
    rent_due_day         SMALLINT NOT NULL DEFAULT 1 CHECK (rent_due_day BETWEEN 1 AND 28),
    deposit_amount       app.money_amount NOT NULL DEFAULT 0 CHECK (deposit_amount >= 0),
    utilities_included   BOOLEAN NOT NULL DEFAULT false,
    istat_indexation     BOOLEAN NOT NULL DEFAULT false,

    -- V3 Agenzia delle Entrate: campi presenti, nessuna integrazione nell'MVP
    registration_number  TEXT,
    registered_at        DATE,

    signed_at            TIMESTAMPTZ,
    terminated_at        TIMESTAMPTZ,
    termination_reason   TEXT,
    contract_document_id UUID,                   -- FK aggiunta in 0006
    notes                TEXT,
    metadata             JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at           TIMESTAMPTZ,

    CONSTRAINT leases_period_ck CHECK (end_date IS NULL OR end_date > start_date),
    -- il tipo di contratto determina la presenza di room_id: nessuno stato ambiguo
    CONSTRAINT leases_target_ck CHECK (
        (lease_type = 'room'           AND room_id IS NOT NULL) OR
        (lease_type = 'whole_property' AND room_id IS NULL)
    ),
    CONSTRAINT leases_terminated_ck CHECK (
        (status <> 'terminated') OR (terminated_at IS NOT NULL)
    ),

    -- Invariante di dominio: nessuna doppia locazione attiva sullo stesso oggetto
    -- nello stesso intervallo. Applicata dal DB perché è integrità, non workflow.
    CONSTRAINT leases_no_overlap_property EXCLUDE USING gist (
        property_id WITH =,
        daterange(start_date, COALESCE(end_date, 'infinity'::date), '[)') WITH &&
    ) WHERE (
        room_id IS NULL AND deleted_at IS NULL
        AND status = ANY (ARRAY['pending_signature','active','expiring']::app.lease_status[])
    ),
    CONSTRAINT leases_no_overlap_room EXCLUDE USING gist (
        room_id WITH =,
        daterange(start_date, COALESCE(end_date, 'infinity'::date), '[)') WITH &&
    ) WHERE (
        room_id IS NOT NULL AND deleted_at IS NULL
        AND status = ANY (ARRAY['pending_signature','active','expiring']::app.lease_status[])
    )
);
CREATE INDEX leases_org_status_ix ON app.leases (org_id, status) WHERE deleted_at IS NULL;
CREATE INDEX leases_property_ix   ON app.leases (property_id, start_date DESC) WHERE deleted_at IS NULL;
CREATE INDEX leases_room_ix       ON app.leases (room_id)
    WHERE room_id IS NOT NULL AND deleted_at IS NULL;
-- scheduler scadenze/rinnovi (dashboard "contratti in scadenza")
CREATE INDEX leases_expiring_ix   ON app.leases (org_id, end_date)
    WHERE end_date IS NOT NULL AND deleted_at IS NULL
      AND status = ANY (ARRAY['active','expiring']::app.lease_status[]);
CREATE UNIQUE INDEX leases_registration_uk ON app.leases (org_id, registration_number)
    WHERE registration_number IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX leases_contract_document_ix ON app.leases (contract_document_id)
    WHERE contract_document_id IS NOT NULL;

CREATE TABLE app.lease_parties (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id         UUID NOT NULL REFERENCES app.organizations (id) ON DELETE RESTRICT,
    lease_id       UUID NOT NULL REFERENCES app.leases (id) ON DELETE CASCADE,
    -- NULL = inquilino invitato ma non ancora registrato (onboarding asincrono)
    user_id        UUID REFERENCES app.users (id) ON DELETE RESTRICT,
    role           app.lease_party_role NOT NULL DEFAULT 'tenant',
    full_name      TEXT NOT NULL,
    email          app.email_address,
    phone          TEXT,
    share_percent  app.percentage NOT NULL DEFAULT 100,
    is_primary     BOOLEAN NOT NULL DEFAULT false,
    joined_at      DATE,
    left_at        DATE,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at     TIMESTAMPTZ,
    CONSTRAINT lease_parties_dates_ck CHECK (left_at IS NULL OR joined_at IS NULL OR left_at >= joined_at)
);
CREATE UNIQUE INDEX lease_parties_lease_user_uk ON app.lease_parties (lease_id, user_id)
    WHERE user_id IS NOT NULL AND deleted_at IS NULL;
-- un solo intestatario principale per contratto
CREATE UNIQUE INDEX lease_parties_primary_uk ON app.lease_parties (lease_id)
    WHERE is_primary AND deleted_at IS NULL;
-- percorso caldo RLS: "di quali contratti sono parte?"
CREATE INDEX lease_parties_user_ix ON app.lease_parties (user_id, lease_id)
    WHERE user_id IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX lease_parties_org_ix  ON app.lease_parties (org_id) WHERE deleted_at IS NULL;

-- FK differita dichiarata in 0003
ALTER TABLE app.org_invitations
    ADD CONSTRAINT org_invitations_lease_fk
    FOREIGN KEY (lease_id) REFERENCES app.leases (id) ON DELETE CASCADE;
CREATE INDEX org_invitations_lease_ix ON app.org_invitations (lease_id)
    WHERE lease_id IS NOT NULL;
