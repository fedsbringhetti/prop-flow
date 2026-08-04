-- =============================================================================
-- Controlli statici sullo schema. Da eseguire in CI dopo `sqlx migrate run`.
-- Ogni query deve restituire ZERO righe: qualsiasi output è un difetto.
--   psql -f ops/sql/checks/schema_invariants.sql "$DATABASE_URL"
-- =============================================================================

\set ON_ERROR_STOP on
\pset pager off

\echo '== 1. Tabelle di business senza RLS attiva =='
SELECT c.relname
  FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
 WHERE n.nspname = 'app' AND c.relkind IN ('r', 'p')
   AND NOT c.relrowsecurity
   AND c.relname NOT LIKE 'audit\_log\_%';   -- le partizioni ereditano la RLS del padre

\echo '== 2. Tabelle con RLS attiva ma senza alcuna policy (accesso negato a tutti) =='
SELECT c.relname
  FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
 WHERE n.nspname = 'app' AND c.relkind IN ('r', 'p') AND c.relrowsecurity
   AND NOT EXISTS (SELECT 1 FROM pg_policy p WHERE p.polrelid = c.oid)
   AND c.relname NOT LIKE 'audit\_log\_%';

\echo '== 3. Viste senza security_invoker (bypass della RLS) =='
SELECT c.relname
  FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
 WHERE n.nspname = 'app' AND c.relkind = 'v'
   AND NOT COALESCE(array_to_string(c.reloptions, ',') LIKE '%security_invoker=true%', false);

\echo '== 4. Foreign key senza indice sulla colonna referenziante =='
SELECT con.conrelid::regclass AS referencing_table,
       att.attname            AS column_name,
       con.conname
  FROM pg_constraint con
  JOIN pg_namespace n ON n.oid = con.connamespace
  CROSS JOIN LATERAL unnest(con.conkey) AS k(attnum)
  JOIN pg_attribute att ON att.attrelid = con.conrelid AND att.attnum = k.attnum
 WHERE con.contype = 'f' AND n.nspname = 'app'
   AND array_length(con.conkey, 1) = 1
   AND NOT EXISTS (
     SELECT 1 FROM pg_index i
      WHERE i.indrelid = con.conrelid
        AND i.indkey[0] = k.attnum        -- basta essere prefisso dell'indice
   )
 ORDER BY 1, 2;

\echo '== 5. Tabelle senza trigger updated_at pur avendo la colonna =='
SELECT c.relname
  FROM pg_class c
  JOIN pg_namespace n ON n.oid = c.relnamespace
  JOIN pg_attribute a ON a.attrelid = c.oid AND a.attname = 'updated_at'
                     AND a.attnum > 0 AND NOT a.attisdropped
 WHERE n.nspname = 'app' AND c.relkind IN ('r', 'p')
   AND NOT EXISTS (
     SELECT 1 FROM pg_trigger t
      WHERE t.tgrelid = c.oid AND t.tgname = 'set_updated_at' AND NOT t.tgisinternal
   );

\echo '== 6. Funzioni SECURITY DEFINER senza search_path fissato (privilege escalation) =='
SELECT p.proname
  FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
 WHERE n.nspname = 'app' AND p.prosecdef
   AND NOT COALESCE(array_to_string(p.proconfig, ',') LIKE '%search_path=%', false);

\echo '== 7. FORCE RLS attiva (ricorrerebbe nelle policy via funzioni SECURITY DEFINER) =='
SELECT c.relname
  FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
 WHERE n.nspname = 'app' AND c.relforcerowsecurity;

\echo '== 8. Colonne monetarie che non usano il dominio app.money_amount =='
-- Confronto sul tipo per OID (non sul nome formattato: format_type omette lo schema
-- quando è nel search_path, e restituirebbe falsi negativi/positivi).
SELECT c.relname AS table_name, a.attname AS column_name, format_type(a.atttypid, a.atttypmod) AS type
  FROM pg_class c
  JOIN pg_namespace n ON n.oid = c.relnamespace
  JOIN pg_attribute a ON a.attrelid = c.oid AND a.attnum > 0 AND NOT a.attisdropped
 WHERE n.nspname = 'app' AND c.relkind = 'r'
   AND (a.attname LIKE '%amount%' OR a.attname LIKE '%rent\_%'
        OR a.attname LIKE '%\_rent' OR a.attname LIKE '%price%')
   AND a.atttypid <> 'app.money_amount'::regtype
   -- eccezioni note: costo AI a 6 decimali, giorno di scadenza del canone
   AND NOT (c.relname = 'ai_jobs' AND a.attname = 'cost_amount')
   AND NOT (c.relname = 'leases'  AND a.attname IN ('rent_frequency', 'rent_due_day'))
 ORDER BY 1, 2;

\echo '== 9. Policy di SELECT che filtrano il soft delete =='
-- Una policy di SELECT che contiene `deleted_at IS NULL` rende impossibile il
-- soft delete: PostgreSQL applica le policy di SELECT anche alla riga nuova di un
-- UPDATE, quindi valorizzare `deleted_at` produrrebbe una riga invisibile a chi
-- la scrive. Il filtro va nelle query, non nella RLS. Vedi migrations/0016.
SELECT tablename, policyname
  FROM pg_policies
 WHERE schemaname = 'app' AND cmd = 'SELECT' AND qual LIKE '%deleted_at IS NULL%'
 ORDER BY 1, 2;
