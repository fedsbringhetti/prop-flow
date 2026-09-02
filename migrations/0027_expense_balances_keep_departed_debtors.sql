-- =============================================================================
-- app.v_expense_group_balances: chi esce con un conto aperto resta nei saldi.
--
-- DIFETTO
-- La vista parte da `app.expense_group_members` con `WHERE m.left_at IS NULL`.
-- Chi lascia il gruppo **sparisce dai saldi portandosi via il debito**: il
-- coinquilino che deve 200 euro esce, e quei 200 euro non risultano più a nessuno.
-- La somma dei saldi netti del gruppo, che deve fare zero, smette di farlo — e il
-- credito di chi ha anticipato resta lì a fronte di un debito che la vista non
-- mostra più.
--
-- È lo stesso difetto delle viste dei saldi degli addebiti corretto dalla `0024`:
-- un numero che mente in silenzio, e mente proprio nel caso che conta — nessuno
-- esce da un gruppo di spesa dopo aver saldato tutto per bene.
--
-- CORREZIONE
-- I membri usciti restano **finché il loro saldo netto è diverso da zero**. Chi
-- esce a zero sparisce, che è il comportamento voluto: non c'è niente da dire su
-- di lui.
--
-- La condizione non può stare nella `WHERE` della vista originale, perché
-- `net_balance` è calcolato nella `SELECT` e una `WHERE` non può nominare un alias
-- della stessa `SELECT`. Da qui la CTE `balances`: calcola, e il filtro si applica
-- **al numero pubblicato**, cast compreso — così una riga che uscirebbe come
-- `0.00` non può essere trattenuta da un residuo di precisione che nessuno vede.
--
-- ALTERNATIVA SCARTATA: RIFIUTARE L'USCITA A SALDO APERTO
-- Rende il difetto irraggiungibile invece di correggerlo, e non costerebbe una
-- migration. Il prezzo si paga sul caso reale: il coinquilino che se ne va davvero
-- senza pagare resterebbe membro per sempre, e il gruppo non potrebbe chiudersi.
-- **Il debito è del gruppo, non della membership.**
--
-- ALTERNATIVA SCARTATA: ACCETTARE E DOCUMENTARE
-- Sarebbe la seconda volta che questo progetto lascia una vista mentire, e la
-- prima è costata una migration correttiva e due ADR (0024, ADR-028, ADR-032). Si
-- fa adesso, mentre nessun client legge questi numeri.
--
-- CONSEGUENZA DA SAPERE: CHI È USCITO NON VEDE IL PROPRIO SALDO
-- La vista è `security_invoker`, quindi le righe passano da
-- `expense_group_members_select`, che chiede `can_access_expense_group(group_id)`
-- — vera per lo staff e per chi ha una membership **attiva**. Ne segue che il
-- debitore uscito compare nei saldi che leggono gli **altri**, e non nei propri:
-- per lui il gruppo è invisibile. È corretto rispetto all'isolamento e non è ciò
-- che questa migration si propone di cambiare — il difetto era che il debito
-- sparisse per **tutti**. Farlo vedere anche a chi è uscito è un allargamento
-- della visibilità, cioè una decisione di prodotto con la sua policy, non un
-- effetto collaterale da prendersi qui.
--
-- `security_invoker = true` è **ripetuto per esteso**, per la ragione della `0024`:
-- senza il flag la vista girerebbe con i diritti del proprietario e ogni
-- organizzazione vedrebbe i saldi di tutte. `CREATE OR REPLACE VIEW` conserva le
-- `reloptions` esistenti quando la clausola manca, ma affidarsi a quella
-- conservazione significa che il giorno in cui qualcuno ricrea la vista da zero la
-- RLS si apre in silenzio. Il controllo 3 delle invarianti dello schema è la rete
-- sotto questa riga, non il suo sostituto.
--
-- `CREATE OR REPLACE` e non `DROP`+`CREATE`: nessuna vista dipende oggi da questa,
-- ma il `REPLACE` costa lo stesso e impone che nomi, ordine e tipi delle colonne
-- restino identici — che è precisamente la proprietà che si vuole difendere.
-- =============================================================================

CREATE OR REPLACE VIEW app.v_expense_group_balances WITH (security_invoker = true) AS
WITH paid AS (
    SELECT e.group_id, e.paid_by_user_id AS user_id, SUM(e.amount) AS amount
      FROM app.expenses e
     WHERE e.deleted_at IS NULL
     GROUP BY 1, 2
), owed AS (
    SELECT e.group_id, s.user_id, SUM(s.share_amount) AS amount
      FROM app.expense_shares s
      JOIN app.expenses e ON e.id = s.expense_id AND e.deleted_at IS NULL
     GROUP BY 1, 2
), settled_out AS (
    SELECT st.group_id, st.from_user_id AS user_id, SUM(st.amount) AS amount
      FROM app.settlements st WHERE st.status = 'confirmed' GROUP BY 1, 2
), settled_in AS (
    SELECT st.group_id, st.to_user_id AS user_id, SUM(st.amount) AS amount
      FROM app.settlements st WHERE st.status = 'confirmed' GROUP BY 1, 2
), balances AS (
    SELECT m.org_id,
           m.group_id,
           m.user_id,
           m.left_at,
           COALESCE(paid.amount, 0)::app.money_amount        AS total_paid,
           COALESCE(owed.amount, 0)::app.money_amount        AS total_owed,
           COALESCE(settled_out.amount, 0)::app.money_amount AS total_settled_out,
           COALESCE(settled_in.amount, 0)::app.money_amount  AS total_settled_in,
           ( COALESCE(paid.amount, 0) - COALESCE(owed.amount, 0)
             + COALESCE(settled_out.amount, 0) - COALESCE(settled_in.amount, 0)
           )::app.money_amount                               AS net_balance
      FROM app.expense_group_members m
      LEFT JOIN paid        ON paid.group_id        = m.group_id AND paid.user_id        = m.user_id
      LEFT JOIN owed        ON owed.group_id        = m.group_id AND owed.user_id        = m.user_id
      LEFT JOIN settled_out ON settled_out.group_id = m.group_id AND settled_out.user_id = m.user_id
      LEFT JOIN settled_in  ON settled_in.group_id  = m.group_id AND settled_in.user_id  = m.user_id
)
SELECT org_id,
       group_id,
       user_id,
       total_paid,
       total_owed,
       total_settled_out,
       total_settled_in,
       net_balance
  FROM balances
 -- Chi è uscito resta finché deve o gli è dovuto qualcosa. Il debito è del
 -- gruppo, non della membership.
 WHERE left_at IS NULL OR net_balance <> 0;
