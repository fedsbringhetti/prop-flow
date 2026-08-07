-- =============================================================================
-- Test di isolamento RLS + invarianti di dominio.
-- DEVE essere eseguito come propflow_app (soggetto a RLS), su un DB con seed.sql.
--
--   psql -f tests/db/rls_isolation.sql "postgresql://propflow_app:...@host/propflow"
--
-- Ogni assert fallito interrompe lo script con un'eccezione.
-- =============================================================================

\set ON_ERROR_STOP on
\pset pager off

CREATE OR REPLACE FUNCTION pg_temp.assert_eq(p_label text, p_actual bigint, p_expected bigint)
RETURNS void LANGUAGE plpgsql AS $$
BEGIN
  IF p_actual IS DISTINCT FROM p_expected THEN
    RAISE EXCEPTION 'FAIL % → atteso %, ottenuto %', p_label, p_expected, p_actual;
  END IF;
  RAISE NOTICE 'ok   %', p_label;
END $$;

\echo '===== 1. Nessun contesto impostato: tutto invisibile ====='
BEGIN;
DO $$
BEGIN
  PERFORM pg_temp.assert_eq('no-context: properties',    (SELECT count(*) FROM app.properties), 0);
  PERFORM pg_temp.assert_eq('no-context: leases',        (SELECT count(*) FROM app.leases), 0);
  PERFORM pg_temp.assert_eq('no-context: charges',       (SELECT count(*) FROM app.charges), 0);
  PERFORM pg_temp.assert_eq('no-context: tickets',       (SELECT count(*) FROM app.tickets), 0);
  PERFORM pg_temp.assert_eq('no-context: organizations', (SELECT count(*) FROM app.organizations), 0);
END $$;
ROLLBACK;

\echo '===== 2. Owner Alfa: vede tutta la propria org, nulla di Beta ====='
BEGIN;
SELECT app.set_request_context(
  'a1000000-0000-4000-8000-000000000001'::uuid,
  'a0000000-0000-4000-8000-000000000001'::uuid);
DO $$
BEGIN
  PERFORM pg_temp.assert_eq('owner A: properties (2 proprie)', (SELECT count(*) FROM app.properties), 2);
  PERFORM pg_temp.assert_eq('owner A: leases',                 (SELECT count(*) FROM app.leases), 1);
  PERFORM pg_temp.assert_eq('owner A: charges',                (SELECT count(*) FROM app.charges), 1);
  PERFORM pg_temp.assert_eq('owner A: tickets',                (SELECT count(*) FROM app.tickets), 1);
  -- include la nota interna
  PERFORM pg_temp.assert_eq('owner A: ticket_messages',        (SELECT count(*) FROM app.ticket_messages), 2);
  PERFORM pg_temp.assert_eq('owner A: nessun dato Beta',
    (SELECT count(*) FROM app.properties WHERE city = 'Roma'), 0);
END $$;
ROLLBACK;

\echo '===== 3. GUC falsificata: owner Alfa dichiara org Beta ====='
BEGIN;
SELECT app.set_request_context(
  'a1000000-0000-4000-8000-000000000001'::uuid,
  'b0000000-0000-4000-8000-000000000001'::uuid);   -- org di cui NON è membro
DO $$
BEGIN
  PERFORM pg_temp.assert_eq('org spoofing: properties', (SELECT count(*) FROM app.properties), 0);
  PERFORM pg_temp.assert_eq('org spoofing: leases',     (SELECT count(*) FROM app.leases), 0);
  PERFORM pg_temp.assert_eq('org spoofing: charges',    (SELECT count(*) FROM app.charges), 0);
END $$;
ROLLBACK;

\echo '===== 4. Tenant Alfa: solo ciò che è legato al proprio contratto ====='
BEGIN;
SELECT app.set_request_context(
  'a1000000-0000-4000-8000-000000000002'::uuid,
  'a0000000-0000-4000-8000-000000000001'::uuid);
DO $$
BEGIN
  -- vede solo l'immobile in cui abita, non il secondo immobile del proprietario
  PERFORM pg_temp.assert_eq('tenant A: properties (solo il proprio)',
    (SELECT count(*) FROM app.properties), 1);
  PERFORM pg_temp.assert_eq('tenant A: proprio immobile',
    (SELECT count(*) FROM app.properties WHERE label = 'Alfa 1'), 1);
  PERFORM pg_temp.assert_eq('tenant A: leases',  (SELECT count(*) FROM app.leases), 1);
  PERFORM pg_temp.assert_eq('tenant A: charges', (SELECT count(*) FROM app.charges), 1);
  -- la nota interna del proprietario NON è visibile
  PERFORM pg_temp.assert_eq('tenant A: ticket_messages (no note interne)',
    (SELECT count(*) FROM app.ticket_messages), 1);
  PERFORM pg_temp.assert_eq('tenant A: expense_shares',
    (SELECT count(*) FROM app.expense_shares), 2);
  PERFORM pg_temp.assert_eq('tenant A: audit_log invisibile (non è owner)',
    (SELECT count(*) FROM app.audit_log), 0);
END $$;
ROLLBACK;

\echo '===== 5. Tenant Alfa non può modificare il contratto ====='
BEGIN;
SELECT app.set_request_context(
  'a1000000-0000-4000-8000-000000000002'::uuid,
  'a0000000-0000-4000-8000-000000000001'::uuid);
DO $$
DECLARE v_rows bigint;
BEGIN
  UPDATE app.leases SET rent_amount = 1 WHERE id = 'a3000000-0000-4000-8000-000000000001';
  GET DIAGNOSTICS v_rows = ROW_COUNT;
  PERFORM pg_temp.assert_eq('tenant A: UPDATE leases bloccato', v_rows, 0);

  UPDATE app.charges SET amount = 1 WHERE id = 'a4000000-0000-4000-8000-000000000001';
  GET DIAGNOSTICS v_rows = ROW_COUNT;
  PERFORM pg_temp.assert_eq('tenant A: UPDATE charges bloccato', v_rows, 0);
END $$;
ROLLBACK;

\echo '===== 6. Tenant Beta non vede nulla di Alfa ====='
BEGIN;
SELECT app.set_request_context(
  'b1000000-0000-4000-8000-000000000002'::uuid,
  'b0000000-0000-4000-8000-000000000001'::uuid);
DO $$
BEGIN
  PERFORM pg_temp.assert_eq('tenant B: properties', (SELECT count(*) FROM app.properties), 1);
  PERFORM pg_temp.assert_eq('tenant B: nessun immobile Alfa',
    (SELECT count(*) FROM app.properties WHERE city = 'Milano'), 0);
  PERFORM pg_temp.assert_eq('tenant B: tickets', (SELECT count(*) FROM app.tickets), 0);
  PERFORM pg_temp.assert_eq('tenant B: expense_shares', (SELECT count(*) FROM app.expense_shares), 0);
END $$;
ROLLBACK;

\echo '===== 7. Le viste rispettano la RLS del chiamante (security_invoker) ====='
BEGIN;
SELECT app.set_request_context(
  'a1000000-0000-4000-8000-000000000001'::uuid,
  'a0000000-0000-4000-8000-000000000001'::uuid);
DO $$
DECLARE v_outstanding numeric;
BEGIN
  -- 800 addebitati - 300 pagati = 500 residui
  SELECT outstanding_amount INTO v_outstanding FROM app.v_charge_balances
   WHERE charge_id = 'a4000000-0000-4000-8000-000000000001';
  IF v_outstanding <> 500.00 THEN
    RAISE EXCEPTION 'FAIL v_charge_balances → atteso 500.00, ottenuto %', v_outstanding;
  END IF;
  RAISE NOTICE 'ok   owner A: v_charge_balances.outstanding = 500.00';

  PERFORM pg_temp.assert_eq('owner A: v_owner_dashboard (solo la propria org)',
    (SELECT count(*) FROM app.v_owner_dashboard), 1);
  PERFORM pg_temp.assert_eq('owner A: v_property_occupancy',
    (SELECT count(*) FROM app.v_property_occupancy), 2);
END $$;
ROLLBACK;

BEGIN;
SELECT app.set_request_context(
  'b1000000-0000-4000-8000-000000000002'::uuid,
  'b0000000-0000-4000-8000-000000000001'::uuid);
DO $$
BEGIN
  PERFORM pg_temp.assert_eq('tenant B: v_charge_balances non vede Alfa',
    (SELECT count(*) FROM app.v_charge_balances
      WHERE charge_id = 'a4000000-0000-4000-8000-000000000001'), 0);
END $$;
ROLLBACK;

\echo '===== 8. Splitwise: saldi netti coerenti ====='
BEGIN;
SELECT app.set_request_context(
  'a1000000-0000-4000-8000-000000000002'::uuid,
  'a0000000-0000-4000-8000-000000000001'::uuid);
DO $$
DECLARE v_net numeric;
BEGIN
  -- tenant A ha anticipato 100 e ne deve 50 → credito netto +50
  SELECT net_balance INTO v_net FROM app.v_expense_group_balances
   WHERE user_id = 'a1000000-0000-4000-8000-000000000002';
  IF v_net <> 50.00 THEN
    RAISE EXCEPTION 'FAIL net_balance payer → atteso 50.00, ottenuto %', v_net;
  END IF;
  RAISE NOTICE 'ok   splitwise: net_balance payer = +50.00';

  SELECT net_balance INTO v_net FROM app.v_expense_group_balances
   WHERE user_id = 'a1000000-0000-4000-8000-000000000001';
  IF v_net <> -50.00 THEN
    RAISE EXCEPTION 'FAIL net_balance debitor → atteso -50.00, ottenuto %', v_net;
  END IF;
  RAISE NOTICE 'ok   splitwise: net_balance debitor = -50.00';
END $$;
ROLLBACK;

\echo '===== 9. DELETE revocato al ruolo applicativo (soft delete obbligatorio) ====='
BEGIN;
SELECT app.set_request_context(
  'a1000000-0000-4000-8000-000000000001'::uuid,
  'a0000000-0000-4000-8000-000000000001'::uuid);
DO $$
BEGIN
  BEGIN
    DELETE FROM app.properties WHERE id = 'a2000000-0000-4000-8000-000000000001';
    RAISE EXCEPTION 'FAIL: DELETE su properties è riuscito, doveva essere negato';
  EXCEPTION WHEN insufficient_privilege THEN
    RAISE NOTICE 'ok   DELETE su properties negato dai privilegi';
  END;
END $$;
ROLLBACK;

\echo '===== 10. Invarianti di dominio ====='
BEGIN;
SELECT app.set_request_context(
  'a1000000-0000-4000-8000-000000000001'::uuid,
  'a0000000-0000-4000-8000-000000000001'::uuid);

-- 10a. doppia locazione sovrapposta sullo stesso immobile
DO $$
BEGIN
  BEGIN
    INSERT INTO app.leases (org_id, property_id, lease_type, status, start_date, end_date, rent_amount)
    VALUES ('a0000000-0000-4000-8000-000000000001', 'a2000000-0000-4000-8000-000000000001',
            'whole_property', 'active', CURRENT_DATE, CURRENT_DATE + 100, 700.00);
    RAISE EXCEPTION 'FAIL: locazione sovrapposta accettata';
  EXCEPTION WHEN exclusion_violation THEN
    RAISE NOTICE 'ok   locazione sovrapposta rifiutata (EXCLUDE)';
  END;
END $$;
ROLLBACK;

-- 10b. quote Splitwise che non sommano all'importo speso
BEGIN;
SELECT app.set_request_context(
  'a1000000-0000-4000-8000-000000000002'::uuid,
  'a0000000-0000-4000-8000-000000000001'::uuid);
DO $$
DECLARE v_expense uuid;
BEGIN
  INSERT INTO app.expenses (org_id, group_id, paid_by_user_id, description, amount)
  VALUES ('a0000000-0000-4000-8000-000000000001', 'a6000000-0000-4000-8000-000000000001',
          'a1000000-0000-4000-8000-000000000002', 'Spesa sbilanciata', 100.00)
  RETURNING id INTO v_expense;

  INSERT INTO app.expense_shares (org_id, expense_id, user_id, share_amount) VALUES
    ('a0000000-0000-4000-8000-000000000001', v_expense,
     'a1000000-0000-4000-8000-000000000002', 40.00),
    ('a0000000-0000-4000-8000-000000000001', v_expense,
     'a1000000-0000-4000-8000-000000000001', 40.00);

  -- il constraint trigger è DEFERRED: scatta qui, non all'INSERT
  BEGIN
    SET CONSTRAINTS ALL IMMEDIATE;
    RAISE EXCEPTION 'FAIL: quote sbilanciate accettate';
  EXCEPTION WHEN check_violation THEN
    RAISE NOTICE 'ok   quote Splitwise sbilanciate rifiutate (80 <> 100)';
  END;
END $$;
ROLLBACK;

-- 10c. riferimento polimorfico inesistente
BEGIN;
SELECT app.set_request_context(
  'a1000000-0000-4000-8000-000000000001'::uuid,
  'a0000000-0000-4000-8000-000000000001'::uuid);
DO $$
BEGIN
  BEGIN
    INSERT INTO app.payments (org_id, payable_type, payable_id, amount, idempotency_key)
    VALUES ('a0000000-0000-4000-8000-000000000001', 'charge',
            '00000000-0000-4000-8000-0000000000ff', 10.00, 'test-orphan-payable');
    RAISE EXCEPTION 'FAIL: payable inesistente accettato';
  EXCEPTION WHEN foreign_key_violation THEN
    RAISE NOTICE 'ok   payable polimorfico inesistente rifiutato';
  END;
END $$;
ROLLBACK;

-- 10d. cross-org stitching
BEGIN;
SELECT app.set_request_context(
  'a1000000-0000-4000-8000-000000000001'::uuid,
  'a0000000-0000-4000-8000-000000000001'::uuid);
DO $$
BEGIN
  BEGIN
    INSERT INTO app.rooms (org_id, property_id, name)
    VALUES ('a0000000-0000-4000-8000-000000000001',
            'b2000000-0000-4000-8000-000000000001', 'Stanza rubata');
    RAISE EXCEPTION 'FAIL: cross-org stitching accettato';
  EXCEPTION WHEN check_violation THEN
    RAISE NOTICE 'ok   cross-org stitching rifiutato';
  END;
END $$;
ROLLBACK;

-- 10e. idempotenza dei pagamenti
BEGIN;
SELECT app.set_request_context(
  'a1000000-0000-4000-8000-000000000001'::uuid,
  'a0000000-0000-4000-8000-000000000001'::uuid);
DO $$
BEGIN
  BEGIN
    INSERT INTO app.payments (org_id, payable_type, payable_id, amount, idempotency_key)
    VALUES ('a0000000-0000-4000-8000-000000000001', 'charge',
            'a4000000-0000-4000-8000-000000000001', 10.00, 'seed-pay-alfa-1');
    RAISE EXCEPTION 'FAIL: idempotency_key duplicata accettata';
  EXCEPTION WHEN unique_violation THEN
    RAISE NOTICE 'ok   idempotency_key duplicata rifiutata';
  END;
END $$;
ROLLBACK;

\echo '===== 11. Audit log: redazione e immutabilità ====='
BEGIN;
SELECT app.set_request_context(
  'a1000000-0000-4000-8000-000000000001'::uuid,
  'a0000000-0000-4000-8000-000000000001'::uuid);
DO $$
DECLARE v_new jsonb;
BEGIN
  UPDATE app.users SET full_name = 'Owner Alfa (rinominato)'
   WHERE id = 'a1000000-0000-4000-8000-000000000001';

  SELECT new_data INTO v_new FROM app.audit_log
   WHERE table_name = 'users' AND action = 'update'
     AND record_id = 'a1000000-0000-4000-8000-000000000001'
   ORDER BY occurred_at DESC LIMIT 1;

  IF v_new IS NULL THEN
    RAISE EXCEPTION 'FAIL: nessuna riga di audit per UPDATE users';
  END IF;
  IF v_new ->> 'password_hash' <> '[REDACTED]' THEN
    RAISE EXCEPTION 'FAIL: password_hash non redatto → %', v_new ->> 'password_hash';
  END IF;
  IF v_new ->> 'full_name' <> 'Owner Alfa (rinominato)' THEN
    RAISE EXCEPTION 'FAIL: full_name non tracciato → %', v_new ->> 'full_name';
  END IF;
  RAISE NOTICE 'ok   audit: full_name tracciato, password_hash redatto';

  BEGIN
    UPDATE app.audit_log SET action = 'insert' WHERE table_name = 'users';
    RAISE EXCEPTION 'FAIL: UPDATE su audit_log riuscito';
  EXCEPTION WHEN insufficient_privilege THEN
    RAISE NOTICE 'ok   audit_log immutabile';
  END;
END $$;
ROLLBACK;

\echo '===== TUTTI I TEST SUPERATI ====='
