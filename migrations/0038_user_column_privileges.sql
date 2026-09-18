-- =============================================================================
-- La propria riga si corregge, non si riscrive: privilegi di colonna su
-- `app.users`.
--
-- DIFETTO
-- `users_update_self` (0014) è
--
--   USING      (id = app.current_user_id() AND deleted_at IS NULL)
--   WITH CHECK (id = app.current_user_id())
--
-- cioè una `WITH CHECK` che nomina **la sola identità della riga**. È la
-- constatazione di ADR-021 e ADR-026 su una quinta tabella: ciò che il
-- `WITH CHECK` non nomina resta libero, e sul ruolo applicativo non c'è nessuna
-- `GRANT UPDATE` di colonna su `app.users` — la 0015 concede `UPDATE` sull'intera
-- tabella.
--
-- La colonna più cara è `platform_role`, e la catena va scritta per esteso perché
-- da una riga sola non si vede:
--
--   UPDATE app.users SET platform_role = 'admin' WHERE id = app.current_user_id()
--     → Login legge users.platform_role e lo mette nei claim del token
--     → il middleware installa app.platform_admin = 'on' nel contesto
--     → app.is_platform_admin() è vera
--     → is_org_member e is_org_staff sono vere su OGNI organizzazione
--
-- Seguono `status`, con cui ci si toglie una sospensione amministrativa, e
-- `failed_login_count` con `locked_until`, che **sono** la difesa dal blocco dopo
-- dieci tentativi descritta in docs/04-sicurezza.md: azzerarli a ogni tentativo
-- rende quella soglia irraggiungibile.
--
-- NON È SFRUTTABILE DALL'API, E VA DETTO CON LA STESSA PRECISIONE
-- Nessuna rotta scrive quelle colonne, e chi non possiede le credenziali del ruolo
-- applicativo non può emettere quella query. È una difesa in profondità mancante,
-- della classe che ADR-021 e ADR-026 hanno già chiuso quattro volte — *una regola
-- applicativa non è un confine*.
--
-- PERCHÉ ADESSO
-- Per una ragione di tempi, e non di gravità. La sezione 6 aggiunge la **prima
-- rotta che scrive `app.users` dal pool applicativo** (`PATCH /api/v1/me` e il
-- cambio password): finché non esiste, la porta è chiusa da un privilegio che
-- nessuno usa; da quando esiste, è chiusa solo da un `UPDATE` scritto bene.
--
-- CONCESSE E NEGATE
-- Concesse: `full_name`, `phone`, `locale`, `password_hash` — le quattro che una
-- **richiesta autenticata** scrive. Le prime tre sono i dati anagrafici che si
-- correggono da soli; `password_hash` è il cambio password da dentro, che gira sul
-- pool applicativo perché chi lo chiede è autenticato (regola 27).
--
-- Negate: `platform_role`, `status`, `email`, `email_verified_at`, `last_login_at`,
-- `failed_login_count`, `locked_until`, `deleted_at`, e con esse tutta la PII
-- cifrata (`fiscal_code_encrypted`, `fiscal_code_fingerprint`, `encryption_key_ref`,
-- `mfa_secret_encrypted`), `avatar_document_id`, `terms_accepted_at`,
-- `privacy_accepted_at` e `created_at`.
--
-- **Le negate non sono irraggiungibili**: le scrive il pool **privilegiato**, che
-- ha BYPASSRLS e i privilegi pieni. `Login` aggiorna `last_login_at`, il contatore
-- dei tentativi e il blocco; il recupero password scrive `password_hash`. È la
-- stessa divisione delle quattro migration che l'hanno preceduta: ciò che una
-- richiesta autenticata può scrivere è un elenco corto e dichiarato, tutto il resto
-- passa da un flusso che il contesto di sicurezza non ha.
--
-- `email` è negata **anche se il prodotto un giorno la cambierà**, ed è la ragione
-- della decisione 2 della sezione 6: non è un dato anagrafico, è la credenziale
-- d'accesso, ed è la chiave con cui `app.has_pending_invitation`,
-- `app.current_user_email()` e `lease_parties.email` si ritrovano. Il giorno in cui
-- il cambio esisterà — con la verifica del nuovo indirizzo, che è la sua
-- precondizione — sarà una migration nuova, una riga.
--
-- LA TRAPPOLA, PERCHÉ SI PAGA UNA VOLTA SOLA
-- `UserRepository::update` nomina **tutte** le colonne che il dominio muove,
-- `status`, `last_login_at`, `failed_login_count` e `locked_until` comprese: da
-- questa migration in poi, chiamarlo dal pool applicativo fallisce con
-- `permission denied for table users`. Oggi lo chiamano solo `Login` e il recupero
-- password, entrambi privilegiati, quindi la migration non rompe niente **e non lo
-- dimostra**. È `update_profile` — che nomina le sole concesse — il metodo che le
-- rotte nuove devono usare.
--
-- `updated_at` non compare perché nessuna query la nomina: la scrive il trigger
-- `set_updated_at`, che è `BEFORE UPDATE` e non è soggetto ai privilegi di colonna
-- del chiamante.
--
-- `propflow_worker` non viene toccato: ha `BYPASSRLS` ed è il ruolo di fiducia per
-- definizione.
--
-- MANUTENZIONE
-- L'elenco concesso e le colonne che compaiono in `update_profile` sono **la stessa
-- lista**. Aggiungerne una qui senza aggiungerla là è un privilegio che nessuno usa;
-- aggiungerla là senza aggiungerla qui è un `permission denied` — che però compare
-- **nei test di integrazione**, che girano come `propflow_app`.
-- =============================================================================

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_app') THEN
    RAISE NOTICE 'ruolo propflow_app assente: privilegi non applicati (ambiente di sviluppo)';
    RETURN;
  END IF;

  -- Utente: da una richiesta autenticata si correggono i propri dati anagrafici e
  -- si cambia la propria password. Chi si è, che ruolo di piattaforma si ha e in
  -- che stato è l'account non sono cose che si scrivono di sé stessi.
  REVOKE UPDATE ON app.users FROM propflow_app;
  GRANT  UPDATE (full_name, phone, locale, password_hash) ON app.users TO propflow_app;
END $$;
