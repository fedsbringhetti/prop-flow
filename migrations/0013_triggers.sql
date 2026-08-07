-- =============================================================================
-- Wiring dei trigger: updated_at, coerenza org, integrità polimorfica, audit
-- =============================================================================

-- updated_at su tutte le tabelle che hanno la colonna: nessuna dimenticanza possibile
DO $$
DECLARE r record;
BEGIN
  FOR r IN
    SELECT c.relname
      FROM pg_class c
      JOIN pg_namespace n ON n.oid = c.relnamespace
      JOIN pg_attribute a ON a.attrelid = c.oid
                         AND a.attname = 'updated_at'
                         AND a.attnum > 0 AND NOT a.attisdropped
     WHERE n.nspname = 'app' AND c.relkind IN ('r', 'p')
     ORDER BY c.relname
  LOOP
    EXECUTE format(
      'CREATE TRIGGER set_updated_at BEFORE UPDATE ON app.%I
         FOR EACH ROW EXECUTE FUNCTION app.tg_set_updated_at()', r.relname);
  END LOOP;
END $$;

-- Coerenza org_id fra figlio e immobile padre (blocca il cross-org stitching)
CREATE TRIGGER assert_org_matches_property BEFORE INSERT OR UPDATE OF org_id, property_id
    ON app.rooms FOR EACH ROW EXECUTE FUNCTION app.tg_assert_org_matches_property();
CREATE TRIGGER assert_org_matches_property BEFORE INSERT OR UPDATE OF org_id, property_id
    ON app.leases FOR EACH ROW EXECUTE FUNCTION app.tg_assert_org_matches_property();
CREATE TRIGGER assert_org_matches_property BEFORE INSERT OR UPDATE OF org_id, property_id
    ON app.tickets FOR EACH ROW EXECUTE FUNCTION app.tg_assert_org_matches_property();
CREATE TRIGGER assert_org_matches_property BEFORE INSERT OR UPDATE OF org_id, property_id
    ON app.utility_bills FOR EACH ROW EXECUTE FUNCTION app.tg_assert_org_matches_property();

-- Integrità dei riferimenti polimorfici (sostituto della FK)
CREATE TRIGGER check_owner_ref BEFORE INSERT OR UPDATE OF owner_type, owner_id
    ON app.documents FOR EACH ROW
    EXECUTE FUNCTION app.tg_check_polymorphic_ref('owner_type', 'owner_id');
CREATE TRIGGER check_payable_ref BEFORE INSERT OR UPDATE OF payable_type, payable_id
    ON app.payments FOR EACH ROW
    EXECUTE FUNCTION app.tg_check_polymorphic_ref('payable_type', 'payable_id');
CREATE TRIGGER check_subject_ref BEFORE INSERT OR UPDATE OF subject_type, subject_id
    ON app.ai_jobs FOR EACH ROW
    EXECUTE FUNCTION app.tg_check_polymorphic_ref('subject_type', 'subject_id');

-- Storico stati ticket
CREATE TRIGGER ticket_status_history AFTER INSERT OR UPDATE OF status
    ON app.tickets FOR EACH ROW EXECUTE FUNCTION app.tg_ticket_status_history();

-- ---------------------------------------------------------------------------
-- Audit sulle operazioni critiche.
-- Escluse per volume/valore: ticket_messages (contenuto utente), user_sessions,
-- ai_jobs, domain_events, ticket_status_history (già append-only e auto-tracciate).
-- ---------------------------------------------------------------------------
CREATE TRIGGER audit AFTER INSERT OR UPDATE OR DELETE ON app.users
    FOR EACH ROW EXECUTE FUNCTION app.tg_audit(
      '{password_hash,mfa_secret_encrypted,fiscal_code_encrypted,fiscal_code_fingerprint,encryption_key_ref}');
CREATE TRIGGER audit AFTER INSERT OR UPDATE OR DELETE ON app.org_invitations
    FOR EACH ROW EXECUTE FUNCTION app.tg_audit('{token_hash}');
CREATE TRIGGER audit AFTER INSERT OR UPDATE OR DELETE ON app.documents
    FOR EACH ROW EXECUTE FUNCTION app.tg_audit('{extracted_data,encryption_key_ref}');
CREATE TRIGGER audit AFTER INSERT OR UPDATE OR DELETE ON app.payments
    FOR EACH ROW EXECUTE FUNCTION app.tg_audit('{provider_payload}');
CREATE TRIGGER audit AFTER INSERT OR UPDATE OR DELETE ON app.utility_bills
    FOR EACH ROW EXECUTE FUNCTION app.tg_audit('{ocr_extracted}');

DO $$
DECLARE r text;
BEGIN
  FOREACH r IN ARRAY ARRAY[
    'organizations', 'org_members', 'properties', 'rooms', 'leases', 'lease_parties',
    'charges', 'expense_groups', 'expense_group_members', 'expenses', 'expense_shares',
    'settlements', 'tickets'
  ] LOOP
    EXECUTE format(
      'CREATE TRIGGER audit AFTER INSERT OR UPDATE OR DELETE ON app.%I
         FOR EACH ROW EXECUTE FUNCTION app.tg_audit()', r);
  END LOOP;
END $$;
