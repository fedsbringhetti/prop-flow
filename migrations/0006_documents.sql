-- =============================================================================
-- Documenti: allegato polimorfico + storage astratto + predisposizione versioning V2
-- =============================================================================

CREATE TABLE app.documents (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id                  UUID NOT NULL REFERENCES app.organizations (id) ON DELETE RESTRICT,
    kind                    app.document_kind NOT NULL,
    status                  app.document_status NOT NULL DEFAULT 'uploaded',

    -- Riferimento polimorfico: nessuna FK (per design), integrità garantita da
    -- app.tg_check_polymorphic_ref(). Permette di estrarre il modulo Documenti
    -- come microservizio senza dipendenze schema-level verso gli altri aggregati.
    owner_type              app.attachment_owner_type NOT NULL,
    owner_id                UUID NOT NULL,

    -- Storage astratto: nessun vendor lock-in (s3 | minio | gcs | azure | fs)
    storage_provider        TEXT NOT NULL DEFAULT 's3',
    storage_bucket          TEXT NOT NULL,
    object_key              TEXT NOT NULL,
    mime_type               TEXT NOT NULL,
    size_bytes              BIGINT NOT NULL CHECK (size_bytes > 0),
    checksum_sha256         BYTEA NOT NULL CHECK (octet_length(checksum_sha256) = 32),
    original_filename       TEXT NOT NULL,
    is_encrypted            BOOLEAN NOT NULL DEFAULT true,
    encryption_key_ref      TEXT,

    -- Versioning: predisposto ora, esposto dall'API in V2
    version                 INTEGER NOT NULL DEFAULT 1 CHECK (version >= 1),
    supersedes_document_id  UUID REFERENCES app.documents (id) ON DELETE SET NULL,
    is_current              BOOLEAN NOT NULL DEFAULT true,

    -- Output OCR/Vision normalizzato (indicizzato GIN per query per campo estratto)
    extracted_data          JSONB NOT NULL DEFAULT '{}'::jsonb,
    uploaded_by             UUID REFERENCES app.users (id) ON DELETE SET NULL,
    -- retention GDPR: purge job applicativo, nessun trigger di cancellazione
    retention_until         DATE,
    metadata                JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at              TIMESTAMPTZ
);
CREATE UNIQUE INDEX documents_object_uk ON app.documents (storage_provider, storage_bucket, object_key);
CREATE INDEX documents_owner_ix ON app.documents (owner_type, owner_id)
    WHERE deleted_at IS NULL AND is_current;
CREATE INDEX documents_org_kind_ix ON app.documents (org_id, kind, created_at DESC)
    WHERE deleted_at IS NULL;
CREATE INDEX documents_extracted_ix ON app.documents USING gin (extracted_data jsonb_path_ops);
CREATE INDEX documents_checksum_ix ON app.documents (org_id, checksum_sha256) WHERE deleted_at IS NULL;
CREATE INDEX documents_processing_ix ON app.documents (status, created_at)
    WHERE status = ANY (ARRAY['uploaded','processing']::app.document_status[]);
CREATE INDEX documents_retention_ix ON app.documents (retention_until)
    WHERE retention_until IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX documents_supersedes_ix ON app.documents (supersedes_document_id)
    WHERE supersedes_document_id IS NOT NULL;
CREATE INDEX documents_uploaded_by_ix ON app.documents (uploaded_by) WHERE uploaded_by IS NOT NULL;

COMMENT ON COLUMN app.documents.owner_type IS
  'I valori dell''enum corrispondono al nome tabella al singolare: la validazione
   del riferimento è centralizzata in app.tg_check_polymorphic_ref().';

-- FK differite verso documents (dipendenze circolari risolte qui)
ALTER TABLE app.users
    ADD CONSTRAINT users_avatar_document_fk
    FOREIGN KEY (avatar_document_id) REFERENCES app.documents (id) ON DELETE SET NULL;
CREATE INDEX users_avatar_document_ix ON app.users (avatar_document_id)
    WHERE avatar_document_id IS NOT NULL;

ALTER TABLE app.leases
    ADD CONSTRAINT leases_contract_document_fk
    FOREIGN KEY (contract_document_id) REFERENCES app.documents (id) ON DELETE SET NULL;

-- ---------------------------------------------------------------------------
-- Integrità referenziale per riferimenti polimorfici (riusata da payments/ai_jobs).
-- TG_ARGV[0] = colonna del tipo, TG_ARGV[1] = colonna dell'id.
-- ---------------------------------------------------------------------------
CREATE FUNCTION app.tg_check_polymorphic_ref() RETURNS trigger
  LANGUAGE plpgsql SECURITY DEFINER SET search_path = app, pg_temp AS $$
DECLARE
  v_row     jsonb := to_jsonb(NEW);
  v_type    text  := v_row ->> TG_ARGV[0];
  v_id      uuid  := (v_row ->> TG_ARGV[1])::uuid;
  v_table   text;
  v_exists  boolean;
BEGIN
  v_table := CASE v_type
    WHEN 'organization'   THEN 'organizations'
    WHEN 'user'           THEN 'users'
    WHEN 'property'       THEN 'properties'
    WHEN 'room'           THEN 'rooms'
    WHEN 'lease'          THEN 'leases'
    WHEN 'ticket'         THEN 'tickets'
    WHEN 'ticket_message' THEN 'ticket_messages'
    WHEN 'utility_bill'   THEN 'utility_bills'
    WHEN 'charge'         THEN 'charges'
    WHEN 'payment'        THEN 'payments'
    WHEN 'expense'        THEN 'expenses'
    WHEN 'expense_share'  THEN 'expense_shares'
    WHEN 'settlement'     THEN 'settlements'
  END;

  IF v_table IS NULL THEN
    RAISE EXCEPTION 'unmapped polymorphic target type: %', v_type
      USING ERRCODE = 'check_violation';
  END IF;

  EXECUTE format('SELECT EXISTS (SELECT 1 FROM app.%I WHERE id = $1)', v_table)
    INTO v_exists USING v_id;

  IF NOT v_exists THEN
    RAISE EXCEPTION 'polymorphic reference %:% not found', v_type, v_id
      USING ERRCODE = 'foreign_key_violation';
  END IF;
  RETURN NEW;
END $$;

COMMENT ON FUNCTION app.tg_check_polymorphic_ref IS
  'Sostituto della FK per riferimenti polimorfici. SECURITY DEFINER: la verifica di
   esistenza non deve dipendere dalla visibilità RLS del chiamante.';
