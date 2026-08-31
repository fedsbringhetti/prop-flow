-- =============================================================================
-- Privilegi a livello di colonna su app.charges.
--
-- DIFETTO
-- La `WITH CHECK` di `charges_update` è identica alla `USING` — entrambe
-- `org_id = app.current_org_id() AND app.is_org_staff(org_id)` — e una `WITH
-- CHECK` che nomina il solo `org_id` **lascia libera ogni altra colonna**. È la
-- stessa constatazione delle migration 0019 e 0020, e vale qui identica: una
-- policy non può confrontare la riga nuova con quella vecchia.
--
-- Concretamente, oggi uno staff può riscrivere l'`amount` di un addebito già
-- emesso. Non è una violazione dell'isolamento — resta dentro la propria
-- organizzazione, ed è registrata in `audit_log`, perché `app.charges` è fra le
-- tabelle con il trigger di audit della 0013 — ma è **la corruzione che conta di
-- più fra quelle possibili**, perché un addebito non è un dato interno: è una
-- pretesa verso una persona.
--
-- PERCHÉ QUI IL CRITERIO DI ADR-026 DÀ «SÌ»
-- Il secondo criterio è «una colonna non ha un motivo legittimo di cambiare e lo
-- schema non ha un vincolo che ne difenda la coerenza». Su un addebito emesso lo
-- soddisfa **quasi ogni colonna**, e il risultato è la GRANT più corta del
-- progetto: una sola.
--
-- Riscrivere l'importo dopo averlo comunicato non è una correzione, è cambiare le
-- carte in tavola: la traccia resta in `audit_log`, ma la riga che l'inquilino
-- rilegge direbbe un'altra cifra senza che nulla, **nella riga stessa**, ricordi
-- la prima. Un addebito sbagliato si annulla e si riemette — `void` esiste per
-- questo, e l'addebito annullato resta visibile a chi se l'era visto chiedere.
--
-- Restano quindi fuori dalla GRANT `lease_id`, `charge_type`, `amount`,
-- `currency`, `due_date`, `period_start`, `period_end`, `description`,
-- `utility_bill_id`, `generation_key`, `issued_at`, `document_id`, `notes` e
-- `deleted_at`: descrivono la pretesa **nel momento in cui è stata fatta**, e
-- cambiarle significa farne un'altra.
--
-- `deleted_at` in particolare: un addebito non si cancella. L'asimmetria con la
-- bolletta — che annullata sparisce, perché `utility_bills_dedup_uk` è parziale su
-- `deleted_at IS NULL` e altrimenti la stessa fattura non sarebbe reinseribile —
-- è deliberata, e qui diventa strutturale invece di restare una convenzione.
--
-- `updated_at` non compare perché nessuna query la nomina: la scrive il trigger
-- `set_updated_at`, che è `BEFORE UPDATE` e non è soggetto ai privilegi di colonna
-- del chiamante.
--
-- PERCHÉ SULLE BOLLETTE LO STRUMENTO **NON** SI USA
-- Sarebbe l'automatismo naturale — la tabella accanto, lo stesso difetto nella
-- `WITH CHECK` — e sarebbe sbagliato. Una bolletta si **digita**, dodici campi alla
-- volta, e finché non è validata correggerne uno è il lavoro previsto, non
-- un'anomalia. La GRANT dovrebbe nominare quasi ogni colonna, e **un elenco che
-- contiene quasi tutto non difende niente**: si pagherebbe l'onere di manutenzione
-- — ogni colonna aggiunta a una UPDATE va aggiunta anche là — senza comprare la
-- proprietà per cui l'onere esisteva.
--
-- L'immutabilità della bolletta è quindi un'invariante di **dominio**
-- (`UtilityBill::apply` rifiuta da `allocated` in poi) e non un privilegio. È la
-- prima volta che ADR-026 viene applicato e dà «no»: il criterio serve a decidere,
-- non a estendersi da sé.
--
-- COSA I PRIVILEGI **NON** FANNO
-- Non dicono nulla su chi possa annullare un addebito o rinunciare all'incasso:
-- `status` resta concessa — deve esserlo, è il lavoro dell'applicazione. Quella
-- parte resta autorizzazione applicativa, in
-- `crates/application/src/use_cases/billing/charge.rs`.
--
-- IL MODULO DEI PAGAMENTI NON AVRÀ BISOGNO DI ALTRO
-- Riconciliare un incasso porta l'addebito a `partially_paid` o `paid`, cioè
-- scrive `status`, che è concessa. Nessuna migration in più quando quel modulo
-- arriverà, ed è la ragione per cui questa GRANT è già scritta al plurale futuro
-- pur nominando una colonna sola.
--
-- CONSEGUENZA DA DIRE AD ALTA VOCE
-- **Non esiste un PATCH sugli addebiti.** Prorogare una scadenza — «ti do una
-- settimana in più» — oggi si fa annullando e riemettendo. È scomodo ed è voluto:
-- è il punto in cui ci si ferma a chiedersi se `due_date` debba davvero essere
-- scrivibile, e il giorno in cui la risposta sarà sì è una migration nuova, una
-- riga.
--
-- MANUTENZIONE
-- Ogni colonna aggiunta in futuro a una UPDATE va aggiunta anche qui, altrimenti
-- la scrittura fallisce con `permission denied` — ma fallisce **nei test di
-- integrazione**, che girano come `propflow_app` con le stesse concessioni.
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

  -- Addebito: si può solo farne avanzare lo stato. Tutto ciò che descrive la
  -- pretesa — a chi, quanto, per cosa, entro quando — è fissato all'emissione.
  REVOKE UPDATE ON app.charges FROM propflow_app;
  GRANT  UPDATE (status) ON app.charges TO propflow_app;
END $$;
