-- =============================================================================
-- app.v_charge_balances: il residuo si azzera per gli stati terminali.
--
-- DIFETTO
-- La vista ha il solo `WHERE c.deleted_at IS NULL` e non guarda mai `status`.
-- `outstanding_amount` è quindi `amount - paid_amount` anche per un addebito
-- **annullato** o **stornato**, e `is_past_due` è vero anche per un annullato con
-- una scadenza nel passato. `app.v_owner_dashboard` somma da qui senza filtrare a
-- sua volta, quindi eredita entrambi gli errori.
--
-- Riprodotto su database vero, tre addebiti sullo stesso contratto:
--
--   issued           800.00  →  outstanding 800.00
--   void             120.00  →  outstanding 120.00   ← la pretesa è caduta
--   issued, scaduto   40.00  →  outstanding  40.00
--
-- `total_outstanding` ha risposto **960.00**, dove il numero vero è 840.00.
-- All'inquilino era stato detto che quei 120 euro non li doveva più, e il numero
-- di testa della dashboard continuava a contarli come credito. `written_off` fa
-- lo stesso ed è peggio: è una perdita, non un credito.
--
-- La vista è così dalla `0015`; è la fetta contabile ad averla resa raggiungibile,
-- perché prima non esisteva alcun addebito. Vedi ADR-028 e ADR-032.
--
-- CORREZIONE
-- Due soli stati azzerano: `void` e `written_off`. `outstanding_amount` vale `0` e
-- `is_past_due` è falso; `paid_amount` **resta quello vero**, perché è un fatto e
-- non una pretesa.
--
-- PERCHÉ `paid` NON È NELL'ELENCO, CHE È LA PARTE CHE CONTA
-- Azzerare per stato dove l'aritmetica è già significativa **inverte il rapporto**
-- fra i due: lo stato memorizzato scavalcherebbe il calcolo fatto sui pagamenti. Se
-- un giorno `status = 'paid'` e la somma dei pagamenti non torna, quello scarto va
-- **visto**, non nascosto — in contabilità la deriva fra ciò che una riga dichiara e
-- ciò che i movimenti dicono è precisamente il difetto che si vuole scoprire. Lo
-- stesso vale per `draft`, che nessuno scrive.
--
-- `void` e `written_off` sono diversi in natura: lì non è dovuto niente **a
-- prescindere dai pagamenti**, perché la pretesa è caduta. È l'unico caso in cui lo
-- stato è l'autorità, e per questo l'elenco si scrive per esteso invece di derivarlo
-- da `is_outstanding`.
--
-- ALTERNATIVA SCARTATA: TOGLIERE LE RIGHE DALLA VISTA
-- È la via che sembra ovvia ed è sbagliata. `find_balance_by_id` serve il dettaglio
-- di un addebito annullato, che l'inquilino **deve** poter vedere — è l'asimmetria
-- deliberata con la bolletta, che annullata sparisce — e senza la riga riceverebbe
-- `404` su qualcosa che gli spetta. Un test della fetta contabile lo fissa già.
--
-- CONSEGUENZE
-- **Una vista sola rimette a posto due numeri.** `v_owner_dashboard` filtra già
-- `outstanding_amount > 0` e conta su `is_past_due`: non va toccata, e non lo è.
--
-- **`outstanding_amount` di un addebito annullato passa da `"120.00"` a `"0.00"`
-- nell'API.** È un cambiamento di significato, non di forma, ed è additivo solo
-- perché nessun client legge ancora quel campo: è la finestra che i debiti dicevano
-- di non lasciar chiudere.
--
-- `security_invoker = true` è **ripetuto per esteso**: senza il flag la vista
-- girerebbe con i diritti del proprietario e ogni organizzazione vedrebbe i saldi di
-- tutte. `CREATE OR REPLACE VIEW` conserva le `reloptions` esistenti quando la
-- clausola manca, ma affidarsi a quella conservazione significa che il giorno in cui
-- qualcuno ricrea la vista da zero la RLS si apre in silenzio. Il controllo 3 delle
-- invarianti dello schema lo verifica per ogni vista di `app`, ed è la rete sotto
-- questa riga, non il suo sostituto.
--
-- `CREATE OR REPLACE` e non `DROP`+`CREATE`: `v_owner_dashboard` dipende da questa
-- vista, e un `DROP` la porterebbe via con sé. Nomi, ordine e tipi delle colonne
-- restano identici, che è la condizione perché il `REPLACE` sia ammesso.
-- =============================================================================

CREATE OR REPLACE VIEW app.v_charge_balances WITH (security_invoker = true) AS
SELECT c.id                AS charge_id,
       c.org_id,
       c.lease_id,
       c.charge_type,
       c.status,
       c.due_date,
       c.currency,
       c.amount,
       COALESCE(p.paid_amount, 0)::app.money_amount              AS paid_amount,
       -- La pretesa è caduta: non è dovuto niente, a prescindere dai pagamenti.
       CASE
         WHEN c.status = ANY (ARRAY['void','written_off']::app.charge_status[])
           THEN 0
         ELSE c.amount - COALESCE(p.paid_amount, 0)
       END::app.money_amount                                     AS outstanding_amount,
       (c.status <> ALL (ARRAY['void','written_off']::app.charge_status[])
        AND c.due_date < CURRENT_DATE
        AND c.amount > COALESCE(p.paid_amount, 0))               AS is_past_due
  FROM app.charges c
  LEFT JOIN LATERAL (
        -- I due stati che contano nel saldo. **Non** sono i tre di
        -- `payments_payable_ix`: quell'indice comprende anche `authorized`, cioè
        -- denaro autorizzato e non incassato, e contarlo farebbe scendere il saldo
        -- su un incasso che non è arrivato. Il predicato più stretto resta
        -- comunque servito dall'indice.
        SELECT SUM(pm.amount - pm.refunded_amount) AS paid_amount
          FROM app.payments pm
         WHERE pm.payable_type = 'charge'
           AND pm.payable_id = c.id
           AND pm.status = ANY (ARRAY['succeeded','partially_refunded']::app.payment_status[])
       ) p ON true
 WHERE c.deleted_at IS NULL;
