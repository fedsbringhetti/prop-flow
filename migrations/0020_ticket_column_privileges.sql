-- =============================================================================
-- Privilegi a livello di colonna su app.tickets.
--
-- DIFETTO
-- La `WITH CHECK` di `tickets_update` vincola **solo** `org_id`. La `USING` lascia
-- passare lo staff, chi ha segnalato e chi è assegnato; la `WITH CHECK` non nomina
-- nessun'altra colonna, quindi ogni colonna diversa da `org_id` è libera per
-- chiunque la `USING` ammetta — e ammette l'inquilino che ha aperto la
-- segnalazione.
--
-- Riprodotto su database reale, con UPDATE scritte a mano come inquilino sotto
-- RLS, sulla segnalazione aperta da lui:
--
--   actual_cost = 9999.00 ............................... riuscito
--   assigned_to = <un altro utente> ..................... riuscito
--   status = 'closed', saltando il flusso ............... riuscito
--   priority = 'urgent' ................................. riuscito
--   spostarlo su un immobile in cui non abita ........... riuscito
--   toccare la segnalazione di un altro ................. rifiutato dalla USING
--   spostarlo in un'altra organizzazione ................ rifiutato dal trigger
--                                          assert_org_matches_property
--
-- Due cose che l'esperimento ha stabilito e che il ragionamento da solo non dava.
-- Il **confine di isolamento tiene**: dopo tutte le scritture l'inquilino vede
-- ancora una sola segnalazione, non ha guadagnato visibilità su niente. E il
-- **danno è registrato**: lo storico annota `open → closed` con lui come autore, e
-- `tickets` è fra le tabelle con il trigger di audit, quindi ogni UPDATE finisce
-- in `audit_log`.
--
-- Non è quindi una violazione dell'isolamento come quella che ha prodotto la 0019
-- (ADR-021): è **corruzione di dati dentro la propria organizzazione**, tracciata.
--
-- PERCHÉ LO STRUMENTO SERVE LO STESSO
-- Il criterio della 0019 — «policy che concedono scrittura a un non membro
-- attraversando il confine» — descrive perché *quelle due* ne avevano bisogno, non
-- esaurisce i casi in cui i privilegi di colonna convengono. Qui convengono per una
-- ragione diversa e più modesta: non proteggere un confine, ma togliere di mezzo
-- una classe di incoerenze che **nessuna policy può esprimere**, perché una policy
-- non può confrontare la riga nuova con quella vecchia. È la stessa constatazione
-- che sta nella 0019 e vale qui identica.
--
-- L'incoerenza in questione è strutturale e lo schema non la impedisce: **non
-- esiste alcun vincolo fra `room_id`, `lease_id` e `property_id`**. Il trigger
-- `assert_org_matches_property` verifica soltanto che l'`org_id` del ticket
-- coincida con quello dell'immobile, quindi una stanza di un altro immobile o il
-- contratto di un altro immobile sono accettati senza protestare. Riproducendo la
-- prova qui sopra si è ottenuta esattamente questa riga: una segnalazione spostata
-- su un immobile, con il `lease_id` che continuava a puntare al contratto del
-- primo.
--
-- CORREZIONE
-- Si toglie al ruolo applicativo il privilegio di aggiornare l'intera tabella e lo
-- si riconcede sulle sole colonne che i repository scrivono. Il permesso è
-- verificato **prima** delle policy, quindi la manovra fallisce con
-- `permission denied` qualunque cosa la policy ammetta.
--
-- Restano fuori dalla GRANT `property_id`, `room_id`, `lease_id`, `reported_by`,
-- `reported_by_kind`, `title`, `description`, `reference` e `currency`: dopo
-- l'inserimento non sono più scrivibili dall'applicazione. È accettabile perché una
-- segnalazione aperta sull'immobile sbagliato **si annulla e si riapre** —
-- `cancelled` esiste per questo — e non si corregge spostandola. Ne discende che
-- l'incoerenza strutturale può nascere solo all'apertura, e il controllo
-- applicativo (`OpenTicket`) sta lì, una volta sola, e non su ogni transizione.
--
-- Resta fuori anche `deleted_at`: una segnalazione non si archivia, si annulla, e
-- l'annullamento è uno stato. Il giorno in cui servisse un soft delete, è una
-- migration nuova — ed è il punto in cui ci si ferma a chiedersi se quella colonna
-- debba davvero essere scrivibile.
--
-- `updated_at` non compare perché nessuna query la nomina: la scrive il trigger
-- `set_updated_at`, che è `BEFORE UPDATE` e non è soggetto ai privilegi di colonna
-- del chiamante.
--
-- COSA I PRIVILEGI **NON** FANNO
-- Non dicono nulla su chi possa chiudere una segnalazione o valorizzarne il costo:
-- `status`, `assigned_to`, `estimated_cost` e `actual_cost` restano concesse —
-- devono esserlo, è il lavoro dell'applicazione. Quella parte resta autorizzazione
-- applicativa, transizione per transizione, in
-- `crates/application/src/use_cases/ticket/transition.rs`.
--
-- MANUTENZIONE
-- Ogni colonna aggiunta in futuro a una `UPDATE` va aggiunta anche qui, altrimenti
-- la scrittura fallisce con `permission denied` — ma fallisce **nei test di
-- integrazione**, che girano come `propflow_app` con le stesse concessioni. Il
-- precedente esiste ed è verde:
-- `the_application_role_cannot_repoint_a_lease_party_to_another_lease`.
--
-- `propflow_worker` non viene toccato: ha `BYPASSRLS` ed è il ruolo di fiducia per
-- definizione. Nessuno dei flussi che lo usano scrive su questa tabella.
-- =============================================================================

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_app') THEN
    RAISE NOTICE 'ruolo propflow_app assente: privilegi non applicati (ambiente di sviluppo)';
    RETURN;
  END IF;

  -- Segnalazione: si possono far avanzare gli stati, assegnarla e registrarne i
  -- costi. Resta fuori tutto ciò che dice *di cosa* la segnalazione parla e *chi*
  -- l'ha aperta.
  REVOKE UPDATE ON app.tickets FROM propflow_app;
  GRANT  UPDATE (status, category, priority,
                 assigned_to, assigned_at,
                 estimated_cost, actual_cost,
                 resolved_at, closed_at)
      ON app.tickets TO propflow_app;
END $$;
