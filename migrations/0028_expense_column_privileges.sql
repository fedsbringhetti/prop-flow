-- =============================================================================
-- Privilegi a livello di colonna su app.expenses.
--
-- DIFETTO, CHE QUI È UN BUCO VERO E NON L'APPLICAZIONE DI UN CRITERIO
-- Le migration 0019, 0020, 0023 e 0025 nascono tutte dalla stessa constatazione —
-- una `WITH CHECK` che nomina il solo `org_id` lascia libera ogni altra colonna,
-- perché una policy non può confrontare la riga nuova con quella vecchia. Vale
-- anche per `expenses_update`, e da sola sarebbe già una ragione sufficiente.
--
-- Ma qui c'è di più, ed è la ragione per cui questa migration esiste: **il trigger
-- che difende l'invariante contabile non copre la porta accanto.**
--
--   expense_shares_balanced   AFTER INSERT OR UPDATE OR DELETE ON app.expense_shares
--
-- Guarda le **quote**. Aggiornare `app.expenses.amount` senza toccare le quote non
-- lo fa scattare, e lascia la somma delle quote diversa dall'importo della spesa —
-- cioè esattamente l'invariante che quel trigger esiste per difendere, aggirata da
-- una `UPDATE` che il trigger non vede passare. `app.tg_expense_shares_balanced`
-- rilegge `app.expenses.amount` ogni volta che è invocato, quindi non è nemmeno un
-- caso limite: è la prima cosa che legge, e nessuno la invoca.
--
-- CORREZIONE
-- Si revoca l'`UPDATE` sulla tabella e lo si riconcede sulle cinque colonne che
-- una correzione legittima tocca. `amount` non è fra esse, quindi la scrittura che
-- sbilancerebbe l'invariante fallisce con `permission denied` invece di passare in
-- silenzio.
--
-- LE CINQUE CONCESSE
--
--   description    il testo di una spesa si corregge: «spesa» → «spesa Coop»
--   category       la categoria è una classificazione, e si sbaglia
--   expense_date   la data in cui si è comprato, che chi registra ricorda dopo
--   notes          testo libero, per definizione
--   deleted_at     l'annullamento
--
-- **`deleted_at` è concessa, e qui la differenza con gli addebiti è deliberata.**
-- Sulla `0023` è fra le negate, perché un addebito annullato deve **restare
-- visibile** a chi se l'era visto chiedere, e per annullarlo c'è
-- `status = 'void'`. `app.expenses` **non ha una colonna di stato** — verificato,
-- non dedotto — quindi il soft delete *è* l'annullamento di una spesa, e negarlo
-- significherebbe non avere alcun modo di disfare una spesa sbagliata.
--
-- LE NEGATE, E LA RAGIONE DI CIASCUNA
--
--   amount            romperebbe l'invariante contabile in silenzio: è il buco
--                     descritto sopra, ed è la ragione di questa migration
--   group_id          sposterebbe la spesa in un altro gruppo, portandosi dietro
--                     quote intestate a persone che in quel gruppo non ci sono
--   paid_by_user_id   cambierebbe **chi ha anticipato**, cioè il verso del credito
--   split_method      descriverebbe una ripartizione diversa da quella che le
--                     quote scritte raccontano
--   currency          vedi ADR-030: la valuta è una sola, e cambiarla su una riga
--                     sola renderebbe la somma dei saldi priva di significato
--   org_id            il confine di isolamento, per la ragione di ADR-021
--   utility_bill_id   è la colonna di `expenses_bill_uk`, unico e parziale:
--                     azzerarlo **libera la bolletta per una seconda
--                     ripartizione**, cioè per un secondo addebito ai coinquilini
--                     della stessa spesa. È lo stesso danno della
--                     `idempotency_key` dei pagamenti (ADR-033), su una colonna
--                     che oggi nessuno scrive
--   document_id       gli allegati alle spese sono fuori perimetro: il giorno in
--                     cui entreranno, è questa GRANT a doversi allargare, e un
--                     `permission denied` è il promemoria giusto
--
-- `created_by`, `metadata` e `created_at` restano fuori perché nessuna query li
-- nomina. `updated_at` non compare fra le concesse perché la scrive il trigger
-- `set_updated_at`, che è `BEFORE UPDATE` e non è soggetto ai privilegi di colonna
-- del chiamante.
--
-- **Una spesa sbagliata si annulla e si riscrive**, come un addebito. La
-- differenza con la bolletta — dove ADR-026 ha dato «no» perché la GRANT avrebbe
-- nominato quasi ogni colonna, e un elenco che contiene quasi tutto non difende
-- niente — è che qui l'elenco delle negate è corto e ognuna ha una ragione propria.
--
-- COSA I PRIVILEGI **NON** FANNO
-- Non dicono nulla su **chi** possa correggere una spesa: quella parte è Row Level
-- Security (`expenses_update` chiede `can_access_expense_group`) e autorizzazione
-- applicativa. E non difendono le quote: `app.expense_shares` resta aggiornabile
-- per intero, ed è corretto — là il trigger scatta, e l'invariante è difesa da chi
-- di dovere.
--
-- MANUTENZIONE
-- Ogni colonna aggiunta in futuro a una `UPDATE` va aggiunta anche qui, altrimenti
-- la scrittura fallisce con `permission denied` — ma fallisce **nei test di
-- integrazione**, che girano come `propflow_app` con le stesse concessioni. Il
-- controllo 12 delle invarianti dello schema verifica l'elenco al contrario.
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

  -- Spesa: si correggono la descrizione, la classificazione, la data e le note, e
  -- si annulla. Quanto, di chi, in quale gruppo e con quale ripartizione sono
  -- fissati alla registrazione.
  REVOKE UPDATE ON app.expenses FROM propflow_app;
  GRANT  UPDATE (description, category, expense_date, notes, deleted_at)
      ON app.expenses TO propflow_app;
END $$;
