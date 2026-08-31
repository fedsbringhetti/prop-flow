-- =============================================================================
-- Privilegi a livello di colonna su app.payments.
--
-- DIFETTO
-- La `WITH CHECK` di `payments_update` è identica alla `USING` — entrambe
-- `org_id = app.current_org_id() AND app.is_org_staff(org_id)` — e una `WITH
-- CHECK` che nomina il solo `org_id` **lascia libera ogni altra colonna**. È la
-- stessa constatazione delle migration 0019, 0020 e 0023, e vale qui identica: una
-- policy non può confrontare la riga nuova con quella vecchia.
--
-- Concretamente, oggi uno staff che conferma una dichiarazione potrebbe nella
-- stessa `UPDATE` riscriverne l'`amount`, o cambiare il `payer_user_id` — cioè
-- **chi** ha pagato e **quanto**, che è l'intero contenuto informativo di un
-- pagamento.
--
-- PERCHÉ QUI IL CRITERIO DI ADR-026 DÀ «SÌ», E PIÙ NETTAMENTE CHE ALTROVE
-- Il secondo criterio è «una colonna non ha un motivo legittimo di cambiare e lo
-- schema non ha un vincolo che ne difenda la coerenza». Su un pagamento lo
-- soddisfa tutto tranne il proprio avanzamento di stato: un pagamento **registra
-- un fatto già avvenuto**, e un fatto non si corregge riscrivendolo.
--
-- La GRANT nomina quindi tre colonne, che sono esattamente le tre che il dominio
-- muove:
--
--   status         pending → succeeded (conferma) o → cancelled (rifiuto)
--   paid_at        valorizzata alla conferma, **azzerata** al rifiuto
--   reconciled_at  valorizzata quando una persona riconosce il movimento
--
-- `paid_at` è nella GRANT proprio per l'azzeramento: `payments_succeeded_ck` è
-- `status <> 'succeeded' OR paid_at IS NOT NULL` e vincola **una sola direzione**
-- — obbliga ad avere la data quando si è riusciti, non a toglierla muovendosi
-- altrove. È la terza volta che questo schema presenta questa forma, dopo
-- `tickets_closed_ck` e `utility_bills_validated_ck`, e ormai è un pattern:
-- l'azzeramento tocca al dominio, quindi la colonna deve restare scrivibile.
--
-- COSA RESTA FUORI, E PERCHÉ
-- `amount`, `payer_user_id`, `payee_user_id`, `method`, `currency`, `payable_type`
-- e `payable_id` descrivono il movimento **come è avvenuto**: cambiarli significa
-- descriverne un altro. `payer_user_id` in particolare non è anagrafica — è ciò
-- che rende la riga visibile a chi ha pagato, perché `payments_select` mostra la
-- riga a `payer_user_id = app.current_user_id()`: riscriverlo toglierebbe a una
-- persona la vista sul proprio pagamento.
--
-- `idempotency_key` è la colonna la cui riscrittura costa denaro: è metà di
-- `payments_idempotency_uk`, e cambiarla libera la chiave per un secondo
-- inserimento, cioè per un saldo che scende due volte. Vedi ADR-033.
--
-- `refunded_amount` resta fuori perché i rimborsi sono fuori dal perimetro della
-- fetta (`docs/09-stato-e-roadmap.md`). Il giorno in cui un rimborso si scriverà,
-- è questa GRANT a doversi allargare — e il fatto che la scrittura fallisca con
-- `permission denied` invece di passare in silenzio è il promemoria giusto.
--
-- `provider`, `provider_payment_id` e `provider_payload` restano fuori per la
-- stessa ragione al contrario: li scriverà un webhook, che non passa da qui.
--
-- `created_at`, `failure_code`, `failure_message` e `metadata`: nessuna query li
-- nomina. `updated_at` non compare perché la scrive il trigger `set_updated_at`,
-- che è `BEFORE UPDATE` e non è soggetto ai privilegi di colonna del chiamante.
--
-- COSA I PRIVILEGI **NON** FANNO
-- Non dicono nulla su **chi** possa confermare o rifiutare: `status` resta
-- concessa, e deve esserlo. Quella parte è autorizzazione applicativa, in
-- `crates/application/src/use_cases/payment/`, e Row Level Security:
-- `payments_update` è staff soltanto, ed è ciò che impedisce a un inquilino di
-- ritirare la propria dichiarazione.
--
-- MANUTENZIONE
-- Ogni colonna aggiunta in futuro a una UPDATE va aggiunta anche qui, altrimenti
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

  -- Pagamento: si può solo farne avanzare lo stato e datare il movimento. Chi ha
  -- pagato, quanto e come sono fissati all'inserimento.
  REVOKE UPDATE ON app.payments FROM propflow_app;
  GRANT  UPDATE (status, paid_at, reconciled_at) ON app.payments TO propflow_app;
END $$;
