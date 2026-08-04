-- =============================================================================
-- Row Level Security
--
-- Modello a due livelli:
--   1. isolamento org  → org_id = app.current_org_id() + membership verificata nel DB
--   2. visibilità ruolo → staff (owner/manager) vede tutta l'org;
--                         tenant vede solo ciò che è legato ai propri contratti/gruppi
--
-- Le funzioni di accesso sono SECURITY DEFINER: leggono le tabelle senza riapplicare
-- le policy, evitando la ricorsione infinita fra policy. Per questo motivo
-- FORCE ROW LEVEL SECURITY non va MAI abilitato su queste tabelle.
--
-- COST 500: segnala al planner che la funzione è costosa, così i predicati
-- indicizzabili (org_id, deleted_at) vengono valutati prima.
-- =============================================================================

CREATE FUNCTION app.member_role(p_org_id uuid) RETURNS app.org_role
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT m.role
    FROM app.org_members m
   WHERE m.org_id = p_org_id
     AND m.user_id = app.current_user_id()
     AND m.status = 'active'
     AND m.deleted_at IS NULL
$$;

CREATE FUNCTION app.is_org_member(p_org_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$ SELECT app.member_role(p_org_id) IS NOT NULL OR app.is_platform_admin() $$;

CREATE FUNCTION app.is_org_staff(p_org_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT COALESCE(app.member_role(p_org_id)
                  = ANY (ARRAY['owner','manager']::app.org_role[]), false)
      OR app.is_platform_admin()
$$;

-- Serve a mostrare i nomi delle controparti (inquilino ↔ proprietario)
CREATE FUNCTION app.shares_org_with(p_user_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT EXISTS (
    SELECT 1
      FROM app.org_members a
      JOIN app.org_members b ON b.org_id = a.org_id
     WHERE a.user_id = app.current_user_id() AND a.status = 'active' AND a.deleted_at IS NULL
       AND b.user_id = p_user_id            AND b.status = 'active' AND b.deleted_at IS NULL
  )
$$;

CREATE FUNCTION app.can_access_lease(p_lease_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT EXISTS (
    SELECT 1
      FROM app.leases l
     WHERE l.id = p_lease_id
       AND (
         app.is_org_staff(l.org_id)
         OR EXISTS (
           SELECT 1 FROM app.lease_parties lp
            WHERE lp.lease_id = l.id
              AND lp.user_id = app.current_user_id()
              AND lp.deleted_at IS NULL
         )
       )
  )
$$;

CREATE FUNCTION app.can_access_property(p_property_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT EXISTS (
    SELECT 1
      FROM app.properties p
     WHERE p.id = p_property_id
       AND (
         app.is_org_staff(p.org_id)
         -- l'inquilino vede l'immobile in cui abita (anche solo per stanza)
         OR EXISTS (
           SELECT 1
             FROM app.leases l
             JOIN app.lease_parties lp ON lp.lease_id = l.id
            WHERE l.property_id = p.id
              AND l.deleted_at IS NULL
              AND lp.user_id = app.current_user_id()
              AND lp.deleted_at IS NULL
         )
       )
  )
$$;

CREATE FUNCTION app.can_access_expense_group(p_group_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT EXISTS (
    SELECT 1
      FROM app.expense_groups g
     WHERE g.id = p_group_id
       AND (
         app.is_org_staff(g.org_id)
         OR EXISTS (
           SELECT 1 FROM app.expense_group_members m
            WHERE m.group_id = g.id
              AND m.user_id = app.current_user_id()
              AND m.left_at IS NULL
         )
       )
  )
$$;

CREATE FUNCTION app.can_access_ticket(p_ticket_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT EXISTS (
    SELECT 1
      FROM app.tickets t
     WHERE t.id = p_ticket_id
       AND (
         app.is_org_staff(t.org_id)
         OR t.reported_by = app.current_user_id()
         OR t.assigned_to = app.current_user_id()
         OR (t.lease_id IS NOT NULL AND app.can_access_lease(t.lease_id))
       )
  )
$$;

-- Accesso ai documenti risolto sull'oggetto a cui sono allegati
CREATE FUNCTION app.can_access_attachment(
    p_owner_type app.attachment_owner_type, p_owner_id uuid
) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT CASE p_owner_type
    WHEN 'property'     THEN app.can_access_property(p_owner_id)
    WHEN 'lease'        THEN app.can_access_lease(p_owner_id)
    WHEN 'ticket'       THEN app.can_access_ticket(p_owner_id)
    WHEN 'user'         THEN p_owner_id = app.current_user_id()
    WHEN 'room'         THEN EXISTS (SELECT 1 FROM app.rooms r
                                      WHERE r.id = p_owner_id
                                        AND app.can_access_property(r.property_id))
    WHEN 'expense'      THEN EXISTS (SELECT 1 FROM app.expenses e
                                      WHERE e.id = p_owner_id
                                        AND app.can_access_expense_group(e.group_id))
    WHEN 'utility_bill' THEN EXISTS (SELECT 1 FROM app.utility_bills b
                                      WHERE b.id = p_owner_id
                                        AND app.can_access_property(b.property_id))
    WHEN 'charge'       THEN EXISTS (SELECT 1 FROM app.charges c
                                      WHERE c.id = p_owner_id
                                        AND app.can_access_lease(c.lease_id))
    -- organization / payment / ticket_message: solo staff
    ELSE false
  END
$$;

-- ---------------------------------------------------------------------------
-- Attivazione RLS su tutte le tabelle di business
-- ---------------------------------------------------------------------------
DO $$
DECLARE r record;
BEGIN
  FOR r IN
    SELECT c.relname FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
     WHERE n.nspname = 'app' AND c.relkind IN ('r', 'p')
  LOOP
    EXECUTE format('ALTER TABLE app.%I ENABLE ROW LEVEL SECURITY', r.relname);
  END LOOP;
END $$;

-- ===== Identity ============================================================
CREATE POLICY organizations_select ON app.organizations FOR SELECT
    USING (deleted_at IS NULL AND app.is_org_member(id));
CREATE POLICY organizations_insert ON app.organizations FOR INSERT
    WITH CHECK (app.current_user_id() IS NOT NULL);   -- registrazione: crea la personal org
CREATE POLICY organizations_update ON app.organizations FOR UPDATE
    USING (deleted_at IS NULL AND app.is_org_staff(id))
    WITH CHECK (app.is_org_staff(id));

CREATE POLICY users_select ON app.users FOR SELECT
    USING (deleted_at IS NULL
           AND (id = app.current_user_id() OR app.shares_org_with(id) OR app.is_platform_admin()));
CREATE POLICY users_update_self ON app.users FOR UPDATE
    USING (id = app.current_user_id() AND deleted_at IS NULL)
    WITH CHECK (id = app.current_user_id());
-- INSERT utenti: solo propflow_worker (BYPASSRLS), il signup è pre-autenticazione

CREATE POLICY org_members_select ON app.org_members FOR SELECT
    USING (deleted_at IS NULL
           AND (user_id = app.current_user_id() OR app.is_org_member(org_id)));
CREATE POLICY org_members_insert ON app.org_members FOR INSERT
    WITH CHECK (
      app.is_org_staff(org_id)
      -- bootstrap: il primo membro della propria org è l'owner stesso
      OR (user_id = app.current_user_id() AND role = 'owner'
          AND NOT EXISTS (SELECT 1 FROM app.org_members m WHERE m.org_id = org_members.org_id))
    );
CREATE POLICY org_members_update ON app.org_members FOR UPDATE
    USING (app.is_org_staff(org_id)) WITH CHECK (app.is_org_staff(org_id));

CREATE POLICY org_invitations_staff ON app.org_invitations FOR ALL
    USING (app.is_org_staff(org_id)) WITH CHECK (app.is_org_staff(org_id));

CREATE POLICY user_sessions_own ON app.user_sessions FOR ALL
    USING (user_id = app.current_user_id()) WITH CHECK (user_id = app.current_user_id());

-- ===== Patrimonio ==========================================================
CREATE POLICY properties_select ON app.properties FOR SELECT
    USING (deleted_at IS NULL AND org_id = app.current_org_id() AND app.can_access_property(id));
CREATE POLICY properties_insert ON app.properties FOR INSERT
    WITH CHECK (org_id = app.current_org_id() AND app.is_org_staff(org_id));
CREATE POLICY properties_update ON app.properties FOR UPDATE
    USING (deleted_at IS NULL AND org_id = app.current_org_id() AND app.is_org_staff(org_id))
    WITH CHECK (org_id = app.current_org_id() AND app.is_org_staff(org_id));

CREATE POLICY rooms_select ON app.rooms FOR SELECT
    USING (deleted_at IS NULL AND org_id = app.current_org_id()
           AND app.can_access_property(property_id));
CREATE POLICY rooms_insert ON app.rooms FOR INSERT
    WITH CHECK (org_id = app.current_org_id() AND app.is_org_staff(org_id));
CREATE POLICY rooms_update ON app.rooms FOR UPDATE
    USING (deleted_at IS NULL AND org_id = app.current_org_id() AND app.is_org_staff(org_id))
    WITH CHECK (org_id = app.current_org_id() AND app.is_org_staff(org_id));

-- ===== Contratti ===========================================================
CREATE POLICY leases_select ON app.leases FOR SELECT
    USING (deleted_at IS NULL AND org_id = app.current_org_id() AND app.can_access_lease(id));
CREATE POLICY leases_insert ON app.leases FOR INSERT
    WITH CHECK (org_id = app.current_org_id() AND app.is_org_staff(org_id));
CREATE POLICY leases_update ON app.leases FOR UPDATE
    USING (deleted_at IS NULL AND org_id = app.current_org_id() AND app.is_org_staff(org_id))
    WITH CHECK (org_id = app.current_org_id() AND app.is_org_staff(org_id));

CREATE POLICY lease_parties_select ON app.lease_parties FOR SELECT
    USING (deleted_at IS NULL AND org_id = app.current_org_id() AND app.can_access_lease(lease_id));
CREATE POLICY lease_parties_insert ON app.lease_parties FOR INSERT
    WITH CHECK (org_id = app.current_org_id() AND app.is_org_staff(org_id));
CREATE POLICY lease_parties_update ON app.lease_parties FOR UPDATE
    USING (org_id = app.current_org_id() AND app.is_org_staff(org_id))
    WITH CHECK (org_id = app.current_org_id() AND app.is_org_staff(org_id));

-- ===== Documenti ===========================================================
CREATE POLICY documents_select ON app.documents FOR SELECT
    USING (deleted_at IS NULL AND org_id = app.current_org_id()
           AND (app.is_org_staff(org_id) OR app.can_access_attachment(owner_type, owner_id)));
CREATE POLICY documents_insert ON app.documents FOR INSERT
    WITH CHECK (org_id = app.current_org_id()
                AND (app.is_org_staff(org_id) OR app.can_access_attachment(owner_type, owner_id)));
CREATE POLICY documents_update ON app.documents FOR UPDATE
    USING (deleted_at IS NULL AND org_id = app.current_org_id()
           AND (app.is_org_staff(org_id) OR uploaded_by = app.current_user_id()))
    WITH CHECK (org_id = app.current_org_id());

-- ===== Ticket ==============================================================
CREATE POLICY tickets_select ON app.tickets FOR SELECT
    USING (deleted_at IS NULL AND org_id = app.current_org_id() AND app.can_access_ticket(id));
-- l'inquilino apre ticket sugli immobili a cui ha accesso
CREATE POLICY tickets_insert ON app.tickets FOR INSERT
    WITH CHECK (org_id = app.current_org_id() AND app.can_access_property(property_id));
CREATE POLICY tickets_update ON app.tickets FOR UPDATE
    USING (deleted_at IS NULL AND org_id = app.current_org_id()
           AND (app.is_org_staff(org_id) OR reported_by = app.current_user_id()
                OR assigned_to = app.current_user_id()))
    WITH CHECK (org_id = app.current_org_id());

CREATE POLICY ticket_messages_select ON app.ticket_messages FOR SELECT
    USING (deleted_at IS NULL AND org_id = app.current_org_id()
           AND app.can_access_ticket(ticket_id)
           -- le note interne restano invisibili all'inquilino
           AND (NOT is_internal OR app.is_org_staff(org_id)));
CREATE POLICY ticket_messages_insert ON app.ticket_messages FOR INSERT
    WITH CHECK (org_id = app.current_org_id() AND app.can_access_ticket(ticket_id)
                AND (NOT is_internal OR app.is_org_staff(org_id)));

CREATE POLICY ticket_status_history_select ON app.ticket_status_history FOR SELECT
    USING (org_id = app.current_org_id() AND app.can_access_ticket(ticket_id));

-- ===== Contabilità =========================================================
CREATE POLICY utility_bills_select ON app.utility_bills FOR SELECT
    USING (deleted_at IS NULL AND org_id = app.current_org_id()
           AND (app.is_org_staff(org_id)
                OR (lease_id IS NOT NULL AND app.can_access_lease(lease_id))));
CREATE POLICY utility_bills_insert ON app.utility_bills FOR INSERT
    WITH CHECK (org_id = app.current_org_id() AND app.is_org_staff(org_id));
CREATE POLICY utility_bills_update ON app.utility_bills FOR UPDATE
    USING (org_id = app.current_org_id() AND app.is_org_staff(org_id))
    WITH CHECK (org_id = app.current_org_id() AND app.is_org_staff(org_id));

CREATE POLICY charges_select ON app.charges FOR SELECT
    USING (deleted_at IS NULL AND org_id = app.current_org_id() AND app.can_access_lease(lease_id));
CREATE POLICY charges_insert ON app.charges FOR INSERT
    WITH CHECK (org_id = app.current_org_id() AND app.is_org_staff(org_id));
CREATE POLICY charges_update ON app.charges FOR UPDATE
    USING (org_id = app.current_org_id() AND app.is_org_staff(org_id))
    WITH CHECK (org_id = app.current_org_id() AND app.is_org_staff(org_id));

CREATE POLICY payments_select ON app.payments FOR SELECT
    USING (org_id = app.current_org_id()
           AND (app.is_org_staff(org_id)
                OR payer_user_id = app.current_user_id()
                OR payee_user_id = app.current_user_id()));
CREATE POLICY payments_insert ON app.payments FOR INSERT
    WITH CHECK (org_id = app.current_org_id()
                AND (app.is_org_staff(org_id) OR payer_user_id = app.current_user_id()));
CREATE POLICY payments_update ON app.payments FOR UPDATE
    USING (org_id = app.current_org_id() AND app.is_org_staff(org_id))
    WITH CHECK (org_id = app.current_org_id() AND app.is_org_staff(org_id));

-- ===== Splitwise ===========================================================
CREATE POLICY expense_groups_select ON app.expense_groups FOR SELECT
    USING (deleted_at IS NULL AND org_id = app.current_org_id()
           AND app.can_access_expense_group(id));
CREATE POLICY expense_groups_insert ON app.expense_groups FOR INSERT
    WITH CHECK (org_id = app.current_org_id() AND app.is_org_member(org_id));
CREATE POLICY expense_groups_update ON app.expense_groups FOR UPDATE
    USING (deleted_at IS NULL AND org_id = app.current_org_id()
           AND app.can_access_expense_group(id))
    WITH CHECK (org_id = app.current_org_id());

CREATE POLICY expense_group_members_select ON app.expense_group_members FOR SELECT
    USING (org_id = app.current_org_id() AND app.can_access_expense_group(group_id));
CREATE POLICY expense_group_members_insert ON app.expense_group_members FOR INSERT
    WITH CHECK (org_id = app.current_org_id() AND app.can_access_expense_group(group_id));
CREATE POLICY expense_group_members_update ON app.expense_group_members FOR UPDATE
    USING (org_id = app.current_org_id() AND app.can_access_expense_group(group_id))
    WITH CHECK (org_id = app.current_org_id());

CREATE POLICY expenses_select ON app.expenses FOR SELECT
    USING (deleted_at IS NULL AND org_id = app.current_org_id()
           AND app.can_access_expense_group(group_id));
CREATE POLICY expenses_insert ON app.expenses FOR INSERT
    WITH CHECK (org_id = app.current_org_id() AND app.can_access_expense_group(group_id));
CREATE POLICY expenses_update ON app.expenses FOR UPDATE
    USING (deleted_at IS NULL AND org_id = app.current_org_id()
           AND app.can_access_expense_group(group_id))
    WITH CHECK (org_id = app.current_org_id());

CREATE POLICY expense_shares_select ON app.expense_shares FOR SELECT
    USING (org_id = app.current_org_id()
           AND EXISTS (SELECT 1 FROM app.expenses e
                        WHERE e.id = expense_shares.expense_id
                          AND app.can_access_expense_group(e.group_id)));
CREATE POLICY expense_shares_insert ON app.expense_shares FOR INSERT
    WITH CHECK (org_id = app.current_org_id()
                AND EXISTS (SELECT 1 FROM app.expenses e
                             WHERE e.id = expense_shares.expense_id
                               AND app.can_access_expense_group(e.group_id)));
CREATE POLICY expense_shares_update ON app.expense_shares FOR UPDATE
    USING (org_id = app.current_org_id()
           AND EXISTS (SELECT 1 FROM app.expenses e
                        WHERE e.id = expense_shares.expense_id
                          AND app.can_access_expense_group(e.group_id)))
    WITH CHECK (org_id = app.current_org_id());

CREATE POLICY settlements_select ON app.settlements FOR SELECT
    USING (org_id = app.current_org_id() AND app.can_access_expense_group(group_id));
CREATE POLICY settlements_insert ON app.settlements FOR INSERT
    WITH CHECK (org_id = app.current_org_id() AND app.can_access_expense_group(group_id)
                AND (from_user_id = app.current_user_id() OR app.is_org_staff(org_id)));
CREATE POLICY settlements_update ON app.settlements FOR UPDATE
    USING (org_id = app.current_org_id() AND app.can_access_expense_group(group_id)
           AND (to_user_id = app.current_user_id() OR from_user_id = app.current_user_id()
                OR app.is_org_staff(org_id)))
    WITH CHECK (org_id = app.current_org_id());

-- ===== AI / eventi / audit =================================================
CREATE POLICY ai_jobs_select ON app.ai_jobs FOR SELECT
    USING (org_id = app.current_org_id() AND app.is_org_staff(org_id));
-- INSERT/UPDATE su ai_jobs: solo propflow_worker (BYPASSRLS)

CREATE POLICY domain_events_insert ON app.domain_events FOR INSERT
    WITH CHECK (org_id IS NULL OR org_id = app.current_org_id());
CREATE POLICY domain_events_select ON app.domain_events FOR SELECT
    USING (app.is_platform_admin());
-- il dispatcher gira come propflow_worker (BYPASSRLS)

-- audit_log: sola lettura, solo owner dell'org (e platform admin).
-- Nessuna policy di INSERT: la traccia la scrive il trigger SECURITY DEFINER.
CREATE POLICY audit_log_select ON app.audit_log FOR SELECT
    USING (app.is_platform_admin()
           OR (org_id = app.current_org_id()
               AND app.member_role(org_id) = 'owner'));
