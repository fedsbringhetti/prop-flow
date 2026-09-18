-- =============================================================================
-- Il ramo dell'inquilino guarda la membership, come quello dello staff.
--
-- IL FATTO
-- Chi esce da un'organizzazione continua a leggerne i dati — e a scriverci —
-- per i quindici minuti del suo access token. La catena: `middleware/context.rs`
-- si fida del ruolo nel token quando `X-Org-Id` coincide con l'`org_id` dei
-- claim, senza rileggere la membership (il baratto del JWT non revocabile,
-- ADR-012, che parla di *validità* del token e non di *membership*); e il ramo
-- dell'inquilino di `app.can_access_lease` e `app.can_access_property` (`0014`)
-- passa da `lease_parties.user_id = app.current_user_id()` **senza guardare
-- `org_members`**. `LeaveOrganization` (`0039`) tocca `org_members` e non le
-- parti, quindi il ramo resta vero dopo l'uscita.
--
-- Prima di ADR-053 il caso era raro: il token di un inquilino portava la sua
-- organizzazione personale, mai quella di chi lo ospitava, e la rilettura dal
-- vivo scattava sempre. Ora porta quella di chi lo ospita, ed è il caso di
-- **ogni** inquilino che esce.
--
-- È diverso dal poscritto della sezione 6 sullo staff: là le policy passano da
-- `member_role` e restituiscono **vuoto**; qui restituiscono **dati**. Questa
-- migration allinea l'inquilino a ciò che quel poscritto già assume.
--
-- LA VIA SCELTA, E QUELLA SCARTATA
-- `AND app.is_org_member(<org>)` nel ramo `EXISTS` dell'inquilino delle due
-- funzioni. Chiude al livello della Row Level Security, dove una lettura non può
-- aggirarla, e tocca solo il ramo che **non** guardava la membership.
--
-- Scartato: rileggere la membership in `Authenticated` a ogni richiesta. Cambia
-- la forma del baratto di ADR-012 per tutte le rotte, staff compreso, dove le
-- policy passano già da `member_role` e restituiscono vuoto: il difetto è solo
-- nel ramo dell'inquilino, e si chiude lì.
--
-- NON SI TOCCA `member_role`. È usata da tutte le policy dello staff, e un
-- cambiamento lì cambia tutto; il difetto sta nel ramo che **non** la usa.
--
-- `CREATE OR REPLACE FUNCTION` con la **stessa firma**: le policy referenziano
-- la funzione per oid, quindi non vanno né toccate né ricreate, e non c'è una
-- finestra in cui una tabella è priva di policy. Il resto del corpo è identico
-- alla `0014`: si riscrive per intero perché `CREATE OR REPLACE` non ammette una
-- modifica parziale.
--
-- IL COSTO
-- Una lettura in più di `org_members` per riga, sull'indice che `member_role`
-- usa già. Le funzioni sono `SECURITY DEFINER` e `is_org_member` anche.
--
-- CHE COSA CAMBIA PER CHI È DENTRO: NIENTE
-- Una parte di un contratto con la membership attiva passa esattamente come
-- prima. Cambia solo chi è parte di un contratto **e** non è più membro: prima
-- vedeva, ora no. Nessun invariante dello schema guarda il corpo delle
-- funzioni.
-- =============================================================================

CREATE OR REPLACE FUNCTION app.can_access_lease(p_lease_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT EXISTS (
    SELECT 1
      FROM app.leases l
     WHERE l.id = p_lease_id
       AND (
         app.is_org_staff(l.org_id)
         OR (
           -- Parte del contratto **e** ancora membro: chi è uscito ha ancora la
           -- riga in `lease_parties`, e senza il secondo termine la vedrebbe.
           app.is_org_member(l.org_id)
           AND EXISTS (
             SELECT 1 FROM app.lease_parties lp
              WHERE lp.lease_id = l.id
                AND lp.user_id = app.current_user_id()
                AND lp.deleted_at IS NULL
           )
         )
       )
  )
$$;

COMMENT ON FUNCTION app.can_access_lease IS
  'Chi vede un contratto: lo staff dell''organizzazione, e chi ne è parte **se è
   ancora membro** dell''organizzazione. Il secondo termine esiste perché uscire
   (0039) tocca org_members e non lease_parties: senza, chi è uscito leggerebbe
   il contratto finché il suo access token non scade (0041).';

CREATE OR REPLACE FUNCTION app.can_access_property(p_property_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT EXISTS (
    SELECT 1
      FROM app.properties p
     WHERE p.id = p_property_id
       AND (
         app.is_org_staff(p.org_id)
         -- l'inquilino vede l'immobile in cui abita (anche solo per stanza),
         -- finché è membro dell'organizzazione: vedi can_access_lease.
         OR (
           app.is_org_member(p.org_id)
           AND EXISTS (
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
  )
$$;

COMMENT ON FUNCTION app.can_access_property IS
  'Chi vede un immobile: lo staff dell''organizzazione, e chi vi abita per un
   contratto **se è ancora membro** dell''organizzazione. È la funzione che
   tickets_insert consulta: senza il termine sulla membership, chi è uscito
   potrebbe aprire una segnalazione nell''organizzazione lasciata (0041).';
