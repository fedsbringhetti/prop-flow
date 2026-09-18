-- Riga d'inserimento usata da issue-owner-invitation.sh.
--
-- File a parte e non `-c` sulla riga di comando: psql interpola le variabili
-- `:'nome'` solo leggendo da uno script (`-f`), non nel testo passato a `-c`
-- (verificato: la stessa istruzione con `-c` dà "syntax error at or near ':'").
INSERT INTO app.owner_invitations (email, token_hash, expires_at)
VALUES (:'email'::app.email_address, :'token_hash'::bytea, now() + make_interval(days => :days));
