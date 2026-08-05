-- =============================================================================
-- Accettazione di un invito: tre scritture in un'organizzazione di cui non si è
-- ancora membri.
--
-- PROBLEMA
-- Accettare un invito richiede di creare la propria membership, collegare la
-- parte contrattuale e marcare l'invito. Le policy esistenti vietano tutte e tre:
-- `org_members_insert` ammette solo lo staff o il bootstrap del primo owner,
-- `org_invitations_staff` è `FOR ALL` con `is_org_staff`, e
-- `lease_parties_update` richiede lo staff.
--
-- PERCHÉ NON IL POOL PRIVILEGIATO
-- La regola 20 di CLAUDE.md limita `begin_privileged()` a tre flussi, ed è ciò che
-- ne tiene piccola la superficie. Quei tre esistono perché un contesto di
-- sicurezza non *può* esistere: avvengono prima dell'autenticazione. Qui il
-- contesto esiste — chi accetta è autenticato — e manca solo la membership. La
-- risposta corretta è estendere la RLS, non aggirarla. Vedi ADR-020.
--
-- DUE CONTROLLI INDIPENDENTI
-- Il token resta verificato dall'applicazione, per confronto sullo hash: la policy
-- non lo vede. L'applicazione prova che chi chiama *possiede* il token; la RLS
-- prova che esiste davvero un invito pendente per quell'email. Nessuno dei due da
-- solo basterebbe.
--
-- CONFINE DESTRO DELLE POLICY DI SELECT
-- Le policy di lettura dell'invitato **non** possono dipendere dallo stato
-- "pendente". Marcare l'invito accettato produrrebbe una riga che chi la scrive non
-- potrebbe più rileggere, ed è esattamente il difetto di ADR-008. La lettura si
-- basa quindi sull'email, che non cambia con l'operazione; il vincolo "pendente"
-- vive nelle `USING` degli UPDATE, dove si applica alla riga *esistente*.
-- =============================================================================

-- SECURITY DEFINER: deve leggere `org_invitations`, che è visibile solo allo staff,
-- e `users`, dove la policy mostra una riga solo a chi è già identificato.
CREATE FUNCTION app.has_pending_invitation(p_org_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT EXISTS (
    SELECT 1
      FROM app.org_invitations i
      JOIN app.users u ON u.email = i.email
     WHERE i.org_id = p_org_id
       AND u.id = app.current_user_id()
       AND u.deleted_at IS NULL
       AND i.accepted_at IS NULL
       AND i.revoked_at IS NULL
       AND i.expires_at > now()
  )
$$;

COMMENT ON FUNCTION app.has_pending_invitation IS
  'Vero se l''utente corrente ha un invito pendente per questa organizzazione.
   Abilita le tre scritture dell''accettazione senza passare dal pool privilegiato
   (ADR-020). Il possesso del token lo verifica l''applicazione, non questa policy.';

-- L'email dell'utente corrente, per confrontarla con quella dell'invito.
-- Come sopra: `users` è protetta, e una policy non può leggerla in linea senza
-- ricorrere su se stessa.
CREATE FUNCTION app.current_user_email() RETURNS app.email_address
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT u.email
    FROM app.users u
   WHERE u.id = app.current_user_id()
     AND u.deleted_at IS NULL
$$;

-- ---------------------------------------------------------------------------
-- Invito: l'invitato legge e marca il proprio.
--
-- Nessun vincolo su `current_org_id()`: chi accetta arriva con il contesto della
-- *propria* organizzazione personale, perché di quella dell'invito non è ancora
-- membro e il middleware rifiuterebbe di selezionarla. L'isolamento qui è
-- l'email, che è più stretta di un'organizzazione: identifica la singola persona.
-- ---------------------------------------------------------------------------
CREATE POLICY org_invitations_invitee_select ON app.org_invitations FOR SELECT
    USING (email = app.current_user_email());

CREATE POLICY org_invitations_invitee_accept ON app.org_invitations FOR UPDATE
    USING (email = app.current_user_email() AND app.has_pending_invitation(org_id))
    WITH CHECK (email = app.current_user_email());

-- ---------------------------------------------------------------------------
-- Membership: iscrizione di sé stessi come inquilino, e solo così.
-- `role = 'tenant'` è nella policy e non solo nel codice: un invito in mano non
-- deve poter diventare un ruolo di staff nemmeno con una query scritta a mano.
-- ---------------------------------------------------------------------------
CREATE POLICY org_members_accept_invitation ON app.org_members FOR INSERT
    WITH CHECK (
      user_id = app.current_user_id()
      AND role = 'tenant'
      AND app.has_pending_invitation(org_id)
    );

-- ---------------------------------------------------------------------------
-- Parte contrattuale: collegamento del proprio `user_id`.
--
-- `user_id IS NULL` nella USING rende l'operazione non ripetibile su una parte
-- già collegata: un invito non trasferisce a sé il contratto di qualcun altro.
-- La SELECT dell'invitato serve a rileggere la riga appena scritta, che la policy
-- generale non coprirebbe: `can_access_lease` interroga `lease_parties` con lo
-- snapshot dello statement in corso, dove il collegamento non è ancora visibile.
-- ---------------------------------------------------------------------------
CREATE POLICY lease_parties_invitee_select ON app.lease_parties FOR SELECT
    USING (email = app.current_user_email() AND app.is_org_member(org_id));

CREATE POLICY lease_parties_invitee_link ON app.lease_parties FOR UPDATE
    USING (
      email = app.current_user_email()
      AND user_id IS NULL
      AND app.has_pending_invitation(org_id)
    )
    WITH CHECK (
      email = app.current_user_email()
      AND user_id = app.current_user_id()
    );

-- ---------------------------------------------------------------------------
-- Outbox: l'evento dell'accettazione porta l'organizzazione che ha invitato, non
-- quella attiva nel contesto.
--
-- Chi accetta arriva con il contesto della propria organizzazione personale, e la
-- policy `domain_events_insert` ammetteva solo `org_id = app.current_org_id()`:
-- l'accettazione riusciva nelle tre scritture e poi si rompeva sull'annuncio,
-- rifiutata dalla RLS proprio nell'ultimo statement. Un evento non può essere
-- opzionale — è nella stessa transazione del comando, quindi il fallimento
-- annullava tutto.
--
-- La condizione aggiunta è `is_org_member` e non `has_pending_invitation`: quando
-- l'evento viene scritto la membership esiste già, e legarla all'invito renderebbe
-- l'ordine delle scritture parte del contratto con il database. "Posso annunciare
-- un fatto di un'organizzazione di cui sono membro" è comunque isolamento, ed è
-- anche più corretto del vincolo sull'organizzazione *attiva*.
-- ---------------------------------------------------------------------------
ALTER POLICY domain_events_insert ON app.domain_events
    WITH CHECK (
      org_id IS NULL
      OR org_id = app.current_org_id()
      OR app.is_org_member(org_id)
    );

-- I privilegi su una funzione nuova non arrivano dal `GRANT ... ON ALL FUNCTIONS`
-- della 0015, che si applica solo a ciò che esisteva allora.
DO $$
DECLARE r text;
BEGIN
  FOREACH r IN ARRAY ARRAY['propflow_app', 'propflow_worker'] LOOP
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = r) THEN
      EXECUTE format(
        'GRANT EXECUTE ON FUNCTION app.has_pending_invitation(uuid) TO %I', r);
      EXECUTE format(
        'GRANT EXECUTE ON FUNCTION app.current_user_email() TO %I', r);
    END IF;
  END LOOP;
END $$;
