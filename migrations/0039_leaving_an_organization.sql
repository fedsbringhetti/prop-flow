-- =============================================================================
-- Uscire da un'organizzazione è uno **stato**, e il divieto è una policy.
--
-- IL FATTO
-- Da un'organizzazione non si esce. `app.org_members` ha `status` e `deleted_at`,
-- quindi l'uscita *sembra* rappresentabile senza toccare lo schema, ma
-- `org_members_update` è `is_org_staff(org_id)` su USING **e** su WITH CHECK: un
-- inquilino non può scrivere la propria riga, e nessuno può uscire da sé.
--
-- STATO E NON SOFT DELETE, E LA RAGIONE È UN DIFETTO GIÀ PAGATO
-- `status = 'left'`, non `deleted_at`. Le due cose non sono intercambiabili:
-- `OrgMemberRepository::find` filtra `deleted_at IS NULL`, quindi una membership
-- cancellata **non si vede più**, e `org_members_org_user_uk` è unico e **non
-- parziale** — chi è uscito così non potrebbe più rientrare né con un `INSERT`,
-- che sbatte sul vincolo, né con un `UPDATE` su una riga che non si trova. È
-- esattamente il difetto che la `0029` ha già pagato sulle spese co-living.
--
-- LA `USING` RESTA LARGA, A STRINGERE È LA `WITH CHECK`
-- Stessa forma della `0029`, e per la stessa ragione: **una policy non può
-- confrontare la riga vecchia con quella nuova**, quindi la direzione di
-- un'uscita si legge solo sulla riga nuova.
--
--   USING      user_id = app.current_user_id()      la propria riga, e solo quella
--   WITH CHECK user_id = app.current_user_id()
--              AND (status <> 'left'                rientrare è sempre ammesso
--                   OR NOT app.has_lease_in_force(…))   uscire, no, se c'è un contratto
--
-- Il secondo ramo è ciò che fa il lavoro. Senza la distinzione fra le due
-- direzioni, il divieto colpirebbe anche la **riattivazione**: chi rientra
-- accettando un invito nuovo passa da un `UPDATE` sulla stessa riga, e in quel
-- momento il contratto nuovo può già essere in essere.
--
-- L'ordine dentro `AcceptInvitation` lo rende oggi impossibile — la membership si
-- riattiva **prima** che `lease_parties.user_id` venga collegato, quindi
-- `has_lease_in_force` è ancora falsa — ma appoggiarsi a quell'ordine sarebbe una
-- garanzia scritta a cinquanta righe di distanza, in un file che nessuno rilegge
-- toccando questa policy.
--
-- IL DIVIETO VALE ANCHE PER LO STAFF, ED È UNA RIGA IN PIÙ QUI
-- Le policy permissive si combinano in **OR**: la `WITH CHECK` di
-- `org_members_update` — `is_org_staff(org_id)` — lascerebbe passare l'uscita di un
-- proprietario con un contratto in essere, cioè proprio il caso che questa
-- migration esiste per chiudere, dalla porta accanto. La si emenda quindi con lo
-- stesso ramo, e non è un allargamento di perimetro: `org_members_update` non ha
-- oggi **nessun** chiamante — `OrgMemberRepository` non aveva un `update` — quindi
-- non rompe niente, e chiude in anticipo la porta che `R39` (togliere qualcuno
-- dalla propria organizzazione) percorrerà.
--
-- PERCHÉ UNA FUNZIONE E NON UN `EXISTS` IN LINEA
-- Perché una sottoquery dentro una policy **subisce la Row Level Security della
-- tabella che legge**: scritta in linea, quella `SELECT` passerebbe da
-- `leases_select`, che è `can_access_lease(id)` e che per lo staff dipende a sua
-- volta dalla membership che si sta aggiornando. È la trappola registrata in testa
-- alla `0026` e ritrovata dalla `0030`, qui alla quarta occorrenza. `SECURITY
-- DEFINER` con `search_path` fissato, come tutte le altre.
--
-- «IN ESSERE» È UNA DEFINIZIONE SOLA
-- La funzione passa da `app.lease_in_force`, che è **l'unica** definizione del
-- progetto (ADR-019). Confrontare le date a mano ne creerebbe una seconda, e
-- diverrebbe falsa il giorno in cui la prima cambia.
--
-- CHI È PARTE, E NON SOLO CHI CI ABITA
-- Nessun filtro su `lp.role`: il garante di un contratto in essere **non** esce.
-- È la differenza con `app.property_residents`, che esclude i garanti perché
-- risponde a «chi abita qui»; qui la domanda è «questa persona ha ancora un
-- legame con un contratto in corso», e un garante ce l'ha. `lp.left_at IS NULL`
-- invece sì: chi ha lasciato il contratto non ne è più parte.
-- =============================================================================

CREATE FUNCTION app.has_lease_in_force(p_org_id uuid, p_user_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT EXISTS (
    SELECT 1
      FROM app.lease_parties lp
      JOIN app.leases l ON l.id = lp.lease_id
     WHERE lp.user_id = p_user_id
       AND lp.deleted_at IS NULL
       AND lp.left_at IS NULL
       AND l.org_id = p_org_id
       AND l.deleted_at IS NULL
       AND app.lease_in_force(l.status, l.start_date, l.end_date)
  )
$$;

COMMENT ON FUNCTION app.has_lease_in_force IS
  'Vero se la persona è parte di un contratto in essere in quella organizzazione.
   Garanti compresi: la domanda è se esista ancora un legame contrattuale, non chi
   ci abita — al contrario di app.property_residents. SECURITY DEFINER perché una
   sottoquery dentro una policy subisce la RLS della tabella che legge, e
   leases_select restringe al singolo contratto (trappola della 0026).
   «In essere» passa da app.lease_in_force e da nessun''altra parte (ADR-019).';

-- I privilegi su una funzione nuova non arrivano dal `GRANT ... ON ALL FUNCTIONS`
-- della 0015, che si applica solo a ciò che esisteva allora: senza questo, la
-- concessione dipenderebbe dal default di PUBLIC invece di essere dichiarata.
DO $$
DECLARE r text;
BEGIN
  FOREACH r IN ARRAY ARRAY['propflow_app', 'propflow_worker'] LOOP
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = r) THEN
      EXECUTE format(
        'GRANT EXECUTE ON FUNCTION app.has_lease_in_force(uuid, uuid) TO %I', r);
    END IF;
  END LOOP;
END $$;

-- ---------------------------------------------------------------------------
-- La propria appartenenza si scrive da sé, in una direzione sola
-- ---------------------------------------------------------------------------

CREATE POLICY org_members_update_self ON app.org_members FOR UPDATE
    USING (user_id = app.current_user_id())
    WITH CHECK (
      user_id = app.current_user_id()
      AND (
        -- Rientrare, o qualunque scrittura che lascia la persona dentro.
        status <> 'left'
        -- Uscire: non con un contratto in essere. Il rifiuto arriva da qui e non
        -- da un `if`, che è la lezione della 0029.
        OR NOT app.has_lease_in_force(org_id, user_id)
      )
    );

-- Lo stesso divieto sul ramo dello staff, perché le policy permissive si sommano
-- in OR e quel ramo lascerebbe passare l'uscita di chi amministra.
ALTER POLICY org_members_update ON app.org_members
    WITH CHECK (
      app.is_org_staff(org_id)
      AND (status <> 'left' OR NOT app.has_lease_in_force(org_id, user_id))
    );

-- ---------------------------------------------------------------------------
-- Privilegi di colonna: `role` è quella che si dimentica
--
-- Aprire la scrittura della propria riga senza negare `role` è **dare a chiunque
-- la promozione a `owner`**: uscire e promuoversi sarebbero la stessa `UPDATE`, e
-- una policy non può accorgersene perché non vede la riga vecchia. È la
-- constatazione di ADR-021 e ADR-026 su una sesta tabella.
--
-- Concesse: `status`, che è l'uscita e il rientro, e `joined_at`, perché **il
-- rientro la riscrive** — lasciare la data vecchia direbbe che la persona è
-- nell'organizzazione da prima di esserci tornata. È la stessa coppia di ragioni
-- della `0029`.
--
-- Negate: `org_id` è il confine di isolamento (ADR-021); `user_id` intesterebbe a
-- un altro un'appartenenza; `role` è la promozione; `invited_by` è un fatto
-- registrato; `deleted_at` è la via che questa migration esiste per **non**
-- prendere.
--
-- `updated_at` non compare perché la scrive il trigger `set_updated_at`, che è
-- `BEFORE UPDATE` e non è soggetto ai privilegi di colonna del chiamante.
--
-- `propflow_worker` non viene toccato: ha `BYPASSRLS` ed è il ruolo di fiducia per
-- definizione.
-- ---------------------------------------------------------------------------

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_app') THEN
    RAISE NOTICE 'ruolo propflow_app assente: privilegi non applicati (ambiente di sviluppo)';
    RETURN;
  END IF;

  REVOKE UPDATE ON app.org_members FROM propflow_app;
  GRANT  UPDATE (status, joined_at) ON app.org_members TO propflow_app;
END $$;
