-- =============================================================================
-- Recupero della password: un codice a uso singolo, con lo hash e non il codice.
--
-- PERCHÉ ESISTE
-- Le rotte di autenticazione erano quattro — register, login, refresh, logout — e
-- non c'era **nessun** modo di recuperare una password. Il primo «non mi ricordo
-- la password» di un utente vero finiva in una UPDATE scritta a mano sul database
-- di produzione. Vedi ADR-052.
--
-- LO HASH E NON IL CODICE
-- Stessa forma di `org_invitations.token_hash` e di `user_sessions.refresh_token_hash`:
-- il valore in chiaro esiste solo nell'email, e chi legge il database non trova una
-- credenziale utilizzabile. L'indice unico è sullo hash, che è ciò su cui si cerca.
--
-- IL LIMITE AGLI INVII NON HA UNA COLONNA SUA
-- Il piano ne prevedeva una — «quando è stato emesso l'ultimo codice» — e non
-- serve: è il `created_at` della riga più recente di quell'utente, che
-- `password_reset_codes_user_ix` rende una lettura sola. Una colonna che
-- duplicasse quel valore avrebbe due fonti di verità per lo stesso fatto.
--
-- Il limite è per **utente** e non per IP: quello vorrebbe uno stato condiviso e un
-- servizio in più, e non serve con una sola istanza. Un indirizzo che non ha un
-- account non produce nessuna riga e non riceve niente, quindi non c'è niente da
-- amplificare.
--
-- ROW LEVEL SECURITY
-- Il flusso gira sul **pool privilegiato**, che ha BYPASSRLS: chi ha dimenticato la
-- password non è autenticato per definizione, quindi non c'è un contesto da
-- installare e la RLS non ha nulla su cui appoggiarsi (regola 27, ADR-052). La
-- policy esiste lo stesso, ed è quella di `user_sessions`: la riga è **tua**. Serve
-- a due cose — l'invariante `rls-senza-policy` di `ops/sql/checks`, e il non
-- concedere le righe di qualcun altro al ruolo applicativo il giorno in cui
-- qualcosa le leggesse da lì.
--
-- NESSUN `deleted_at`, NESSUN `updated_at`
-- Non è un'entità di business: è una credenziale a tempo. Un codice usato o scaduto
-- è inerte, e le righe restano — il ruolo applicativo non ha il privilegio DELETE
-- (ADR-003). La potatura è un'operazione del runbook, con il ruolo del migratore,
-- e non un trigger che nessuno guarda.
-- =============================================================================

CREATE TABLE app.password_reset_codes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES app.users (id) ON DELETE CASCADE,
    -- SHA-256 del codice: mai il codice in chiaro.
    code_hash   BYTEA NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    used_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT password_reset_codes_used_ck CHECK (used_at IS NULL OR used_at >= created_at)
);

CREATE UNIQUE INDEX password_reset_codes_hash_uk ON app.password_reset_codes (code_hash);
-- Regge due letture: il codice più recente di un utente, per il limite agli invii.
-- Copre anche la foreign key, che l'invariante `fk-senza-indice` esige.
CREATE INDEX password_reset_codes_user_ix
    ON app.password_reset_codes (user_id, created_at DESC);

ALTER TABLE app.password_reset_codes ENABLE ROW LEVEL SECURITY;

-- Come `user_sessions_own`: la riga è tua. Sotto la RLS nessuno la scrive — quel
-- percorso passa dal pool privilegiato — e nessuno legge quelle di un altro.
CREATE POLICY password_reset_codes_own ON app.password_reset_codes FOR ALL
    USING (user_id = app.current_user_id())
    WITH CHECK (user_id = app.current_user_id());

COMMENT ON TABLE app.password_reset_codes IS
  'Codici di recupero della password: hash, scadenza breve, uso singolo. Il valore in
   chiaro esiste solo nell''email. Scritti e letti dal pool privilegiato, perché chi
   ha dimenticato la password non è autenticato (ADR-052).';

-- I privilegi su una tabella nuova non arrivano dal `GRANT ... ON ALL TABLES` della
-- 0015, che si applica solo a ciò che esisteva allora. Nessun DELETE, come ovunque.
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_app') THEN
    EXECUTE 'GRANT SELECT, INSERT, UPDATE ON app.password_reset_codes TO propflow_app';
  END IF;
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_worker') THEN
    EXECUTE 'GRANT SELECT, INSERT, UPDATE, DELETE ON app.password_reset_codes TO propflow_worker';
  END IF;
END $$;
