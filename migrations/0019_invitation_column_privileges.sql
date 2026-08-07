-- =============================================================================
-- Privilegi a livello di colonna sulle tabelle che l'invitato può scrivere.
--
-- DIFETTO
-- Le policy di UPDATE introdotte dalla 0018 vincolano nel `WITH CHECK` soltanto le
-- colonne che nominano; tutte le altre restano libere. Su
-- `org_invitations_invitee_accept` il `WITH CHECK` era `email = current_user_email()`,
-- quindi `org_id` non era vincolato:
--
--   UPDATE app.org_invitations SET org_id = '<altra org>' WHERE email = '<la mia>';
--
-- passava la `USING` (la riga *esistente* era un invito pendente valido) e passava
-- il `WITH CHECK` (l'email non cambiava). Dopo quella riga
-- `has_pending_invitation(<altra org>)` diventava vero, e la policy
-- `org_members_accept_invitation` permetteva l'iscrizione come `tenant` a
-- un'organizzazione qualsiasi di cui si conosca l'identificatore. Da lì
-- `shares_org_with` rendeva leggibili nome ed email di tutti i suoi membri.
--
-- La stessa forma su `lease_parties_invitee_link`: il `WITH CHECK` fissava `email` e
-- `user_id` ma non `lease_id`, quindi l'invitato poteva agganciarsi a un contratto
-- diverso da quello per cui era stato invitato e leggerne canone e immobile.
--
-- Entrambi riprodotti su database reale prima di scrivere questa migration.
--
-- PERCHÉ PROPRIO QUESTE DUE POLICY
-- Sono le uniche che concedono scrittura a un **non membro**, attraversando il
-- confine dell'organizzazione. Altrove una colonna non vincolata resta dentro il
-- perimetro di chi ha già accesso, e il danno massimo è muovere un proprio dato fra
-- proprie organizzazioni. Qui invece la colonna non vincolata *è* il confine.
--
-- CORREZIONE
-- Non si irrigidisce il `WITH CHECK`: una policy non può confrontare la riga nuova
-- con quella vecchia, quindi ogni tentativo in quella direzione finirebbe per
-- dipendere da sottigliezze di snapshot. Si toglie invece al ruolo applicativo il
-- privilegio di aggiornare quelle colonne. Il permesso è verificato prima delle
-- policy, quindi la manovra fallisce con `permission denied` qualunque cosa la
-- policy ammetta.
--
-- L'elenco concesso è esattamente ciò che i repository aggiornano oggi
-- (`crates/infrastructure/src/db/repositories/{invitation,lease_party}.rs`): la
-- suite completa passa invariata. Aggiungere una colonna a una di quelle `SET`
-- richiederà di aggiungerla anche qui, ed è voluto — è il punto in cui ci si ferma a
-- chiedersi se quella colonna debba davvero essere scrivibile dall'applicazione.
--
-- `propflow_worker` non viene toccato: ha `BYPASSRLS` ed è il ruolo di fiducia per
-- definizione. Nessuno dei flussi che lo usano scrive su queste tabelle.
-- =============================================================================

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_app') THEN
    RAISE NOTICE 'ruolo propflow_app assente: privilegi non applicati (ambiente di sviluppo)';
    RETURN;
  END IF;

  -- Invito: si può solo marcarlo accettato o revocato. Restano fuori org_id,
  -- email, role, token_hash, lease_id ed expires_at, cioè tutto ciò che definisce
  -- *quale* invito è e a cosa dà diritto.
  REVOKE UPDATE ON app.org_invitations FROM propflow_app;
  GRANT  UPDATE (accepted_at, accepted_by, revoked_at)
      ON app.org_invitations TO propflow_app;

  -- Parte contrattuale: si possono aggiornare i dati della persona e il
  -- collegamento all'utente. Restano fuori org_id e lease_id, cioè a *quale*
  -- contratto e a quale organizzazione la riga appartiene.
  REVOKE UPDATE ON app.lease_parties FROM propflow_app;
  GRANT  UPDATE (user_id, role, full_name, email, phone, share_percent, is_primary,
                 joined_at, left_at, deleted_at)
      ON app.lease_parties TO propflow_app;
END $$;
