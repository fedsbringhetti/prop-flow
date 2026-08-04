-- =============================================================================
-- Viste di sola lettura per la Dashboard + privilegi.
--
-- security_invoker = true (PG15+): la vista applica la RLS del CHIAMANTE.
-- Senza questo flag le viste girerebbero con i diritti dell'owner, bucando la RLS.
-- Le viste NON contengono regole di business: solo aggregazioni derivabili.
-- =============================================================================

-- Saldo residuo di un addebito: derivato dai pagamenti riusciti
CREATE VIEW app.v_charge_balances WITH (security_invoker = true) AS
SELECT c.id                AS charge_id,
       c.org_id,
       c.lease_id,
       c.charge_type,
       c.status,
       c.due_date,
       c.currency,
       c.amount,
       COALESCE(p.paid_amount, 0)::app.money_amount              AS paid_amount,
       (c.amount - COALESCE(p.paid_amount, 0))::app.money_amount AS outstanding_amount,
       (c.due_date < CURRENT_DATE
        AND c.amount > COALESCE(p.paid_amount, 0))               AS is_past_due
  FROM app.charges c
  LEFT JOIN LATERAL (
        SELECT SUM(pm.amount - pm.refunded_amount) AS paid_amount
          FROM app.payments pm
         WHERE pm.payable_type = 'charge'
           AND pm.payable_id = c.id
           AND pm.status = ANY (ARRAY['succeeded','partially_refunded']::app.payment_status[])
       ) p ON true
 WHERE c.deleted_at IS NULL;

-- Saldo netto per membro di un gruppo Splitwise:
-- (anticipato) - (dovuto) + (settlement in uscita) - (settlement in entrata)
CREATE VIEW app.v_expense_group_balances WITH (security_invoker = true) AS
WITH paid AS (
    SELECT e.group_id, e.paid_by_user_id AS user_id, SUM(e.amount) AS amount
      FROM app.expenses e
     WHERE e.deleted_at IS NULL
     GROUP BY 1, 2
), owed AS (
    SELECT e.group_id, s.user_id, SUM(s.share_amount) AS amount
      FROM app.expense_shares s
      JOIN app.expenses e ON e.id = s.expense_id AND e.deleted_at IS NULL
     GROUP BY 1, 2
), settled_out AS (
    SELECT st.group_id, st.from_user_id AS user_id, SUM(st.amount) AS amount
      FROM app.settlements st WHERE st.status = 'confirmed' GROUP BY 1, 2
), settled_in AS (
    SELECT st.group_id, st.to_user_id AS user_id, SUM(st.amount) AS amount
      FROM app.settlements st WHERE st.status = 'confirmed' GROUP BY 1, 2
)
SELECT m.org_id,
       m.group_id,
       m.user_id,
       COALESCE(paid.amount, 0)::app.money_amount        AS total_paid,
       COALESCE(owed.amount, 0)::app.money_amount        AS total_owed,
       COALESCE(settled_out.amount, 0)::app.money_amount AS total_settled_out,
       COALESCE(settled_in.amount, 0)::app.money_amount  AS total_settled_in,
       ( COALESCE(paid.amount, 0) - COALESCE(owed.amount, 0)
         + COALESCE(settled_out.amount, 0) - COALESCE(settled_in.amount, 0)
       )::app.money_amount                               AS net_balance
  FROM app.expense_group_members m
  LEFT JOIN paid        ON paid.group_id        = m.group_id AND paid.user_id        = m.user_id
  LEFT JOIN owed        ON owed.group_id        = m.group_id AND owed.user_id        = m.user_id
  LEFT JOIN settled_out ON settled_out.group_id = m.group_id AND settled_out.user_id = m.user_id
  LEFT JOIN settled_in  ON settled_in.group_id  = m.group_id AND settled_in.user_id  = m.user_id
 WHERE m.left_at IS NULL;

-- Occupazione immobile (co-living: per stanza; intero: per contratto)
CREATE VIEW app.v_property_occupancy WITH (security_invoker = true) AS
SELECT p.id AS property_id,
       p.org_id,
       p.is_coliving,
       count(r.id)                                            AS rooms_total,
       count(r.id) FILTER (WHERE r.status = 'occupied')        AS rooms_occupied,
       CASE WHEN count(r.id) > 0
            THEN round(100.0 * count(r.id) FILTER (WHERE r.status = 'occupied')
                       / count(r.id), 2)
       END                                                    AS occupancy_percent,
       count(DISTINCT l.id) FILTER (WHERE l.status = 'active') AS active_leases,
       SUM(l.rent_amount) FILTER (WHERE l.status = 'active')::app.money_amount
                                                              AS active_monthly_rent
  FROM app.properties p
  LEFT JOIN app.rooms  r ON r.property_id = p.id AND r.deleted_at IS NULL
  LEFT JOIN app.leases l ON l.property_id = p.id AND l.deleted_at IS NULL
 WHERE p.deleted_at IS NULL
 GROUP BY p.id, p.org_id, p.is_coliving;

-- Riepilogo per la dashboard proprietario
CREATE VIEW app.v_owner_dashboard WITH (security_invoker = true) AS
SELECT o.id AS org_id,
       (SELECT count(*) FROM app.properties p
         WHERE p.org_id = o.id AND p.deleted_at IS NULL)                    AS properties_count,
       (SELECT count(*) FROM app.leases l
         WHERE l.org_id = o.id AND l.deleted_at IS NULL AND l.status = 'active') AS active_leases,
       (SELECT COALESCE(SUM(l.rent_amount), 0) FROM app.leases l
         WHERE l.org_id = o.id AND l.deleted_at IS NULL AND l.status = 'active'
       )::app.money_amount                                                  AS monthly_rent_expected,
       (SELECT COALESCE(SUM(b.outstanding_amount), 0) FROM app.v_charge_balances b
         WHERE b.org_id = o.id AND b.outstanding_amount > 0
       )::app.money_amount                                                  AS total_outstanding,
       (SELECT count(*) FROM app.v_charge_balances b
         WHERE b.org_id = o.id AND b.is_past_due)                           AS overdue_charges,
       (SELECT count(*) FROM app.tickets t
         WHERE t.org_id = o.id AND t.deleted_at IS NULL
           AND t.status <> ALL (ARRAY['resolved','closed','cancelled']::app.ticket_status[])
       )                                                                    AS open_tickets,
       (SELECT count(*) FROM app.leases l
         WHERE l.org_id = o.id AND l.deleted_at IS NULL
           AND l.status = ANY (ARRAY['active','expiring']::app.lease_status[])
           AND l.end_date BETWEEN CURRENT_DATE AND CURRENT_DATE + INTERVAL '90 days'
       )                                                                    AS leases_expiring_90d,
       (SELECT count(*) FROM app.utility_bills ub
         WHERE ub.org_id = o.id AND ub.deleted_at IS NULL
           AND ub.status = ANY (ARRAY['draft','pending_review']::app.bill_status[])
       )                                                                    AS bills_pending_review
  FROM app.organizations o
 WHERE o.deleted_at IS NULL;

-- ---------------------------------------------------------------------------
-- Privilegi.
-- I ruoli sono creati da ops (CREATE ROLE è cluster-wide, non appartiene a una
-- migration di schema); la migration è idempotente rispetto alla loro esistenza.
-- ---------------------------------------------------------------------------
DO $$
DECLARE r record;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_app') THEN
    RAISE NOTICE 'role propflow_app missing: skipping grants (dev/test environment)';
    RETURN;
  END IF;

  EXECUTE 'GRANT USAGE ON SCHEMA app TO propflow_app';
  -- DELETE deliberatamente NON concesso: la cancellazione è soft (deleted_at)
  EXECUTE 'GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA app TO propflow_app';
  EXECUTE 'GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA app TO propflow_app';
  EXECUTE 'GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA app TO propflow_app';
  EXECUTE 'ALTER DEFAULT PRIVILEGES IN SCHEMA app
             GRANT SELECT, INSERT, UPDATE ON TABLES TO propflow_app';
  EXECUTE 'ALTER DEFAULT PRIVILEGES IN SCHEMA app
             GRANT USAGE, SELECT ON SEQUENCES TO propflow_app';

  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_worker') THEN
    EXECUTE 'GRANT USAGE ON SCHEMA app TO propflow_worker';
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA app TO propflow_worker';
    EXECUTE 'GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA app TO propflow_worker';
    EXECUTE 'GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA app TO propflow_worker';
  END IF;

  -- audit_log: sola lettura per app e worker. La revoca va applicata al padre E a
  -- ogni partizione: i privilegi delle partizioni non si ereditano dal padre.
  FOR r IN
    SELECT c.relname FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
     WHERE n.nspname = 'app' AND c.relkind IN ('r', 'p') AND c.relname LIKE 'audit\_log%'
  LOOP
    EXECUTE format('REVOKE INSERT, UPDATE, DELETE ON app.%I FROM propflow_app', r.relname);
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_worker') THEN
      EXECUTE format('REVOKE INSERT, UPDATE, DELETE ON app.%I FROM propflow_worker', r.relname);
    END IF;
  END LOOP;
END $$;
