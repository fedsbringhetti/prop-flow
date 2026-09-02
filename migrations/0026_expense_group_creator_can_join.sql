-- =============================================================================
-- Chi crea un gruppo di spesa può iscrivere sé stesso, e nessun altro.
--
-- DIFETTO
-- Lo schema, letto come è uscito dalla `0014`, produce un gruppo **inutilizzabile
-- da chi lo ha creato**:
--
--   expense_groups_insert         org_id = current_org_id() AND is_org_member(org_id)
--   expense_group_members_insert  org_id = current_org_id() AND can_access_expense_group(group_id)
--   can_access_expense_group      is_org_staff(org_id) OR membership attiva nel gruppo
--
-- Un inquilino soddisfa la prima e **non** la seconda: appena creato il gruppo non
-- ne è membro, quindi non può aggiungere nessuno — nemmeno sé stesso. Il gruppo
-- nasce vuoto e resta vuoto, a meno che non passi lo staff.
--
-- CORREZIONE
-- Un secondo ramo nella `WITH CHECK`, stretto quanto il permesso che serve:
-- iscrivere **sé stessi** in un gruppo che si è creati. `user_id =
-- app.current_user_id()` è la parte che non si può omettere — senza, chiunque
-- creasse un gruppo potrebbe infilarci chiunque.
--
-- PERCHÉ UNA FUNZIONE `SECURITY DEFINER` E NON UN `EXISTS` NELLA POLICY
-- È la trappola che farebbe perdere più tempo, perché la versione sbagliata sembra
-- corretta. Scrivere direttamente nella policy
--
--   EXISTS (SELECT 1 FROM app.expense_groups g
--            WHERE g.id = group_id AND g.created_by = app.current_user_id())
--
-- non funziona: **una sottoquery dentro una policy subisce la Row Level Security
-- della tabella che legge**, quindi quel `SELECT` passa da `expense_groups_select`,
-- che chiede `can_access_expense_group(id)` — falsa proprio per il creatore che si
-- sta cercando di autorizzare. La sottoquery vede zero righe e il ramo nuovo non
-- scatta mai. È la ragione per cui **tutte** le `can_access_*` di questo progetto
-- sono `SECURITY DEFINER`, e qui la si incontra al contrario: non come una scelta
-- di prestazioni, ma come l'unico modo di leggere il fatto che serve.
--
-- PERCHÉ LA FUNZIONE ANCORA IL GRUPPO ALL'ORGANIZZAZIONE ATTIVA
-- `app.current_org_id()` è una GUC impostata dall'applicazione e **non è una
-- prova**: a renderla un fatto sono le funzioni che rileggono la membership dal
-- database, ed è ciò che dice il commento di `app.set_request_context` dalla
-- `0001`. Una funzione che verificasse il solo `created_by` lascerebbe questo giro:
-- chi ha creato un gruppo nell'organizzazione A e ne è poi **uscito** potrebbe,
-- con il contesto dell'organizzazione B di cui è ancora membro, inserire una
-- riga di membership verso il gruppo di A — e da quel momento
-- `can_access_expense_group` risponderebbe vero per lui su un gruppo di
-- un'organizzazione a cui non appartiene più.
--
-- Le due condizioni aggiunte lo chiudono: il gruppo deve stare
-- nell'organizzazione **attiva**, e di quella si deve essere membri **adesso**.
-- Poiché la `WITH CHECK` esterna vincola già `org_id = app.current_org_id()`, la
-- riga nuova non può che avere l'organizzazione del gruppo.
--
-- NESSUN FILTRO SUL SOFT DELETE
-- `created_by` è un fatto storico e non cambia quando il gruppo viene archiviato;
-- «il gruppo è ancora vivo» è un filtro di business e sta nel `WHERE` delle query,
-- come stabilito da ADR-008 e come fanno già tutte le `can_access_*`. Metterlo qui
-- mescolerebbe di nuovo le due responsabilità che la `0016` ha separato.
--
-- PERCHÉ NON ALLARGARE `can_access_expense_group` CON `created_by`
-- Sarebbe una riga sola e sarebbe sbagliata: quella funzione governa anche la
-- `SELECT`, quindi chi ha creato un gruppo e ne è poi uscito continuerebbe a
-- leggerne per sempre spese e saldi. Il permesso che serve è stretto — iscrivere
-- sé stessi, una volta — e va scritto stretto.
--
-- PERCHÉ NON «SOLO LO STAFF CREA E POPOLA»
-- Non richiederebbe questa migration e darebbe ragione allo schema come è, ma
-- farebbe passare dal proprietario ogni coinquilino che arriva. Un co-living che
-- non può organizzarsi da sé non è il prodotto descritto in `docs/01-prodotto.md`.
--
-- «UNA VOLTA» LO IMPONE GIÀ LO SCHEMA
-- `expense_group_members_group_user_uk` è unico su `(group_id, user_id)`: il
-- secondo inserimento dello stesso utente nello stesso gruppo è un conflitto, e
-- non serve che la policy lo dica una seconda volta. Chi esce e vuole rientrare
-- passa da un `UPDATE` che riporta `left_at` a `NULL`, non da un `INSERT`.
-- =============================================================================

CREATE FUNCTION app.created_expense_group(p_group_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT EXISTS (
    SELECT 1
      FROM app.expense_groups g
     WHERE g.id = p_group_id
       AND g.created_by = app.current_user_id()
       -- Il gruppo deve stare nell'organizzazione attiva, e di quella si deve
       -- essere membri adesso: `current_org_id()` da sola è una dichiarazione,
       -- non una prova. Vedi la nota in testa.
       AND g.org_id = app.current_org_id()
       AND app.is_org_member(g.org_id)
  )
$$;

COMMENT ON FUNCTION app.created_expense_group IS
  'Vero se l''utente corrente ha creato questo gruppo di spesa, nell''organizzazione
   attiva di cui è ancora membro. Abilita il solo auto-inserimento del creatore in
   `expense_group_members_insert`: senza, un gruppo appena creato da un inquilino
   resterebbe vuoto per sempre. SECURITY DEFINER perché una sottoquery dentro una
   policy subisce la RLS della tabella che legge, ed `expense_groups_select`
   chiede proprio l''accesso che qui manca.';

-- `ALTER POLICY ... WITH CHECK` e non `DROP`/`CREATE`: la policy resta la stessa,
-- con il nome che le query di verifica e il controllo delle invarianti già
-- conoscono, e non c'è una finestra in cui la tabella è priva di policy di INSERT.
ALTER POLICY expense_group_members_insert ON app.expense_group_members
    WITH CHECK (
      org_id = app.current_org_id()
      AND (
        app.can_access_expense_group(group_id)
        -- Il creatore, e solo su sé stesso.
        OR (user_id = app.current_user_id() AND app.created_expense_group(group_id))
      )
    );

-- I privilegi su una funzione nuova non arrivano dal `GRANT ... ON ALL FUNCTIONS`
-- della 0015, che si applica solo a ciò che esisteva allora.
DO $$
DECLARE r text;
BEGIN
  FOREACH r IN ARRAY ARRAY['propflow_app', 'propflow_worker'] LOOP
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = r) THEN
      EXECUTE format(
        'GRANT EXECUTE ON FUNCTION app.created_expense_group(uuid) TO %I', r);
    END IF;
  END LOOP;
END $$;
