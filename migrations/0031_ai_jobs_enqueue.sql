-- =============================================================================
-- Accodamento di un job AI: lo staff può inserire, e solo un job `queued`.
--
-- DIFETTO
-- Lo schema, letto come è uscito dalla `0014` e dalla `0015`, rende
-- `app.ai_jobs` **non scrivibile** dal ruolo applicativo — e non si vede dai
-- privilegi:
--
--   0015   GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA app TO propflow_app
--   0014   CREATE POLICY ai_jobs_select ... FOR SELECT      <-- l'unica policy
--
-- Con la Row Level Security attiva e **nessuna policy per un comando**, quel
-- comando è negato: la mancanza di una policy non è permissiva. Il privilegio
-- quindi c'è e la policy no, e chi guarda `has_table_privilege` conclude il
-- contrario. Verificato prima di scrivere questa migration: l'`INSERT` dallo
-- staff con il contesto installato risponde
-- «new row violates row-level security policy for table "ai_jobs"», che il
-- repository traduce in `auth.denied_by_policy` — il codice riservato ai nostri
-- difetti.
--
-- CORREZIONE
-- Una policy di `INSERT` per lo staff dell'organizzazione. L'`UPDATE` resta del
-- solo `propflow_worker`, che ha `BYPASSRLS` e non passa da alcuna policy: chi
-- accoda dichiara il lavoro, chi lo esegue ne scrive l'esito, e le due cose non
-- sono lo stesso permesso.
--
-- PERCHÉ NON IL POOL PRIVILEGIATO
-- È ADR-020 alla lettera, e la stessa domanda della `0018`. `begin_privileged()`
-- è per i flussi in cui un contesto di sicurezza **non può** esistere —
-- registrazione, login, rotazione del refresh token — perché avvengono prima
-- dell'autenticazione. Qui il contesto esiste: chi carica la scansione è
-- autenticato ed è staff. Usare il pool privilegiato per comodità allargherebbe
-- la superficie che la regola 20 di `CLAUDE.md` esiste per tenere piccola.
--
-- PERCHÉ `status = 'queued'` STA NELLA `WITH CHECK` E NON SOLO NEL CODICE
-- È la stessa forma di `role = 'tenant'` in `org_members_accept_invitation` della
-- `0018`. Senza, una richiesta HTTP potrebbe inserire un job che si dichiara già
-- `succeeded` con un `output` scritto a mano — cioè far dire alla macchina quello
-- che vuole chi chiama, che è il contrario del motivo per cui `ai_jobs` esiste.
-- Il vincolo è sulla riga nuova, quindi nemmeno una query scritta a mano lo
-- aggira.
--
-- I DUE CONTATORI NON SONO NELLA POLICY, E VA DETTO
-- `attempts`, `tokens_*`, `cost_amount` e `latency_ms` restano ai loro default e
-- la policy non li nomina: un chiamante malevolo potrebbe dichiarare un costo o
-- un numero di tentativi falsi al momento dell'inserimento. Non è un buco che
-- valga una condizione in più oggi — quei valori li **riscrive il worker** alla
-- chiusura del job, e finché non esiste un fornitore vero il costo è zero
-- (`docs/09-stato-e-roadmap.md`, «un tetto di spesa per organizzazione» è fuori
-- perimetro). Il giorno in cui il tetto di spesa esisterà, la difesa corretta è
-- un privilegio di colonna come nella `0023`, non un ramo in più qui: una policy
-- non può confrontare la riga nuova con quella vecchia, ed è esattamente il
-- criterio di ADR-026.
--
-- NESSUNA POLICY DI `UPDATE` PER L'APPLICAZIONE
-- Deliberato, e non una dimenticanza da colmare alla prima occasione. Un job è
-- il registro di ciò che una macchina ha fatto: se l'API potesse riscriverlo, la
-- misura dell'accuratezza dell'OCR — che è la sola domanda che conta su questo
-- modulo — sarebbe scritta da chi viene misurato. Annullare un job non serve:
-- `cancelled` resta nell'enum e il dominio non lo scrive, vedi la decisione A del
-- piano.
--
-- NESSUN FILTRO SUL SOFT DELETE
-- `app.ai_jobs` **non ha** `deleted_at`: è la terza tabella senza, dopo i
-- pagamenti e quelle del co-living. Filtrarla per abitudine darebbe un errore di
-- colonna inesistente, qui come nelle query.
-- =============================================================================

CREATE POLICY ai_jobs_enqueue ON app.ai_jobs FOR INSERT
    WITH CHECK (org_id = app.current_org_id()
                AND app.is_org_staff(org_id)
                AND status = 'queued');

COMMENT ON POLICY ai_jobs_enqueue ON app.ai_jobs IS
  'Accodamento di un job AI dallo staff dell''organizzazione attiva. `status =
   ''queued''` è nella WITH CHECK e non solo nel codice: una richiesta non deve
   poter inserire un job che si dichiara già concluso. L''aggiornamento resta del
   solo propflow_worker (BYPASSRLS).';
