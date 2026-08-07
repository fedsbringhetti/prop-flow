-- =============================================================================
-- Ticket di manutenzione + triage AI Vision + thread di conversazione
-- =============================================================================

CREATE SEQUENCE app.ticket_reference_seq;

CREATE TABLE app.tickets (
    id                     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id                 UUID NOT NULL REFERENCES app.organizations (id) ON DELETE RESTRICT,
    property_id            UUID NOT NULL REFERENCES app.properties (id) ON DELETE RESTRICT,
    room_id                UUID REFERENCES app.rooms (id) ON DELETE SET NULL,
    lease_id               UUID REFERENCES app.leases (id) ON DELETE SET NULL,
    reference              TEXT NOT NULL DEFAULT
        ('TK-' || to_char(nextval('app.ticket_reference_seq'), 'FM000000')),

    title                  TEXT NOT NULL CHECK (length(btrim(title)) BETWEEN 1 AND 200),
    description            TEXT NOT NULL,
    category               app.ticket_category NOT NULL DEFAULT 'other',
    priority               app.ticket_priority NOT NULL DEFAULT 'medium',
    status                 app.ticket_status NOT NULL DEFAULT 'open',

    reported_by            UUID REFERENCES app.users (id) ON DELETE SET NULL,
    reported_by_kind       app.ticket_actor NOT NULL DEFAULT 'tenant',
    -- V2 Marketplace: qui si aggancerà assigned_vendor_id senza toccare il resto
    assigned_to            UUID REFERENCES app.users (id) ON DELETE SET NULL,
    assigned_at            TIMESTAMPTZ,

    -- Triage AI Vision: suggerimenti tracciati a parte dai valori decisi dall'umano,
    -- così si può misurare l'accuratezza del modello senza inquinare il dato di dominio.
    ai_job_id              UUID,                  -- FK aggiunta in 0010
    ai_suggested_category  app.ticket_category,
    ai_suggested_priority  app.ticket_priority,
    ai_confidence          app.confidence,
    ai_analysis            JSONB NOT NULL DEFAULT '{}'::jsonb,

    estimated_cost         app.money_amount CHECK (estimated_cost IS NULL OR estimated_cost >= 0),
    actual_cost            app.money_amount CHECK (actual_cost IS NULL OR actual_cost >= 0),
    currency               app.currency_code NOT NULL DEFAULT 'EUR',

    sla_due_at             TIMESTAMPTZ,
    scheduled_at           TIMESTAMPTZ,
    resolved_at            TIMESTAMPTZ,
    closed_at              TIMESTAMPTZ,
    resolution_notes       TEXT,
    metadata               JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at             TIMESTAMPTZ,

    CONSTRAINT tickets_resolved_ck CHECK (status <> 'resolved' OR resolved_at IS NOT NULL),
    CONSTRAINT tickets_closed_ck   CHECK (status <> 'closed'   OR closed_at   IS NOT NULL),

    search_vector          TSVECTOR GENERATED ALWAYS AS (
        to_tsvector('italian',
            coalesce(reference, '') || ' ' || coalesce(title, '') || ' ' ||
            coalesce(description, '') || ' ' || coalesce(resolution_notes, ''))
    ) STORED
);
CREATE UNIQUE INDEX tickets_reference_uk ON app.tickets (reference);
CREATE INDEX tickets_org_status_ix ON app.tickets (org_id, status, priority DESC, created_at DESC)
    WHERE deleted_at IS NULL;
-- indice parziale sulla coda di lavoro: la dashboard proprietario legge solo questi
CREATE INDEX tickets_open_ix ON app.tickets (org_id, priority DESC, created_at)
    WHERE deleted_at IS NULL
      AND status = ANY (ARRAY['open','triaged','assigned','in_progress',
                              'waiting_tenant','waiting_owner']::app.ticket_status[]);
CREATE INDEX tickets_property_ix ON app.tickets (property_id, created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX tickets_reporter_ix ON app.tickets (reported_by, created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX tickets_assignee_ix ON app.tickets (assigned_to, status)
    WHERE assigned_to IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX tickets_sla_ix ON app.tickets (sla_due_at)
    WHERE sla_due_at IS NOT NULL AND deleted_at IS NULL
      AND status <> ALL (ARRAY['resolved','closed','cancelled']::app.ticket_status[]);
CREATE INDEX tickets_search_ix ON app.tickets USING gin (search_vector);
CREATE INDEX tickets_lease_ix ON app.tickets (lease_id) WHERE lease_id IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX tickets_room_ix ON app.tickets (room_id) WHERE room_id IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX tickets_ai_job_ix ON app.tickets (ai_job_id) WHERE ai_job_id IS NOT NULL;

CREATE TABLE app.ticket_messages (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id        UUID NOT NULL REFERENCES app.organizations (id) ON DELETE RESTRICT,
    ticket_id     UUID NOT NULL REFERENCES app.tickets (id) ON DELETE CASCADE,
    author_id     UUID REFERENCES app.users (id) ON DELETE SET NULL,
    author_kind   app.ticket_actor NOT NULL,
    body          TEXT NOT NULL CHECK (length(btrim(body)) > 0),
    -- messaggio visibile solo allo staff dell'org (note interne proprietario/manager)
    is_internal   BOOLEAN NOT NULL DEFAULT false,
    ai_generated  BOOLEAN NOT NULL DEFAULT false,
    ai_job_id     UUID,                          -- FK aggiunta in 0010
    metadata      JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at    TIMESTAMPTZ
);
CREATE INDEX ticket_messages_thread_ix ON app.ticket_messages (ticket_id, created_at)
    WHERE deleted_at IS NULL;
CREATE INDEX ticket_messages_public_ix ON app.ticket_messages (ticket_id, created_at)
    WHERE NOT is_internal AND deleted_at IS NULL;
-- non parziale: un indice parziale non può servire la verifica della FK su org_id
CREATE INDEX ticket_messages_org_ix ON app.ticket_messages (org_id, created_at DESC);
CREATE INDEX ticket_messages_author_ix ON app.ticket_messages (author_id) WHERE author_id IS NOT NULL;
CREATE INDEX ticket_messages_ai_job_ix ON app.ticket_messages (ai_job_id) WHERE ai_job_id IS NOT NULL;

-- Storico transizioni: append-only, alimentato da trigger (è un fatto osservato,
-- non una decisione: la decisione resta nel Service Layer).
CREATE TABLE app.ticket_status_history (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id       UUID NOT NULL,
    ticket_id    UUID NOT NULL REFERENCES app.tickets (id) ON DELETE CASCADE,
    from_status  app.ticket_status,
    to_status    app.ticket_status NOT NULL,
    changed_by   UUID,
    reason       TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ticket_status_history_ticket_ix ON app.ticket_status_history (ticket_id, created_at);
CREATE INDEX ticket_status_history_org_ix ON app.ticket_status_history (org_id, created_at DESC);

CREATE FUNCTION app.tg_ticket_status_history() RETURNS trigger
  LANGUAGE plpgsql SECURITY DEFINER SET search_path = app, pg_temp AS $$
BEGIN
  IF TG_OP = 'INSERT' THEN
    INSERT INTO app.ticket_status_history (org_id, ticket_id, from_status, to_status, changed_by)
    VALUES (NEW.org_id, NEW.id, NULL, NEW.status, app.current_user_id());
  ELSIF NEW.status IS DISTINCT FROM OLD.status THEN
    INSERT INTO app.ticket_status_history (org_id, ticket_id, from_status, to_status, changed_by)
    VALUES (NEW.org_id, NEW.id, OLD.status, NEW.status, app.current_user_id());
  END IF;
  RETURN NULL;
END $$;
