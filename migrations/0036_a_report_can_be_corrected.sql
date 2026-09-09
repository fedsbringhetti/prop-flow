-- =============================================================================
-- Il testo di una segnalazione si corregge, finché nessuno l'ha presa in carico.
--
-- QUESTA MIGRATION REVOCA UNA DECISIONE, e va letta con la `0020` davanti.
--
-- IL FATTO
-- Chi apre una segnalazione con un refuso nel titolo non può correggerlo, e non
-- perché manchi una rotta: `title` e `description` **non sono scrivibili dal
-- ruolo applicativo**. La `0020` gliele ha tolte con questa ragione scritta:
--
--   «una segnalazione aperta sull'immobile sbagliato si annulla e si riapre»
--
-- A difendere quella scelta ci sono altre due cose oltre alla migration: il
-- controllo `colonna-strutturale-scrivibile` di
-- `ops/sql/checks/schema_invariants.sql`, e il test
-- `the_application_role_cannot_move_a_ticket_to_another_property`, che asserisce
-- il `permission denied`. Entrambi vengono emendati insieme a questo file, e
-- l'emendamento **è** il documento della revoca.
--
-- CHE COSA CAMBIA, E CHE COSA NO
-- Il criterio di ADR-026 **non cambia**: il privilegio di colonna serve «quando
-- una colonna non ha un motivo legittimo di cambiare». Cambia il fatto — ora ce
-- l'ha. L'argomento «si annulla e si riapre» vale per l'**immobile**, dove
-- riaprire è l'unico modo di rimettere insieme una riga coerente; non vale per un
-- refuso, dove costa a chi segnala di rifare tutto e di perdere il riferimento
-- `TK-000042` che ha già comunicato a voce.
--
-- Restano fuori `property_id`, `room_id`, `lease_id`, `reported_by`,
-- `reported_by_kind`, `reference`, `currency` e `deleted_at`, e per loro la frase
-- della `0020` resta vera alla lettera. La causa radice che ADR-026 descrive —
-- **non esiste alcun vincolo fra `room_id`, `lease_id` e `property_id`** — non è
-- stata rimossa, e il privilegio resta la sola difesa che copre quella porta.
--
-- CHE COSA IL PRIVILEGIO SMETTE DI GARANTIRE
-- Va detto per intero, perché è il pezzo che si perde. Da qui in avanti un
-- inquilino che sia `reported_by` può riscrivere titolo e descrizione **da
-- qualunque stato** con una `UPDATE` scritta a mano sotto RLS: il limite «finché
-- è `open`» è applicativo, e una policy non può esprimerlo perché **non può
-- confrontare la riga nuova con quella vecchia** — la constatazione di ADR-021 e
-- ADR-026, qui a proprio sfavore.
--
-- Resta vero ciò che la `0020` aveva **misurato** e non dedotto: il confine di
-- isolamento tiene — dopo la scrittura quell'inquilino non vede una riga in più —
-- e ogni `UPDATE` finisce in `audit_log`, perché `tickets` è fra le tabelle con
-- il trigger di audit. È esattamente il compromesso che già vale per `status`,
-- `assigned_to` e i due costi, che sono concesse e la cui disciplina è
-- applicativa, transizione per transizione.
--
-- E non si prova a stringere il confine nella `USING` di `tickets_update`:
-- restringerla a `status = 'open'` per chi ha segnalato romperebbe la
-- **riapertura**, che parte proprio da `closed`.
--
-- CHE COSA NON DIVENTA CORREGGIBILE
-- Categoria e priorità. Sono il lavoro del triage, e lasciarle modificabili
-- altrove renderebbe `triaged` una dichiarazione non verificabile — è la ragione
-- per cui esistono come transizione e non come campi di un `PATCH`.
--
-- Vedi ADR-048.
-- =============================================================================

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_app') THEN
    RAISE NOTICE 'ruolo propflow_app assente: privilegi non applicati (ambiente di sviluppo)';
    RETURN;
  END IF;

  -- Si aggiungono alle undici colonne già concesse dalla `0020`, dalla `0021` e
  -- dalla `0035`. `PATCH /tickets/{id}/report` scrive queste due più
  -- `visible_to_residents`, che la `0035` ha già concesso.
  GRANT UPDATE (title, description) ON app.tickets TO propflow_app;
END $$;
