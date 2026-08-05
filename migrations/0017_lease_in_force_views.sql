-- =============================================================================
-- Contratti in essere: predicato derivato dalle date, e viste che lo usano.
--
-- PROBLEMA
-- Lo stato memorizzato di un contratto cambia solo su azioni esplicite
-- (attivazione, disdetta, annullamento): nessuno scheduler lo porta a 'expired'
-- alla scadenza. `v_property_occupancy` e `v_owner_dashboard` contavano invece
-- `status = 'active'`, quindi un contratto a termine restava fra quelli in essere
-- per sempre, gonfiando occupazione e canone atteso.
--
-- DECISIONE
-- "In essere", "in scadenza" e "scaduto" si calcolano dalle date. Lo stato
-- memorizzato resta la decisione di chi amministra; la scadenza è un fatto del
-- calendario, e dedurla non richiede né un job né una colonna da tenere
-- aggiornata. Vedi ADR-019.
--
-- NOTA SUL CONFINE DESTRO
-- `end_date` è **esclusiva**: il contratto copre `[start_date, end_date)`. Non è
-- una scelta nuova, è quella già presa dai vincoli `leases_no_overlap_*`, che
-- usano `daterange(start_date, COALESCE(end_date, 'infinity'), '[)')`. Con un
-- confine inclusivo (`p_end >= CURRENT_DATE`) il giorno di `end_date` sarebbe
-- contemporaneamente "ancora in essere" per questo predicato e "slot libero" per
-- il vincolo di esclusione: due contratti sullo stesso immobile risulterebbero in
-- essere per un giorno, e la dashboard sommerebbe due canoni. Cioè esattamente
-- l'errore di conteggio che questa migration corregge.
-- =============================================================================

CREATE FUNCTION app.lease_in_force(
    p_status app.lease_status,
    p_start  date,
    p_end    date
) RETURNS boolean
  LANGUAGE sql STABLE PARALLEL SAFE AS
$$
  SELECT p_status = 'active'
     AND p_start <= CURRENT_DATE
     AND (p_end IS NULL OR p_end > CURRENT_DATE)
$$;

COMMENT ON FUNCTION app.lease_in_force IS
  'Un contratto è in essere se è attivo e oggi cade in [start_date, end_date).
   Lo stato da solo non basta: un contratto a termine resta ''active'' anche dopo
   la scadenza, perché nessuno lo aggiorna (ADR-019).';

-- ---------------------------------------------------------------------------
-- Le viste tornano a contare ciò che intendevano contare.
-- CREATE OR REPLACE impone colonne identiche in nome, tipo e ordine: cambia solo
-- il predicato.
-- ---------------------------------------------------------------------------
CREATE OR REPLACE VIEW app.v_property_occupancy WITH (security_invoker = true) AS
SELECT p.id AS property_id,
       p.org_id,
       p.is_coliving,
       count(r.id)                                            AS rooms_total,
       count(r.id) FILTER (WHERE r.status = 'occupied')        AS rooms_occupied,
       CASE WHEN count(r.id) > 0
            THEN round(100.0 * count(r.id) FILTER (WHERE r.status = 'occupied')
                       / count(r.id), 2)
       END                                                    AS occupancy_percent,
       count(DISTINCT l.id) FILTER (
         WHERE app.lease_in_force(l.status, l.start_date, l.end_date)
       )                                                      AS active_leases,
       SUM(l.rent_amount) FILTER (
         WHERE app.lease_in_force(l.status, l.start_date, l.end_date)
       )::app.money_amount                                    AS active_monthly_rent
  FROM app.properties p
  LEFT JOIN app.rooms  r ON r.property_id = p.id AND r.deleted_at IS NULL
  LEFT JOIN app.leases l ON l.property_id = p.id AND l.deleted_at IS NULL
 WHERE p.deleted_at IS NULL
 GROUP BY p.id, p.org_id, p.is_coliving;

CREATE OR REPLACE VIEW app.v_owner_dashboard WITH (security_invoker = true) AS
SELECT o.id AS org_id,
       (SELECT count(*) FROM app.properties p
         WHERE p.org_id = o.id AND p.deleted_at IS NULL)                    AS properties_count,
       (SELECT count(*) FROM app.leases l
         WHERE l.org_id = o.id AND l.deleted_at IS NULL
           AND app.lease_in_force(l.status, l.start_date, l.end_date))      AS active_leases,
       (SELECT COALESCE(SUM(l.rent_amount), 0) FROM app.leases l
         WHERE l.org_id = o.id AND l.deleted_at IS NULL
           AND app.lease_in_force(l.status, l.start_date, l.end_date)
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
       -- "in scadenza" è un contratto in essere la cui fine cade nei prossimi 90
       -- giorni: lo stato 'expiring' non viene più memorizzato da nessuno.
       (SELECT count(*) FROM app.leases l
         WHERE l.org_id = o.id AND l.deleted_at IS NULL
           AND app.lease_in_force(l.status, l.start_date, l.end_date)
           AND l.end_date <= CURRENT_DATE + INTERVAL '90 days'
       )                                                                    AS leases_expiring_90d,
       (SELECT count(*) FROM app.utility_bills ub
         WHERE ub.org_id = o.id AND ub.deleted_at IS NULL
           AND ub.status = ANY (ARRAY['draft','pending_review']::app.bill_status[])
       )                                                                    AS bills_pending_review
  FROM app.organizations o
 WHERE o.deleted_at IS NULL;

-- I privilegi su una funzione nuova non arrivano dal `GRANT ... ON ALL FUNCTIONS`
-- della 0015, che si applica solo a ciò che esisteva allora: senza questo, la
-- concessione dipenderebbe dal default di PUBLIC invece di essere dichiarata.
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_app') THEN
    EXECUTE 'GRANT EXECUTE ON FUNCTION app.lease_in_force(app.lease_status, date, date)
               TO propflow_app';
  END IF;
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_worker') THEN
    EXECUTE 'GRANT EXECUTE ON FUNCTION app.lease_in_force(app.lease_status, date, date)
               TO propflow_worker';
  END IF;
END $$;
