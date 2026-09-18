-- =============================================================================
-- Il proprietario nasce da un invito di piattaforma: `app.owner_invitations`.
--
-- PERCHÉ ESISTE
-- ADR-053: chi diventa proprietario non si registra da una rotta aperta a
-- chiunque — `RegisterOwner` smette di esserlo — ma da un invito emesso da uno
-- script in `ops/` dietro un token. Come si verifichi che una persona sia un
-- proprietario vero è una decisione fuori dal prodotto; nel prodotto vive solo il
-- fatto che il token esiste.
--
-- NESSUN `org_id`: LA TABELLA È DI PIATTAFORMA
-- L'invito **precede** l'organizzazione — la crea l'accettazione, come
-- `RegisterOwner` fa oggi per chiunque. Non è `org_invitations` con un `org_id`
-- annullabile: quella tabella ha `org_invitations_staff FOR ALL
-- USING (is_org_staff(org_id))`, e un `org_id` nullo la renderebbe falsa o da
-- riscrivere per un ramo che oggi non ha; il suo indice parziale
-- `org_invitations_pending_uk` è su `(org_id, email)`, che per un invito senza
-- organizzazione non avrebbe senso; e ogni query su quella tabella erediterebbe
-- un ramo «senza organizzazione» che oggi non possiede. Una tabella nuova evita
-- tutti e tre.
--
-- LO HASH E NON IL TOKEN, COME OGNI CREDENZIALE OPACA DEL PROGETTO
-- Stessa forma di `org_invitations.token_hash` e di
-- `password_reset_codes.code_hash` (migration `0037`): il valore in chiaro esiste
-- solo nel messaggio che chi verifica il proprietario gli consegna fuori banda, e
-- chi legge il database non trova una credenziale utilizzabile.
--
-- ROW LEVEL SECURITY: UNA POLICY CHE NEGA TUTTO
-- Il ruolo applicativo non deve mai vedere questa tabella attraverso una sessione
-- HTTP autenticata: non esiste una rotta che la legga da lì, e concedere una
-- policy «la riga è tua» come `password_reset_codes_own` non avrebbe un soggetto —
-- un invito da proprietario non appartiene a nessuno finché non è accettato, e chi
-- lo accetta non ha ancora un `current_user_id()` da confrontare (è una
-- registrazione, non un'azione di un utente esistente). `USING (false)` è quindi
-- la policy più stretta che soddisfi il controllo 2 delle invarianti (RLS attiva
-- senza policy nega tutto, ed è un errore da correggere e non una scelta): nega
-- ogni riga, per ogni comando, a chi è soggetto alla RLS. Chi scrive la riga (lo
-- script in `ops/`, con il ruolo del migratore) e chi la accetta (il pool
-- privilegiato) non sono soggetti alla RLS e non passano da qui — il primo perché
-- è proprietario della tabella, il secondo perché ha `BYPASSRLS`.
--
-- NESSUN `deleted_at`, NESSUN `updated_at`
-- Come `password_reset_codes`: non è un'entità di business, è una credenziale a
-- tempo. Una riga accettata, scaduta o revocata resta — il ruolo applicativo non
-- ha comunque il privilegio `DELETE` (ADR-003) — e la potatura è un'operazione del
-- runbook con il ruolo del migratore, non un trigger che nessuno guarda.
--
-- `accepted_by` HA IL SUO INDICE PER L'INVARIANTE 4
-- Una foreign key senza indice sulla colonna referenziante trasformerebbe ogni
-- verifica di `ON DELETE SET NULL` in una scansione sequenziale.
--
-- I PRIVILEGI DI COLONNA, E PERCHÉ TOCCANO ANCHE IL POOL PRIVILEGIATO
-- `GRANT SELECT, UPDATE (accepted_at, accepted_by)`: sono le due colonne che
-- l'accettazione scrive, ed è la coppia che entra nel controllo 12. La concessione
-- serve a **entrambi** i ruoli non proprietari:
--
--   - `propflow_app`, perché il controllo 12 verifica *quali* colonne restano
--     scrivibili confrontando i privilegi di quel ruolo, e perché è la stessa
--     forma di `password_reset_codes_own` — innocua sotto `USING (false)`, che
--     nega comunque ogni riga, ma dichiarata e non lasciata al default;
--   - `propflow_worker`, perché è il pool **privilegiato** che l'accettazione usa
--     davvero (`begin_privileged`, ADR-053 punto 3, regola 27): `BYPASSRLS` ignora
--     le policy, non i privilegi di tabella, e una tabella nuova non eredita nulla
--     dal `GRANT ... ON ALL TABLES` della `0015` (stessa nota della `0037`). Senza
--     questa riga l'accettazione, che legge per cercare l'invito e scrive per
--     marcarlo accettato, fallirebbe con `permission denied` nonostante
--     `BYPASSRLS` — la stessa distinzione per cui la `0037` concede
--     esplicitamente anche a `propflow_worker`.
--
-- Nessun `INSERT` a `propflow_app`: la riga la scrive solo lo script in `ops/`,
-- con il ruolo del migratore, che è proprietario della tabella e non ha bisogno di
-- una `GRANT`. `propflow_worker` riceve anche `INSERT`: non perché il flusso di
-- produzione la usi — quello passa dallo script — ma perché è l'unica connessione
-- disponibile ai test di integrazione per costruire una riga di prova (i test non
-- hanno una connessione con il ruolo del migratore: `ops/ci/test-integration.sh`
-- passa a `cargo test` solo `PROPFLOW_TEST_APP_URL` e
-- `PROPFLOW_TEST_PRIVILEGED_URL`). Concederla è comunque conforme al confine che
-- conta: il ruolo applicativo resta senza, ed è quello per cui la tabella esiste.
-- Nessun `DELETE`: la potatura è del runbook, come per `password_reset_codes`.
--
-- LA TRAPPOLA: `ALTER DEFAULT PRIVILEGES` DELLA 0015 CONCEDE GIÀ TUTTO A
-- `propflow_app`, E VA DISFATTA QUI
-- La 0015 non ha concesso a `propflow_app` solo le tabelle che esistevano allora:
-- ha anche impostato `ALTER DEFAULT PRIVILEGES IN SCHEMA app GRANT SELECT,
-- INSERT, UPDATE ON TABLES TO propflow_app`, che si applica a **ogni tabella
-- nuova creata da chi ha eseguito quel comando** — il migratore, cioè ogni
-- migration successiva, questa compresa. Verificato eseguendo
-- `ops/sql/checks/schema_invariants.sql` subito dopo aver scritto solo la `GRANT
-- UPDATE (accepted_at, accepted_by)` senza le due `REVOKE`: il controllo 12
-- segnalava le cinque colonne negate come ancora scrivibili, perché
-- `propflow_app` aveva già `UPDATE` sull'intera tabella dal default. È la
-- differenza con `propflow_worker`, che nella stessa migration `0015` riceve solo
-- un `GRANT ... ON ALL TABLES` una tantum — non un default — e per questo la nota
-- della `0037` («i privilegi non arrivano dal GRANT della 0015») è vera per
-- `propflow_worker` e **falsa** per `propflow_app`: quella migration lo scrive
-- comunque per esplicito, ma qui non basterebbe, perché senza le `REVOKE`
-- resterebbe anche l'`INSERT` che questa tabella non deve concedere.
-- =============================================================================

CREATE TABLE app.owner_invitations (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email        app.email_address NOT NULL,
    -- SHA-256 del token: mai il token in chiaro.
    token_hash   BYTEA NOT NULL,
    expires_at   TIMESTAMPTZ NOT NULL,
    accepted_at  TIMESTAMPTZ,
    accepted_by  UUID REFERENCES app.users (id) ON DELETE SET NULL,
    revoked_at   TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT owner_invitations_expiry_ck CHECK (expires_at > created_at)
);

CREATE UNIQUE INDEX owner_invitations_token_uk ON app.owner_invitations (token_hash);
-- Un solo invito pendente per indirizzo, come org_invitations_pending_uk.
CREATE UNIQUE INDEX owner_invitations_pending_uk ON app.owner_invitations (email)
    WHERE accepted_at IS NULL AND revoked_at IS NULL;
-- Copre la foreign key verso `users`, per l'invariante `fk-senza-indice`.
CREATE INDEX owner_invitations_accepted_by_ix ON app.owner_invitations (accepted_by)
    WHERE accepted_by IS NOT NULL;

ALTER TABLE app.owner_invitations ENABLE ROW LEVEL SECURITY;

-- La policy più stretta che soddisfi il controllo 2: nega tutto, a tutti, sotto la
-- RLS. Chi scrive (lo script, come proprietario della tabella) e chi accetta (il
-- pool privilegiato, BYPASSRLS) non sono soggetti alla RLS e non passano da qui.
CREATE POLICY owner_invitations_none ON app.owner_invitations FOR ALL USING (false);

COMMENT ON TABLE app.owner_invitations IS
  'Inviti da proprietario, di piattaforma e senza org_id: la crea l''accettazione.
   Emessi da uno script in ops/ con il ruolo del migratore, dietro una verifica che
   sta fuori dal prodotto (ADR-053). Hash e non token in chiaro, come ogni
   credenziale opaca del progetto. RLS attiva con una sola policy che nega tutto al
   ruolo soggetto alla RLS: la scrive il migratore, la legge e la aggiorna il pool
   privilegiato.';

DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_app') THEN
    -- `ALTER DEFAULT PRIVILEGES` della 0015 concede SELECT, INSERT, UPDATE a
    -- livello di **tabella intera** a ogni tabella nuova creata dal migratore per
    -- propflow_app — a differenza di propflow_worker, che in quella migration ha
    -- solo un `GRANT ... ON ALL TABLES` una tantum, applicato a ciò che esisteva
    -- allora. Senza le due REVOKE qui sotto, propflow_app manterrebbe INSERT e
    -- UPDATE su ogni colonna: `USING (false)` negherebbe comunque le righe, ma il
    -- controllo 12 delle invarianti troverebbe le colonne negate ancora
    -- scrivibili, ed è un privilegio che non deve esistere anche se innocuo.
    -- `REVOKE ALL` e non solo INSERT, UPDATE: così ciò che resta è scritto qui
    -- sotto per esteso, e non «il default meno due cose». Il SELECT è concesso di
    -- nuovo perché `USING (false)` lo rende innocuo e perché la prosa che lo
    -- descrive deve corrispondere a una riga che esiste.
    EXECUTE 'REVOKE ALL ON app.owner_invitations FROM propflow_app';
    EXECUTE 'GRANT SELECT, UPDATE (accepted_at, accepted_by) ON app.owner_invitations TO propflow_app';
  END IF;
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_worker') THEN
    EXECUTE 'GRANT SELECT, INSERT, UPDATE (accepted_at, accepted_by) ON app.owner_invitations TO propflow_worker';
  END IF;
END $$;
