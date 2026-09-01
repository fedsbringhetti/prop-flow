-- =============================================================================
-- Chi esce da un gruppo di spesa esce per sé stesso, e lo dice la policy.
--
-- DIFETTO
-- Due difetti della stessa policy, e per questo una migration sola.
--
-- 1. `expense_group_members_update` ha una `WITH CHECK` che nomina il solo
--    `org_id`, quindi lascia libera ogni altra colonna: un membro attivo può
--    riscrivere `user_id` di chiunque nel gruppo — cioè intestare a un'altra
--    persona una membership, e con essa le quote già scritte a suo nome — oppure
--    `group_id`, spostando la riga in un altro gruppo. È la constatazione di
--    ADR-021 e ADR-026, che qui vale su una quarta tabella.
--
-- 2. La `USING` ammette **qualunque membro attivo del gruppo**, quindi la Row
--    Level Security lascia passare una query scritta a mano che toglie un altro
--    coinquilino. Il service layer lo impedisce — `LeaveGroup` risponde
--    `expense_group.membership_not_yours` — ma una regola applicativa non è un
--    confine, e questa si legge come una garanzia proprio perché è scritta in tre
--    posti. Un coinquilino non deve poter togliere unilateralmente un altro da un
--    conto condiviso, tanto meno uno con un saldo aperto.
--
-- LA RESTRIZIONE OVVIA È SBAGLIATA, E ROMPE IL RIENTRO
-- Viene naturale stringere la `USING` con
-- `user_id = app.current_user_id() OR app.is_org_staff(org_id)`. **Non si può**:
-- il rientro di un coinquilino è un `UPDATE` sulla riga di **un altro**, fatto da
-- chi lo riammette — `expense_group_members_group_user_uk` è unico e non parziale,
-- quindi chi è uscito non può avere una seconda riga e reintegrarlo non è un
-- `INSERT`. Con quella `USING`, un membro non staff non potrebbe più far rientrare
-- nessuno, e il sintomo sarebbe zero righe aggiornate, cioè
-- `auth.denied_by_policy` — il codice riservato ai nostri difetti.
--
-- CORREZIONE
-- La `USING` resta larga: governa **quali righe** si possono toccare, e toccarle
-- serve. A stringere è la `WITH CHECK`, che vede la riga **nuova**:
--
--   user_id = app.current_user_id()   esco io, o rientro io
--   OR app.is_org_staff(org_id)       lo staff amministra
--   OR left_at IS NULL                il risultato lascia la persona DENTRO
--
-- Il terzo ramo è la parte che fa il lavoro: un membro non staff può scrivere la
-- riga di un altro **solo se il risultato lo lascia nel gruppo**. Reintegrare sì,
-- togliere no. È la distinzione che una `USING` non può esprimere, perché la
-- direzione di un'uscita si legge sulla riga nuova e non su quella esistente.
--
-- COSA RESTA AMMESSO, E VA SAPUTO
-- Un membro attivo può ancora cambiare il **peso** di un altro membro attivo: il
-- terzo ramo lo consente, perché la riga risultante ha `left_at IS NULL`. È
-- deliberato — il gruppo deve poter registrare che qualcuno si è spostato nella
-- doppia, e farlo passare dal proprietario renderebbe il co-living dipendente da
-- lui per un fatto interno alla casa. Oggi nessuno use case espone quella
-- scrittura: `AddGroupMember` su un membro già attivo viene rifiutato dal dominio
-- con `expense_group.membership_unchanged`. Il giorno in cui una rotta la
-- esponesse, la policy la ammetterebbe, e va deciso allora se è ciò che si vuole.
--
-- PRIVILEGI DI COLONNA
-- La `WITH CHECK` difende la **direzione** dell'operazione e non le colonne: una
-- policy non può confrontare la riga nuova con quella vecchia, quindi non sa dire
-- che `user_id` è cambiato. Da qui la `GRANT` ristretta, con il criterio di
-- ADR-026 applicato a una quarta tabella.
--
-- Concesse: `room_id`, `display_name`, `weight`, `joined_at`, `left_at` — le
-- cinque che il dominio muove. `joined_at` è fra esse perché **il rientro la
-- riscrive**: `expense_group_members_dates_ck` è
-- `left_at IS NULL OR left_at >= joined_at`, e lasciare la data vecchia sarebbe
-- ammesso dal vincolo ma direbbe che la persona è nel gruppo da prima di esserci
-- tornata.
--
-- Negate: `org_id` è il confine di isolamento (ADR-021); `group_id` sposterebbe
-- la membership in un altro gruppo, portandosi dietro una persona che in quel
-- gruppo nessuno ha ammesso; `user_id` intesterebbe a un altro una membership e
-- con essa le quote già scritte — ed è la più cara delle tre, perché
-- `app.expense_shares` continua a puntare alla persona vecchia e i saldi
-- smetterebbero di sommare a zero senza che nulla protesti.
--
-- `updated_at` non compare perché la scrive il trigger `set_updated_at`, che è
-- `BEFORE UPDATE` e non è soggetto ai privilegi di colonna del chiamante.
--
-- `propflow_worker` non viene toccato: ha `BYPASSRLS` ed è il ruolo di fiducia
-- per definizione.
-- =============================================================================

ALTER POLICY expense_group_members_update ON app.expense_group_members
    WITH CHECK (
      org_id = app.current_org_id()
      AND (
        -- Esco io, o rientro io.
        user_id = app.current_user_id()
        -- Lo staff amministra.
        OR app.is_org_staff(org_id)
        -- Un altro membro può reintegrare, e non togliere: il risultato deve
        -- lasciare la persona dentro il gruppo.
        OR left_at IS NULL
      )
    );

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_app') THEN
    RAISE NOTICE 'ruolo propflow_app assente: privilegi non applicati (ambiente di sviluppo)';
    RETURN;
  END IF;

  -- Membership: si muovono la stanza, il nome mostrato, il peso e le due date.
  -- Chi è la persona, in quale gruppo e in quale organizzazione sono fissati
  -- all'iscrizione.
  REVOKE UPDATE ON app.expense_group_members FROM propflow_app;
  GRANT  UPDATE (room_id, display_name, weight, joined_at, left_at)
      ON app.expense_group_members TO propflow_app;
END $$;
