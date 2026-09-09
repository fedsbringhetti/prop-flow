-- =============================================================================
-- La visibilità di una segnalazione diventa una scelta di chi la apre.
--
-- IL FATTO, CHE NON È UN DIFETTO
-- Una segnalazione aperta sull'immobile intero — la caldaia condominiale, la
-- serratura del portone — non la vede nessun altro inquilino. Non è una svista:
-- è ciò che `app.can_access_ticket` dice dalla `0014`. Concede l'accesso allo
-- staff, a chi ha segnalato, a chi è assegnato, e poi a
-- `t.lease_id IS NOT NULL AND can_access_lease(...)` — quindi il quarto ramo
-- passa dal **contratto**, e una segnalazione che non ne indica uno resta
-- privata.
--
-- Quel default è giusto e **resta**: un guasto dentro casa propria non riguarda i
-- vicini di pianerottolo. Si aggiunge la scelta esplicita per il caso vero.
--
-- PERCHÉ IL RAMO STA NELLA FUNZIONE E NON IN `tickets_select`
-- È la parte che si sbaglia. `app.can_access_ticket` non serve una policy sola:
-- la chiamano `tickets_select`, `ticket_messages_select`,
-- `ticket_status_history_select` e `can_access_attachment('ticket', …)`.
-- Allargare la sola policy di riga darebbe a un coinquilino una segnalazione
-- **che si apre e non si legge** — niente thread, niente storico, niente
-- fotografie del guasto — e nessun errore che spieghi perché.
--
-- `CREATE OR REPLACE FUNCTION` con la **stessa firma**: le policy la referenziano
-- per oid, quindi non vanno né toccate né ricreate, e non c'è una finestra in cui
-- una tabella è priva di policy.
--
-- CHE COSA LA CONDIVISIONE DÀ, E CHE COSA NON DÀ
-- Dà anche il **thread** e le **fotografie**, ed è voluto: una visibilità che
-- permetta di leggere «ho chiamato il tecnico» ma non di rispondere «da me non
-- funziona ancora» sarebbe una bacheca, non una segnalazione condivisa.
-- `ticket_messages_insert` passa dalla stessa funzione, quindi un residente può
-- scrivere nel thread.
--
-- Non dà **nessuna scrittura sulla segnalazione**: `tickets_update` ha un
-- predicato in linea — staff, `reported_by`, `assigned_to` — e non passa da
-- `can_access_ticket`. Un residente non riapre, non annulla, non corregge e non
-- chiude. È la stessa asimmetria che il modulo ha già per il coinquilino sullo
-- stesso contratto.
--
-- Le **note interne** restano invisibili: `ticket_messages_select` ha un `AND` a
-- parte su `is_internal`, che questo ramo non tocca.
--
-- PERCHÉ `app.property_residents` E NON `app.shares_home_with`
-- La `0030` ha introdotto due predicati sul perimetro della casa, e vanno scelti
-- e non riscritti: una terza definizione di «chi abita qui» è la trappola che
-- quel file documenta alla sua terza occorrenza.
--
-- `shares_home_with(p_user_id)` è **utente↔utente**: userebbe `reported_by` come
-- termine di confronto, e ne verrebbero due difetti. `reported_by` è
-- `ON DELETE SET NULL`, quindi la condivisione sparirebbe da sé il giorno in cui
-- l'autore viene cancellato. E quella funzione **non filtra `lease_in_force`** —
-- è deliberato, il suo commento lo dice, perché è un *pavimento* per la
-- leggibilità dei nomi e non un elenco di chi ci abita adesso: un inquilino
-- uscito due anni fa vedrebbe le segnalazioni nuove della casa in cui non vive
-- più.
--
-- `property_residents(p_property_id)` è ancorata all'**immobile**, che è la
-- dimensione giusta perché `property_id` è obbligatorio su ogni segnalazione.
-- Filtra `lease_in_force`, filtra `left_at IS NULL`, esclude i garanti, e ha già
-- `can_access_property` come confine nel corpo.
--
-- Ne segue che, come per ogni funzione con il confine dentro, sul **pool
-- privilegiato** — che non ha contesto — questo ramo risponde «no». È la lezione
-- che la sezione 4 ha lasciato in un test su `v_property_occupancy`: chi verifica
-- la condivisione deve installare `app.current_org_id` **e**
-- `app.current_user_id`, perché `can_access_property` passa da `is_org_staff`.
--
-- L'ORDINE DEI RAMI NON È FORMATTAZIONE
-- Il booleano va scritto per primo e il ramo va per ultimo nell'`OR`.
-- `can_access_ticket` è `COST 500` e viene valutata **per riga** nell'elenco
-- delle segnalazioni: con `visible_to_residents` davanti, la funzione che
-- restituisce righe non viene chiamata affatto sulle segnalazioni private, che
-- sono il default e saranno la maggioranza.
--
-- Vedi ADR-047.
-- =============================================================================

-- `NOT NULL DEFAULT false`: il default **è** la decisione, non un ripiego. Una
-- segnalazione nasce privata, e chi la apre può dichiararla di interesse comune.
--
-- Il nome dice «residenti» e non «tutti gli inquilini»: quest'ultima formula si
-- legge come «dell'organizzazione», che sarebbe un confine più largo e sbagliato
-- — un inquilino di un'altra casa dello stesso proprietario non c'entra niente.
-- La casella nel form usa invece la parola comune, perché lì il confine è già
-- chiaro dal contesto.
ALTER TABLE app.tickets
    ADD COLUMN visible_to_residents BOOLEAN NOT NULL DEFAULT false;

COMMENT ON COLUMN app.tickets.visible_to_residents IS
  'Se vero, la segnalazione è leggibile da chi risulta residente nel suo immobile
   secondo app.property_residents — e con essa il thread, lo storico e gli
   allegati, perché app.can_access_ticket serve tutte e quattro quelle policy.
   Non concede alcuna scrittura sulla riga: tickets_update ha un predicato
   proprio. Default false: il comportamento di prima di questa migration.';

-- Il quinto ramo. Il resto della funzione è identico alla `0014`: si riscrive per
-- intero perché `CREATE OR REPLACE` non ammette una modifica parziale, non perché
-- qualcosa degli altri quattro rami sia cambiato.
CREATE OR REPLACE FUNCTION app.can_access_ticket(p_ticket_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT EXISTS (
    SELECT 1
      FROM app.tickets t
     WHERE t.id = p_ticket_id
       AND (
         app.is_org_staff(t.org_id)
         OR t.reported_by = app.current_user_id()
         OR t.assigned_to = app.current_user_id()
         OR (t.lease_id IS NOT NULL AND app.can_access_lease(t.lease_id))
         -- Ultimo, e dietro al booleano: vedi la nota sull'ordine in testa.
         OR (t.visible_to_residents
             AND EXISTS (
               SELECT 1
                 FROM app.property_residents(t.property_id) pr
                WHERE pr.user_id = app.current_user_id()
             ))
       )
  )
$$;

COMMENT ON FUNCTION app.can_access_ticket IS
  'Chi vede una segnalazione: lo staff, chi l''ha aperta, chi ce l''ha in carico,
   chi è parte del contratto indicato, e — se la segnalazione è dichiarata di
   interesse comune — chi risulta residente nel suo immobile. È la funzione che
   quattro policy consultano (tickets_select, ticket_messages_select,
   ticket_status_history_select, can_access_attachment), ed è la ragione per cui
   il ramo della visibilità sta qui e non in una policy sola. Non dice nulla su
   chi possa **scrivere**: quello è il predicato in linea di tickets_update.';

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_app') THEN
    RAISE NOTICE 'ruolo propflow_app assente: privilegi non applicati (ambiente di sviluppo)';
    RETURN;
  END IF;

  -- Si aggiunge alle dieci colonne già concesse dalla `0020` e dalla `0021`. La
  -- casella si può cambiare dopo l'apertura — è uno dei tre campi di
  -- `PATCH /tickets/{id}/report` — quindi la colonna deve essere scrivibile.
  -- **Non** entra fra le colonne strutturali del controllo 12: quelle sono le
  -- colonne che non hanno un motivo legittimo di cambiare, e questa ce l'ha.
  GRANT UPDATE (visible_to_residents) ON app.tickets TO propflow_app;
END $$;
