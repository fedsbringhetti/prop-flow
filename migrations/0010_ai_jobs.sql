-- =============================================================================
-- AI: registro job provider-agnostico (OCR bollette, Vision triage ticket, LLM)
-- Nessun riferimento a un vendor specifico: provider_name/model sono dati.
-- =============================================================================

CREATE TABLE app.ai_jobs (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id             UUID NOT NULL REFERENCES app.organizations (id) ON DELETE RESTRICT,
    capability         app.ai_capability NOT NULL,
    -- task applicativo: 'bill_ocr', 'ticket_triage', 'ticket_reply_draft'…
    task               TEXT NOT NULL,
    status             app.ai_job_status NOT NULL DEFAULT 'queued',

    provider_name      TEXT NOT NULL,             -- 'anthropic' | 'openai' | 'tesseract' | 'local'
    model              TEXT,
    provider_version   TEXT,

    -- soggetto polimorfico su cui il job opera
    subject_type       app.attachment_owner_type NOT NULL,
    subject_id         UUID NOT NULL,
    input_document_id  UUID REFERENCES app.documents (id) ON DELETE SET NULL,

    -- Privacy: qui vanno SOLO parametri e riferimenti (mai il prompt con PII,
    -- mai l'immagine). Il payload grezzo del provider resta nello storage oggetti.
    request_params     JSONB NOT NULL DEFAULT '{}'::jsonb,
    output             JSONB NOT NULL DEFAULT '{}'::jsonb,
    confidence         app.confidence,

    tokens_input       INTEGER CHECK (tokens_input IS NULL OR tokens_input >= 0),
    tokens_output      INTEGER CHECK (tokens_output IS NULL OR tokens_output >= 0),
    cost_amount        NUMERIC(12,6) CHECK (cost_amount IS NULL OR cost_amount >= 0),
    currency           app.currency_code NOT NULL DEFAULT 'EUR',
    latency_ms         INTEGER CHECK (latency_ms IS NULL OR latency_ms >= 0),

    attempts           SMALLINT NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    error_code         TEXT,
    error_message      TEXT,
    -- correlazione con il tracing OpenTelemetry
    trace_id           TEXT,
    started_at         TIMESTAMPTZ,
    finished_at        TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT ai_jobs_finished_ck CHECK (
        status <> ALL (ARRAY['succeeded','failed']::app.ai_job_status[]) OR finished_at IS NOT NULL
    )
);
-- coda di lavoro: SELECT ... FOR UPDATE SKIP LOCKED sul worker
CREATE INDEX ai_jobs_queue_ix ON app.ai_jobs (created_at)
    WHERE status = ANY (ARRAY['queued','running']::app.ai_job_status[]);
CREATE INDEX ai_jobs_subject_ix ON app.ai_jobs (subject_type, subject_id, created_at DESC);
CREATE INDEX ai_jobs_org_task_ix ON app.ai_jobs (org_id, task, created_at DESC);
CREATE INDEX ai_jobs_cost_ix ON app.ai_jobs (org_id, created_at) WHERE cost_amount IS NOT NULL;
CREATE INDEX ai_jobs_output_ix ON app.ai_jobs USING gin (output jsonb_path_ops);
CREATE INDEX ai_jobs_document_ix ON app.ai_jobs (input_document_id)
    WHERE input_document_id IS NOT NULL;
CREATE INDEX ai_jobs_trace_ix ON app.ai_jobs (trace_id) WHERE trace_id IS NOT NULL;

-- FK differite verso ai_jobs
ALTER TABLE app.tickets
    ADD CONSTRAINT tickets_ai_job_fk
    FOREIGN KEY (ai_job_id) REFERENCES app.ai_jobs (id) ON DELETE SET NULL;
ALTER TABLE app.ticket_messages
    ADD CONSTRAINT ticket_messages_ai_job_fk
    FOREIGN KEY (ai_job_id) REFERENCES app.ai_jobs (id) ON DELETE SET NULL;
ALTER TABLE app.utility_bills
    ADD CONSTRAINT utility_bills_ai_job_fk
    FOREIGN KEY (ai_job_id) REFERENCES app.ai_jobs (id) ON DELETE SET NULL;
