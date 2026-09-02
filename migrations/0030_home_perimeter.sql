-- =============================================================================
-- Il perimetro della casa: chi ci abita si vede, chi abita altrove no.
--
-- Due metà di una cosa sola, e per questo una migration sola. Oggi sono sbagliate
-- tutte e due, in direzioni opposte:
--
--   1. un coinquilino **non** ha modo di sapere chi altro abita casa sua —
--      `can_access_lease` è per contratto, e in un co-living per stanza ognuno ha
--      il proprio;
--   2. lo stesso coinquilino legge la riga `app.users` di **ogni** persona
--      dell'organizzazione — inquilini di altri immobili compresi — perché
--      `users_select` ammette `shares_org_with`.
--
-- La prima metà è una capacità che manca, la seconda un pavimento che non c'è.
--
-- PERCHÉ UNA FUNZIONE CHE RESTITUISCE RIGHE, E NESSUNA POLICY ALLARGATA
-- La via naturale era estendere `lease_parties_select` al perimetro
-- dell'immobile, riusando `can_access_property`. **È stata provata sul database e
-- non funziona**, e le ragioni sono due a due profondità diverse.
--
-- *Prima profondità.* Scritta in linea nella policy, la condizione è
--
--   EXISTS (SELECT 1 FROM app.leases l
--            WHERE l.id = lease_parties.lease_id
--              AND app.can_access_property(l.property_id))
--
-- e non compare **nemmeno una riga in più**: una sottoquery dentro una policy
-- subisce la Row Level Security della tabella che legge, quindi quel `SELECT`
-- passa da `leases_select`, che è `can_access_lease(id)` — falsa proprio per i
-- contratti dei coinquilini che si sta cercando di raggiungere. È la trappola
-- della `0026`, qui alla terza occorrenza.
--
-- *Seconda profondità.* Superata la prima con una funzione `SECURITY DEFINER` che
-- risolve il contratto nel suo immobile, le righe compaiono — ma solo finché la
-- query non tocca `app.leases`:
--
--   app.lease_parties da sola                        X, Y, Z ✓
--   unita a app.leases (per filtrare l'immobile)     solo X  ✗
--
-- e unirle è obbligatorio, perché l'immobile e «in essere» stanno sul contratto.
-- `leases_select` è rimasta per contratto e rifiltra tutto. Il sintomo è un
-- elenco che sembra vuoto **senza alcun errore**.
--
-- IL PREZZO CHE NON SI PAGA
-- Far funzionare quella via avrebbe richiesto di allargare **anche**
-- `leases_select` al perimetro dell'immobile. Misurato: da quel momento X legge
-- `rent_amount` dei contratti di Y e Z, cioè **il canone dei coinquilini**. Una
-- policy è per riga e non per colonna, e `leases.rent_amount` sta sulla riga.
--
-- Si fa invece una funzione che **proietta**: `app.property_residents` restituisce
-- tre colonne scelte e nient'altro, con il confine dentro. Nessuna riga in più
-- diventa leggibile, quindi né il canone né l'email escono — non perché li si
-- ometta per prudenza, ma perché non c'è nulla da cui ometterli.
--
-- IL CONFINE RESTA NEL DATABASE
-- La funzione è l'unica via all'elenco e il controllo è nel suo corpo: non è una
-- regola del service layer che una query scritta a mano potrebbe aggirare.
-- =============================================================================

-- ---------------------------------------------------------------------------
-- Prima parte: i residenti di un immobile
-- ---------------------------------------------------------------------------

CREATE FUNCTION app.property_residents(p_property_id uuid)
RETURNS TABLE (user_id uuid, full_name text, room_name text)
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT lp.user_id, lp.full_name, r.name
    FROM app.properties p
    JOIN app.leases l        ON l.property_id = p.id
    JOIN app.lease_parties lp ON lp.lease_id = l.id
    LEFT JOIN app.rooms r    ON r.id = l.room_id AND r.deleted_at IS NULL
   WHERE p.id = p_property_id
     AND p.deleted_at IS NULL
     -- L'immobile deve stare nell'organizzazione **attiva**, e non solo in una a
     -- cui il chiamante appartiene: è la stessa anchor di `properties_select`, e
     -- la ragione è quella scritta in testa alla `0026` — `current_org_id()` da
     -- sola è una dichiarazione, ma senza di essa una richiesta fatta nel
     -- contesto dell'organizzazione B leggerebbe righe dell'organizzazione A.
     -- `can_access_property` verifica un rapporto con l'organizzazione
     -- dell'immobile, non che sia quella attiva: le due condizioni servono
     -- entrambe.
     AND p.org_id = app.current_org_id()
     -- Il confine, dentro la funzione: risponde solo a chi vede già l'immobile.
     -- Lo staff dell'organizzazione, oppure chi ci abita — anche solo per stanza.
     AND app.can_access_property(p.id)
     AND l.deleted_at IS NULL
     -- «In essere» passa da `app.lease_in_force` e da nessun'altra parte: è
     -- l'unica definizione del progetto (ADR-019). Confrontare le date a mano
     -- creerebbe la seconda definizione che quella funzione esiste per impedire,
     -- e un inquilino uscito due anni fa comparirebbe ancora fra i residenti —
     -- lo stato memorizzato resta 'active' perché nessuno scheduler lo cambia.
     AND app.lease_in_force(l.status, l.start_date, l.end_date)
     AND lp.deleted_at IS NULL
     -- Chi ha lasciato il contratto non è un residente. `left_at IS NULL` e non
     -- un confronto con `CURRENT_DATE`: è il filtro che il dominio usa già in
     -- `LeaseParty::ensure_tenancy_complete`, e `lease_parties_dates_ck` non dice
     -- se `left_at` sia inclusiva o esclusiva — inventare qui una semantica del
     -- confine sarebbe una seconda definizione per una colonna che oggi nessun
     -- caso d'uso valorizza.
     AND lp.left_at IS NULL
     -- Il garante non abita lì. `LeasePartyRole::shares_rent()` fa già questa
     -- distinzione nel dominio, ed è la stessa ragione per cui il proprietario
     -- non compare: l'elenco è di chi ci vive, non di chi ha firmato.
     AND lp.role <> 'guarantor'
$$;

COMMENT ON FUNCTION app.property_residents IS
  'I residenti di un immobile: chi ha una parte contrattuale non conclusa su un
   contratto in essere, garanti esclusi. Restituisce righe e non un booleano, ed è
   il punto: **proietta** tre colonne invece di rendere leggibili le righe da cui
   vengono, quindi né il canone né l''email dei coinquilini escono. Allargare
   `lease_parties_select` avrebbe richiesto di allargare anche `leases_select`, e
   con essa `rent_amount`. SECURITY DEFINER perché deve leggere `app.leases` di
   altri, che `leases_select` restringe al singolo contratto.
   `user_id` è NULL per chi è stato invitato e non ha ancora accettato: ha un nome
   e nessun identificatore, quindi non è aggiungibile a un gruppo di spesa.
   L''elenco ha una riga per parte contrattuale: chi affitta due stanze dello
   stesso immobile compare due volte, con la stanza a distinguerle.';

-- I privilegi su una funzione nuova non arrivano dal `GRANT ... ON ALL FUNCTIONS`
-- della 0015, che si applica solo a ciò che esisteva allora: senza questo, la
-- concessione dipenderebbe dal default di PUBLIC invece di essere dichiarata.
DO $$
DECLARE r text;
BEGIN
  FOREACH r IN ARRAY ARRAY['propflow_app', 'propflow_worker'] LOOP
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = r) THEN
      EXECUTE format(
        'GRANT EXECUTE ON FUNCTION app.property_residents(uuid) TO %I', r);
    END IF;
  END LOOP;
END $$;

-- ---------------------------------------------------------------------------
-- Seconda parte: `users_select` si restringe dall'organizzazione alla casa
--
-- PERCHÉ, VISTO CHE OGGI NON TRAPELA NIENTE
-- Nessun endpoint espone l'anagrafica di un'altra persona, e nessuna query legge
-- `app.users` per qualcuno che non sia il chiamante — le tre query su quella
-- tabella stanno tutte in `user.rs`, tutte nei flussi di autenticazione. Quindi
-- restringere non corregge una fuga in corso: chiude un **pavimento**. La riga
-- `app.users` porta nome, **email** e **telefono**, e `shares_org_with` la mostra
-- oggi a ogni membro dell'organizzazione, inquilini di immobili diversi compresi.
--
-- Questo progetto ha appena passato una migration — la `0029` — a smettere di
-- fidarsi di regole applicative su confini che il database lascia aperti.
-- Applicare qui un metro diverso sarebbe incoerente.
--
-- IL VERDE DELLA SUITE NON È LA PROVA CHE SIA GIUSTO
-- Restringendo, **nessun test fallisce**. Va letto per quello che è: non prova che
-- la restrizione sia sicura, prova che la capacità per cui `shares_org_with`
-- esiste — «mostrare i nomi delle controparti», dice il commento della `0014` —
-- **non è mai stata costruita**. La finestra per restringere è aperta e la chiude
-- il primo client che legga un nome da lì. La verifica vera è la matrice dello
-- scenario a due immobili, non la suite.
--
-- I NOMI NON SERVONO DA QUI, ED È IL MOTIVO PER CUI NESSUNO LI LEGGE
-- Lo schema denormalizza i nomi **dove si mostrano**: `lease_parties.full_name` ed
-- `email`, `expense_group_members.display_name`. `app.property_residents` proietta
-- da `lease_parties`, non da `app.users`: le due metà di questa migration sono
-- indipendenti, e nessuna ha bisogno dell'altra per funzionare.
--
-- I DUE RAMI SULLO STAFF NON SONO DECORAZIONE
-- Senza `is_staff_of_user` il proprietario perderebbe di vista i propri inquilini,
-- che non abitano con lui. Senza `is_my_staff` un inquilino non potrebbe **mai**
-- sapere il nome del proprio proprietario, e `properties.owner_id` punterebbe a
-- una riga illeggibile.
--
-- Le due funzioni **non sono la stessa al contrario**, e scambiarle produce una
-- policy che sembra funzionare finché il chiamante non è un inquilino:
--   `is_staff_of_user(u)`  io sono staff di un'organizzazione a cui u appartiene;
--   `is_my_staff(u)`       u è staff di un'organizzazione a cui io appartengo.
-- ---------------------------------------------------------------------------

CREATE FUNCTION app.shares_home_with(p_user_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT EXISTS (
    SELECT 1
      FROM app.properties p
      JOIN app.leases mine   ON mine.property_id  = p.id AND mine.deleted_at IS NULL
      JOIN app.lease_parties me ON me.lease_id = mine.id
                               AND me.user_id = app.current_user_id()
                               AND me.deleted_at IS NULL
      JOIN app.leases theirs ON theirs.property_id = p.id AND theirs.deleted_at IS NULL
      JOIN app.lease_parties them ON them.lease_id = theirs.id
                                 AND them.user_id = p_user_id
                                 AND them.deleted_at IS NULL
     WHERE p.deleted_at IS NULL
       -- Come in `app.property_residents`: l'immobile deve stare
       -- nell'organizzazione **attiva**. Bypassare la RLS significa bypassare
       -- l'isolamento, non solo la visibilità per ruolo.
       AND p.org_id = app.current_org_id()
  )
$$;

COMMENT ON FUNCTION app.shares_home_with IS
  'Vero se questa persona e il chiamante hanno entrambi una parte contrattuale su
   contratti dello stesso immobile: è la nozione di «casa» che
   `app.can_access_property` usa già per ammettere l''inquilino, e riscriverla
   sarebbe la seconda definizione che ADR-019 esiste per impedire.
   **Non filtra `app.lease_in_force`, e la differenza con `app.property_residents`
   è voluta**: l''elenco dei residenti dice chi ci abita *adesso*, questa dice con
   chi hai abitato — e un conto condiviso sopravvive alla scadenza del contratto,
   quindi restringere qui renderebbe illeggibile il nome di chi si sta ancora
   pagando. È un pavimento, non un elenco.
   SECURITY DEFINER perché deve leggere le parti contrattuali di **altri**, che
   oggi nessuno vede.';

CREATE FUNCTION app.is_staff_of_user(p_user_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT EXISTS (
    SELECT 1
      FROM app.org_members m
     WHERE m.user_id = p_user_id
       AND m.status = 'active'
       AND m.deleted_at IS NULL
       AND app.is_org_staff(m.org_id)
  )
$$;

COMMENT ON FUNCTION app.is_staff_of_user IS
  'Vero se il chiamante è staff di un''organizzazione a cui questa persona
   appartiene: è il ramo che tiene il proprietario in vista dei propri inquilini,
   che non abitano con lui. Riusa `app.is_org_staff` invece di riscriverne il
   predicato — che vi comprende anche l''amministratore di piattaforma è
   ridondante con il ramo che `users_select` ha già, e una quarta scrittura di
   «staff» costerebbe più della ridondanza.';

CREATE FUNCTION app.is_my_staff(p_user_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT EXISTS (
    SELECT 1
      FROM app.org_members m
     WHERE m.user_id = p_user_id
       AND m.status = 'active'
       AND m.deleted_at IS NULL
       AND m.role = ANY (ARRAY['owner','manager']::app.org_role[])
       AND app.is_org_member(m.org_id)
  )
$$;

COMMENT ON FUNCTION app.is_my_staff IS
  'Vero se questa persona è staff di un''organizzazione a cui il chiamante
   appartiene: senza, un inquilino non potrebbe mai sapere il nome del proprio
   proprietario, e `properties.owner_id` punterebbe a una riga illeggibile. **Non
   è `app.is_staff_of_user` al contrario**: là è il chiamante a essere staff, qui
   è l''altro. Scambiarle produce una policy che sembra funzionare finché il
   chiamante non è un inquilino.';

-- `ALTER POLICY` e non `DROP`/`CREATE`: la policy resta la stessa, con il nome che
-- le query di verifica già conoscono, e non c'è una finestra in cui la tabella è
-- priva di policy di SELECT.
--
-- Nessun `deleted_at IS NULL`, e non è una svista: la `0016` lo ha tolto da tutte
-- le policy di SELECT, e il controllo 9 delle invarianti impedisce che rientri.
ALTER POLICY users_select ON app.users
    USING (
      -- io stesso
      id = app.current_user_id()
      -- chi abita la mia stessa casa
      OR app.shares_home_with(id)
      -- lo staff vede le persone delle proprie organizzazioni
      OR app.is_staff_of_user(id)
      -- e l'inquilino vede lo staff della propria
      OR app.is_my_staff(id)
      OR app.is_platform_admin()
    );

-- `app.shares_org_with` resta definita e **non viene rimossa**: la `0014` la
-- dichiara e toglierla è un cambiamento a sé, che va fatto quando si è certi che
-- nessuna policy futura la voglia. Dopo questa migration non è più usata da
-- nessuna policy.

DO $$
DECLARE r text; f text;
BEGIN
  FOREACH r IN ARRAY ARRAY['propflow_app', 'propflow_worker'] LOOP
    CONTINUE WHEN NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = r);
    FOREACH f IN ARRAY ARRAY['shares_home_with', 'is_staff_of_user', 'is_my_staff'] LOOP
      EXECUTE format('GRANT EXECUTE ON FUNCTION app.%I(uuid) TO %I', f, r);
    END LOOP;
  END LOOP;
END $$;
