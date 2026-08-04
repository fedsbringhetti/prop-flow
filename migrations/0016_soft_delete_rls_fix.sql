-- =============================================================================
-- Correzione: le policy di SELECT non devono filtrare il soft delete.
--
-- PROBLEMA
-- PostgreSQL applica le policy di SELECT anche alla riga *nuova* di un UPDATE,
-- non solo a quella esistente: non si può scrivere una riga che poi non si
-- potrebbe rileggere. Con `deleted_at IS NULL` dentro la policy di SELECT, un
-- soft delete diventa quindi impossibile per il ruolo applicativo — l'UPDATE che
-- valorizza `deleted_at` produce una riga invisibile a chi la sta scrivendo, e
-- PostgreSQL risponde "new row violates row-level security policy".
--
-- Verificato empiricamente: con la policy originale, `UPDATE app.properties SET
-- deleted_at = now()` come `propflow_app` fallisce; togliendo la condizione dalla
-- sola policy di SELECT, riesce.
--
-- CAUSA
-- Le policy mescolavano due responsabilità diverse: l'isolamento dei dati (di chi
-- è questa riga?) e un filtro di business (questa riga è ancora attiva?). La
-- prima è una proprietà di sicurezza e deve stare nella RLS. La seconda è una
-- condizione applicativa, e messa nella RLS entra in conflitto con l'operazione
-- che la produce.
--
-- CORREZIONE
-- Le policy di SELECT tornano a esprimere solo l'isolamento. Il filtro sul soft
-- delete resta dove era già presente: nel `WHERE deleted_at IS NULL` di ogni
-- query dei repository e nelle viste applicative.
--
-- CONSEGUENZA
-- Una query che dimenticasse il filtro vedrebbe anche le righe archiviate. Non è
-- una falla di isolamento: quelle righe appartengono comunque all'organizzazione
-- del chiamante e restano soggette al filtro per ruolo. Si perde una comodità,
-- non una garanzia. Il controllo 9 di `ops/sql/checks/schema_invariants.sql`
-- impedisce che la condizione rientri nelle policy per distrazione.
--
-- Le policy di UPDATE mantengono `deleted_at IS NULL` nella clausola USING: là
-- si applica alla riga *esistente* e impedisce di modificare un'entità
-- archiviata, senza creare alcuna contraddizione.
-- =============================================================================

ALTER POLICY organizations_select ON app.organizations
    USING (app.is_org_member(id));

ALTER POLICY users_select ON app.users
    USING (
        id = app.current_user_id()
        OR app.shares_org_with(id)
        OR app.is_platform_admin()
    );

ALTER POLICY org_members_select ON app.org_members
    USING (user_id = app.current_user_id() OR app.is_org_member(org_id));

ALTER POLICY properties_select ON app.properties
    USING (org_id = app.current_org_id() AND app.can_access_property(id));

ALTER POLICY rooms_select ON app.rooms
    USING (org_id = app.current_org_id() AND app.can_access_property(property_id));

ALTER POLICY leases_select ON app.leases
    USING (org_id = app.current_org_id() AND app.can_access_lease(id));

ALTER POLICY lease_parties_select ON app.lease_parties
    USING (org_id = app.current_org_id() AND app.can_access_lease(lease_id));

ALTER POLICY documents_select ON app.documents
    USING (
        org_id = app.current_org_id()
        AND (app.is_org_staff(org_id) OR app.can_access_attachment(owner_type, owner_id))
    );

ALTER POLICY tickets_select ON app.tickets
    USING (org_id = app.current_org_id() AND app.can_access_ticket(id));

ALTER POLICY ticket_messages_select ON app.ticket_messages
    USING (
        org_id = app.current_org_id()
        AND app.can_access_ticket(ticket_id)
        AND (NOT is_internal OR app.is_org_staff(org_id))
    );

ALTER POLICY utility_bills_select ON app.utility_bills
    USING (
        org_id = app.current_org_id()
        AND (
            app.is_org_staff(org_id)
            OR (lease_id IS NOT NULL AND app.can_access_lease(lease_id))
        )
    );

ALTER POLICY charges_select ON app.charges
    USING (org_id = app.current_org_id() AND app.can_access_lease(lease_id));

ALTER POLICY expense_groups_select ON app.expense_groups
    USING (org_id = app.current_org_id() AND app.can_access_expense_group(id));

ALTER POLICY expenses_select ON app.expenses
    USING (org_id = app.current_org_id() AND app.can_access_expense_group(group_id));

-- Le policy di UPDATE che valorizzano `deleted_at` devono poter riscrivere la
-- riga: il loro WITH CHECK non menziona `deleted_at`, quindi non serve toccarle.
-- Resta invariata anche la USING con `deleted_at IS NULL`, che continua a
-- impedire la modifica di entità archiviate.
