-- =============================================================================
-- Invarianti dello schema. Da eseguire dopo le migration, in CI e in locale.
--
-- Ogni controllo raccoglie le proprie violazioni in una tabella temporanea; alla
-- fine le violazioni vengono stampate e, se ce n'è almeno una, lo script solleva
-- un'eccezione. Serve a entrambi gli usi: in locale si legge *cosa* non torna, in
-- CI il processo esce con codice diverso da zero e la pipeline si ferma.
--
--   psql -v ON_ERROR_STOP=1 -f ops/sql/checks/schema_invariants.sql "$DATABASE_URL"
-- =============================================================================

\set ON_ERROR_STOP on
\pset pager off

-- Tabella temporanea di sessione: senza ON COMMIT DROP, che fuori da una
-- transazione esplicita la distruggerebbe subito dopo averla creata.
CREATE TEMP TABLE invariant_violations (
    check_name TEXT NOT NULL,
    detail     TEXT NOT NULL
);

-- 1. Ogni tabella di business deve avere la Row Level Security attiva.
--    Le partizioni di audit_log ereditano la RLS del padre e sono escluse.
INSERT INTO invariant_violations
SELECT 'rls-non-attiva', c.relname
  FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
 WHERE n.nspname = 'app' AND c.relkind IN ('r', 'p')
   AND NOT c.relrowsecurity
   AND c.relname NOT LIKE 'audit\_log\_%'
   AND c.relname <> '_sqlx_migrations';

-- 2. RLS attiva senza policy nega l'accesso a tutti: è un errore, non una scelta.
INSERT INTO invariant_violations
SELECT 'rls-senza-policy', c.relname
  FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
 WHERE n.nspname = 'app' AND c.relkind IN ('r', 'p') AND c.relrowsecurity
   AND NOT EXISTS (SELECT 1 FROM pg_policy p WHERE p.polrelid = c.oid)
   AND c.relname NOT LIKE 'audit\_log\_%';

-- 3. Una vista senza security_invoker gira con i diritti dell'owner e aggira la
--    RLS del chiamante.
INSERT INTO invariant_violations
SELECT 'vista-senza-security-invoker', c.relname
  FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
 WHERE n.nspname = 'app' AND c.relkind = 'v'
   AND NOT COALESCE(array_to_string(c.reloptions, ',') LIKE '%security_invoker=true%', false);

-- 4. Una foreign key senza indice utilizzabile sulla colonna referenziante
--    trasforma ogni verifica di integrità in una scansione sequenziale.
INSERT INTO invariant_violations
SELECT 'fk-senza-indice',
       format('%s.%s (%s)', con.conrelid::regclass, att.attname, con.conname)
  FROM pg_constraint con
  JOIN pg_namespace n ON n.oid = con.connamespace
  CROSS JOIN LATERAL unnest(con.conkey) AS k(attnum)
  JOIN pg_attribute att ON att.attrelid = con.conrelid AND att.attnum = k.attnum
 WHERE con.contype = 'f' AND n.nspname = 'app'
   AND array_length(con.conkey, 1) = 1
   AND NOT EXISTS (
     SELECT 1 FROM pg_index i WHERE i.indrelid = con.conrelid AND i.indkey[0] = k.attnum
   );

-- 5. Una tabella con updated_at ma senza il trigger si affiderebbe a ogni singola
--    query per aggiornarlo, e prima o poi una se lo dimentica.
INSERT INTO invariant_violations
SELECT 'updated-at-senza-trigger', c.relname
  FROM pg_class c
  JOIN pg_namespace n ON n.oid = c.relnamespace
  JOIN pg_attribute a ON a.attrelid = c.oid AND a.attname = 'updated_at'
                     AND a.attnum > 0 AND NOT a.attisdropped
 WHERE n.nspname = 'app' AND c.relkind IN ('r', 'p')
   AND NOT EXISTS (
     SELECT 1 FROM pg_trigger t
      WHERE t.tgrelid = c.oid AND t.tgname = 'set_updated_at' AND NOT t.tgisinternal
   );

-- 6. SECURITY DEFINER senza search_path fissato è una scalata di privilegi in
--    attesa: chi chiama può far risolvere i nomi non qualificati a oggetti suoi.
INSERT INTO invariant_violations
SELECT 'definer-senza-search-path', p.proname
  FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
 WHERE n.nspname = 'app' AND p.prosecdef
   AND NOT COALESCE(array_to_string(p.proconfig, ',') LIKE '%search_path=%', false);

-- 7. FORCE ROW LEVEL SECURITY applica le policy anche all'owner, e le nostre
--    funzioni di accesso sono SECURITY DEFINER proprio per leggere senza policy:
--    con FORCE attiva le policy ricorrono su se stesse.
INSERT INTO invariant_violations
SELECT 'force-rls-attiva', c.relname
  FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
 WHERE n.nspname = 'app' AND c.relforcerowsecurity;

-- 8. Le colonne monetarie devono usare il dominio, che è l'unico posto dove si
--    cambia la precisione.
INSERT INTO invariant_violations
SELECT 'colonna-monetaria-fuori-dominio',
       format('%s.%s è %s', c.relname, a.attname, format_type(a.atttypid, a.atttypmod))
  FROM pg_class c
  JOIN pg_namespace n ON n.oid = c.relnamespace
  JOIN pg_attribute a ON a.attrelid = c.oid AND a.attnum > 0 AND NOT a.attisdropped
 WHERE n.nspname = 'app' AND c.relkind = 'r'
   AND (a.attname LIKE '%amount%' OR a.attname LIKE '%rent\_%'
        OR a.attname LIKE '%\_rent' OR a.attname LIKE '%price%')
   AND a.atttypid <> 'app.money_amount'::regtype
   -- eccezioni volute: costo AI a sei decimali, frequenza e giorno del canone
   AND NOT (c.relname = 'ai_jobs' AND a.attname = 'cost_amount')
   AND NOT (c.relname = 'leases' AND a.attname IN ('rent_frequency', 'rent_due_day'));

-- 9. `deleted_at IS NULL` in una policy di SELECT rende impossibile il soft
--    delete: PostgreSQL applica le policy di SELECT anche alla riga nuova di un
--    UPDATE, quindi valorizzare deleted_at produrrebbe una riga invisibile a chi
--    la scrive. Il filtro va nelle query. Vedi migrations/0016.
INSERT INTO invariant_violations
SELECT 'policy-select-filtra-soft-delete', format('%s.%s', tablename, policyname)
  FROM pg_policies
 WHERE schemaname = 'app' AND cmd = 'SELECT' AND qual LIKE '%deleted_at IS NULL%';

-- 10. Una seconda `_sqlx_migrations` fuori da `public` significa che il migratore
--     ha girato con search_path = app, public su un database dove lo schema app
--     esisteva già: SQLx creerebbe una tabella vuota e riapplicherebbe tutto.
INSERT INTO invariant_violations
SELECT 'bookkeeping-migration-duplicato', format('%s._sqlx_migrations', n.nspname)
  FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
 WHERE c.relname = '_sqlx_migrations' AND n.nspname <> 'public';

-- 11. Colonne che definiscono il confine di isolamento non devono essere
--     aggiornabili dal ruolo applicativo. Le policy dell'invitato concedono
--     scrittura a un non membro, e il `WITH CHECK` vincola solo le colonne che
--     nomina: se `org_id` o `lease_id` tornassero scrivibili, un invito potrebbe
--     essere ripuntato altrove. Vedi migrations/0019 e ADR-021.
INSERT INTO invariant_violations
SELECT 'colonna-di-confine-scrivibile', format('%s.%s', t.tbl, t.col)
  FROM (
    VALUES ('org_invitations', 'org_id'),   ('org_invitations', 'email'),
           ('org_invitations', 'role'),     ('org_invitations', 'token_hash'),
           ('org_invitations', 'lease_id'), ('org_invitations', 'expires_at'),
           ('lease_parties',   'org_id'),   ('lease_parties',   'lease_id')
  ) AS t(tbl, col)
 WHERE EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_app')
   AND has_column_privilege('propflow_app', format('app.%I', t.tbl), t.col, 'UPDATE');

-- 12. Colonne strutturali che non devono essere aggiornabili dal ruolo
--     applicativo. Il criterio è **diverso** da quello del controllo 11: qui non
--     c'è un confine di isolamento da difendere — chi le riscrivesse resterebbe
--     dentro la propria organizzazione — ma una classe di incoerenze che nessuna
--     policy può esprimere, perché una policy non può confrontare la riga nuova
--     con quella vecchia. Vedi migrations/0020, migrations/0023 e ADR-026.
--
--     Per le **segnalazioni**: lo schema non ha alcun vincolo fra `room_id`,
--     `lease_id` e `property_id`, e la `WITH CHECK` di `tickets_update` nomina il
--     solo `org_id`. Una segnalazione aperta sull'immobile sbagliato si annulla e
--     si riapre, non si sposta.
--
--     Per gli **addebiti**: la `WITH CHECK` di `charges_update` ha lo stesso
--     difetto, e qui l'elenco è quasi la tabella intera perché un addebito è una
--     **pretesa verso una persona** — riscriverne l'importo dopo averlo comunicato
--     non è una correzione. La sola colonna concessa è `status`, che basta anche
--     al modulo dei pagamenti. `deleted_at` è fra le negate: un addebito annullato
--     resta visibile a chi se l'era visto chiedere.
--
--     Per i **pagamenti**: la `WITH CHECK` di `payments_update` ha lo stesso
--     difetto, e l'elenco è di nuovo quasi la tabella intera perché un pagamento
--     **registra un fatto già avvenuto** — chi ha pagato, quanto, come — e un
--     fatto non si corregge riscrivendolo. Le sole colonne concesse sono `status`,
--     `paid_at` e `reconciled_at`, cioè le tre che il dominio muove; `paid_at` fra
--     esse perché il rifiuto di una dichiarazione la **azzera**, e
--     `payments_succeeded_ck` vincola una sola direzione. `idempotency_key` è
--     quella la cui riscrittura costa denaro: è metà di
--     `payments_idempotency_uk`, e liberarla apre a un secondo inserimento, cioè
--     a un saldo che scende due volte. Vedi migrations/0025 e ADR-033.
--
--     Un nome solo per tre tabelle, e non un controllo per ciascuna: il criterio è
--     quello di ADR-026 in tutti i casi, e inventargli un nome nuovo renderebbe
--     illeggibile la ragione comune.
INSERT INTO invariant_violations
SELECT 'colonna-strutturale-scrivibile', format('%s.%s', t.tbl, t.col)
  FROM (
    VALUES ('tickets', 'property_id'), ('tickets', 'room_id'),
           ('tickets', 'lease_id'),    ('tickets', 'reported_by'),
           ('tickets', 'reported_by_kind'),
           ('tickets', 'title'),       ('tickets', 'description'),
           ('tickets', 'reference'),   ('tickets', 'currency'),
           ('charges', 'lease_id'),        ('charges', 'charge_type'),
           ('charges', 'amount'),          ('charges', 'currency'),
           ('charges', 'due_date'),        ('charges', 'period_start'),
           ('charges', 'period_end'),      ('charges', 'description'),
           ('charges', 'utility_bill_id'), ('charges', 'generation_key'),
           ('charges', 'issued_at'),       ('charges', 'document_id'),
           ('charges', 'notes'),           ('charges', 'deleted_at'),
           ('payments', 'org_id'),          ('payments', 'payable_type'),
           ('payments', 'payable_id'),      ('payments', 'payer_user_id'),
           ('payments', 'payee_user_id'),   ('payments', 'amount'),
           ('payments', 'currency'),        ('payments', 'method'),
           ('payments', 'idempotency_key'), ('payments', 'reference'),
           ('payments', 'refunded_amount'), ('payments', 'provider'),
           ('payments', 'provider_payment_id')
  ) AS t(tbl, col)
 WHERE EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_app')
   AND has_column_privilege('propflow_app', format('app.%I', t.tbl), t.col, 'UPDATE');

-- ---------------------------------------------------------------------------
\echo ''
\echo 'Violazioni rilevate (vuoto = tutto in ordine):'
SELECT check_name, detail FROM invariant_violations ORDER BY 1, 2;

DO $$
DECLARE v_count bigint;
BEGIN
  SELECT count(*) INTO v_count FROM invariant_violations;
  IF v_count > 0 THEN
    RAISE EXCEPTION '% invarianti dello schema violate (elenco sopra)', v_count;
  END IF;
  RAISE NOTICE 'tutte le invarianti dello schema sono rispettate';
END $$;
