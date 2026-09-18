-- =============================================================================
-- Una bolletta pagata si può dire pagata.
--
-- IL FATTO
-- `app.bill_status` contiene `'paid'` da sempre e **nessuno lo scrive**: le
-- transizioni esposte sono validazione, riaddebito e annullamento. Chi amministra
-- paga il fornitore e non ha modo di registrarlo, quindi l'elenco delle bollette
-- non distingue quelle saldate da quelle da saldare — che è la sola domanda che
-- si fa guardandolo.
--
-- PERCHÉ UNA COLONNA E NON UNA RIGA IN `app.payments`
-- Il modulo dei pagamenti è costruito per il denaro che **entra**: `payable_type`
-- è un enum con tre valori — `charge`, `expense_share`, `settlement` — e un
-- pagamento ha `payer_user_id` e `payee_user_id`. Un fornitore non è un utente, e
-- una riga con il beneficiario a `NULL` sarebbe un pagamento che non dice a chi
-- sono andati i soldi. Il denaro in uscita è un modulo che non esiste ed è
-- registrato da tre fette fra ciò che resta fuori: questa colonna dichiara un
-- fatto, non lo modella.
--
-- LA DATA È OBBLIGATORIA QUANDO LO STATO C'È
-- Stessa forma di `utility_bills_validated_ck` qui accanto e di
-- `leases_terminated_ck`: uno stato che dichiara un fatto senza la data del fatto
-- non dice quando è successo, e «quando» è metà dell'informazione — le bollette si
-- pagano a scadenza, e sapere che una è stata pagata in ritardo è il motivo per
-- cui la si guarda.
--
-- CHI L'HA DICHIARATA NON DIVENTA UNA COLONNA
-- `validated_by` esiste perché la validazione è il momento in cui una persona si
-- prende la responsabilità dei numeri che una macchina ha letto: è un fatto sul
-- dato. Dichiarare pagata una bolletta è un fatto sul denaro, e chi l'ha fatto sta
-- in `audit_log` come ogni altra scrittura.
-- =============================================================================

ALTER TABLE app.utility_bills
    ADD COLUMN paid_at TIMESTAMPTZ;

ALTER TABLE app.utility_bills
    ADD CONSTRAINT utility_bills_paid_ck CHECK (status <> 'paid' OR paid_at IS NOT NULL);

COMMENT ON COLUMN app.utility_bills.paid_at IS
  'Quando la bolletta è stata pagata al fornitore, dichiarato da chi amministra.
   Non è un pagamento del modulo `app.payments`, che modella il denaro in entrata:
   qui si registra un fatto avvenuto fuori dal prodotto.';
