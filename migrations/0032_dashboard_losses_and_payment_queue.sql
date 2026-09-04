-- =============================================================================
-- Il nono numero della dashboard, e l'indice della coda delle dichiarazioni.
--
-- Due cose in una migration perché sono lo stesso lavoro: chiudere il giro del
-- denaro dal lato del proprietario. La prima gli dice quanto ha perso, la seconda
-- gli fa trovare chi dice di aver pagato. Separarle darebbe due migration che
-- toccano lo stesso modulo nello stesso giorno.
--
-- =============================================================================
-- 1. `written_off_total`: la perdita esce dall'indistinto
-- =============================================================================
--
-- DIFETTO
-- `app.v_owner_dashboard` porta otto numeri e in nessuno si vede una perdita.
-- `void` e `written_off` escono entrambi da `total_outstanding` e da
-- `overdue_charges` — dalla `0024`, che è corretta — e da lì in poi sono
-- **indistinguibili**: la dashboard mostra un insoluto che scende e non dice se è
-- sceso perché qualcuno ha pagato o perché si è rinunciato a incassare.
--
-- I due stati sono diversi in natura, e la distinzione è già scritta nel dominio:
-- `void` dice «non doveva essere chiesto», `written_off` dice «era dovuto e ci
-- rinunciamo». Solo il secondo è una perdita, e la dashboard è il primo report.
--
-- CORREZIONE
-- Un nono numero, sommato sui soli addebiti `written_off`.
--
-- LA PERDITA È IL RESIDUO, NON L'IMPORTO — ED È LA PARTE CHE SI SBAGLIA
-- Un addebito da 800.00 con 300.00 incassati e poi stornato è una perdita di
-- **500.00**: gli altri trecento sono entrati e non si sono persi. La somma è
-- quindi `amount - paid_amount`, e non `amount`.
--
-- **`outstanding_amount` non può produrla**, ed è la trappola: la `0024` lo azzera
-- proprio su `written_off`. È la stessa colonna che rende questo numero necessario
-- — è azzerandola che la perdita è sparita dalla vista — e che non può calcolarlo.
-- Si somma quindi da `amount` e `paid_amount`, che nella vista dei saldi restano
-- entrambe vere: `paid_amount` è un fatto e non una pretesa, e la `0024` non la
-- tocca.
--
-- IL `GREATEST(..., 0)` NON C'È, ED È DELIBERATO
-- Un sovrapagamento su un addebito poi stornato darebbe `amount - paid_amount`
-- negativo, cioè una perdita negativa che abbassa il totale. Troncare a zero
-- nasconderebbe uno scarto che in contabilità è precisamente ciò che si vuole
-- vedere — è lo stesso argomento con cui la `0024` tiene `paid` fuori dagli stati
-- che azzerano. Il caso è raro (bisogna sovrapagare *e* poi stornare) e, se
-- accade, il numero deve mostrarlo.
--
-- PERCHÉ SERVIVA PRIMA LO STORNO DA `partially_paid`
-- Finché `Charge::write_off` era ammesso dal solo `issued`, un addebito su cui
-- fosse entrato del denaro **non era stornabile**: questo numero avrebbe contato
-- metà delle perdite senza che chi lo legge avesse modo di saperlo. Le due cose
-- stanno nella stessa fetta per questo. Nessuna migration per quel lato: la `0023`
-- concede già l'`UPDATE` sulla sola `status`.
--
-- IL CORPO SI RIPRENDE DALLA `0017`, NON DALLA `0015` — ED È LA TRAPPOLA
-- `CREATE OR REPLACE VIEW` **riscrive la vista per intero**: la definizione che si
-- copia diventa la definizione, e ogni correzione applicata da una migration
-- intermedia sparisce senza che nulla protesti. Questa vista è stata definita
-- nella `0015` e **corretta dalla `0017`**, che ha sostituito `status = 'active'`
-- con `app.lease_in_force(...)` in tre punti — `active_leases`,
-- `monthly_rent_expected` e `leases_expiring_90d` — perché un contratto a termine
-- resta `active` anche dopo la scadenza e nessuno scheduler lo riscrive (ADR-019).
--
-- Ripartire dal corpo della `0015`, che è il primo che si trova cercando
-- `CREATE VIEW app.v_owner_dashboard`, rimetterebbe in piedi quel difetto: la
-- dashboard tornerebbe a contare fra i contratti in essere quelli scaduti, e a
-- sommarne i canoni. Non fallirebbe nulla in migrazione, e il sintomo sarebbe due
-- numeri gonfi. Il corpo qui sotto è quello della `0017` con una colonna in fondo.
--
-- LO SCHEMA È APPEND-ONLY: `CREATE OR REPLACE VIEW`, LE PRECEDENTI NON SI TOCCANO
-- Nomi, ordine e tipi delle otto colonne esistenti restano identici e la nona è in
-- fondo, che è la condizione perché il `REPLACE` sia ammesso — aggiungere una
-- colonna in mezzo, o cambiarne il tipo, richiederebbe un `DROP`, e un `DROP`
-- porterebbe via le dipendenze. La `0015` e la `0017` non si toccano.
--
-- `security_invoker = true` è **ripetuto per esteso**, come nella `0024`: senza il
-- flag la vista girerebbe con i diritti del proprietario e ogni organizzazione
-- vedrebbe i numeri di tutte. `CREATE OR REPLACE VIEW` conserva le `reloptions`
-- quando la clausola manca, ma affidarsi a quella conservazione significa che il
-- giorno in cui qualcuno ricrea la vista da zero la RLS si apre in silenzio. Il
-- controllo 3 delle invarianti dello schema è la rete sotto questa riga, non il
-- suo sostituto.
--
-- CONSEGUENZE
-- **`total_outstanding` non cambia**, e non deve: un credito non compensa un
-- insoluto, e una perdita non è un insoluto. Il nono numero si aggiunge, non
-- ridefinisce. Aggiungere un campo è additivo; cambiare cosa significa un campo
-- già pubblicato è una rottura, e un client legge già `total_outstanding`.
--
-- =============================================================================
-- 2. `payments_pending_ix`: la coda delle dichiarazioni
-- =============================================================================
--
-- `GET /api/v1/payments?status=pending` è la coda che il proprietario guarda per
-- sapere chi dice di aver pagato. Senza, una dichiarazione si scopre solo aprendo
-- gli addebiti a uno a uno — e la notifica avviserebbe di qualcosa che poi non si
-- trova.
--
-- PERCHÉ UN INDICE NUOVO, CON `payments_org_status_ix` GIÀ LÌ
-- Quello esistente è `(org_id, status, created_at DESC)` e servirebbe il filtro,
-- ma **non l'ordinamento**: la paginazione keyset di questa API ordina per `id`,
-- che con gli UUIDv7 equivale a ordinare per data di creazione senza una colonna
-- in più. Con quell'indice il planner filtra e poi ordina; con questo scorre già
-- in ordine e si ferma alla pagina.
--
-- PARZIALE SU `pending`, E NON SU TUTTI GLI STATI: è la stessa forma di
-- `charges_outstanding_ix` e di `tickets_open_ix`. `pending` è una popolazione
-- che resta piccola per costruzione — una dichiarazione ci sta il tempo che lo
-- staff impiega a guardarla — mentre `succeeded` cresce con ogni incasso e per
-- sempre. Un indice su tutti gli stati costerebbe quanto la tabella per servire
-- la frazione che interessa.
--
-- `org_id` È LA PRIMA COLONNA E NESSUNA QUERY LO NOMINA
-- A metterlo nel `WHERE` è `payments_select`, che è `org_id = current_org_id()`:
-- la Row Level Security aggiunge il predicato della policy alla query come un qual
-- qualsiasi, e da lì il planner lo usa come condizione d'indice —
-- `app.current_org_id()` è `STABLE`, quindi valutabile una volta sola. Senza
-- `org_id` in testa l'indice servirebbe l'ordinamento e non la selezione, cioè
-- scorrerebbe le dichiarazioni di ogni organizzazione per scartarle dopo.
--
-- IL FILTRO PARAMETRICO DA SOLO NON USA QUESTO INDICE, E VA SAPUTO
-- Misurato, non dedotto, perché l'intuizione dice il contrario. Il pattern che ogni
-- altro elenco di questo progetto usa —
-- `($1::text IS NULL OR status = $1::app.payment_status)` — su un banco di 80 000
-- pagamenti produce:
--
--   Sort  ->  Bitmap Heap Scan  ->  Bitmap Index Scan on payments_org_status_ix
--
-- cioè l'indice **sbagliato**, seguito da un ordinamento. La causa non è la
-- selettività: è che il cast da `text` a un enum passa da `enum_in`, che PostgreSQL
-- dichiara `STABLE` e non `IMMUTABLE`. `'pending'::text::app.payment_status` non
-- viene quindi ridotto a una costante nemmeno nel piano *custom*, e senza una
-- costante il planner non può dimostrare che il predicato della query implica
-- quello dell'indice parziale.
--
-- È la differenza con `charges_outstanding_ix`, dove il rimedio serviva per una
-- ragione diversa — da un parametro `bool` non si deriva *quali* tre valori
-- l'indice contenga — ma è lo stesso rimedio: **il predicato si scrive per
-- esteso.** Il repository aggiunge una condizione ridondante,
-- `status = 'pending'::app.payment_status`, accesa da un booleano che vale `true`
-- esattamente quando il filtro chiede `pending`. Con quella il piano diventa
--
--   Index Scan using payments_pending_ix
--
-- e ci resta anche dopo le sei esecuzioni che portano al piano generico. Su ogni
-- altro stato il ramo è spento, la condizione è vera per tutte le righe e il
-- risultato non cambia: `?status=` resta un filtro generale.
-- =============================================================================

CREATE OR REPLACE VIEW app.v_owner_dashboard WITH (security_invoker = true) AS
SELECT o.id AS org_id,
       (SELECT count(*) FROM app.properties p
         WHERE p.org_id = o.id AND p.deleted_at IS NULL)                    AS properties_count,
       (SELECT count(*) FROM app.leases l
         WHERE l.org_id = o.id AND l.deleted_at IS NULL
           AND app.lease_in_force(l.status, l.start_date, l.end_date))      AS active_leases,
       (SELECT COALESCE(SUM(l.rent_amount), 0) FROM app.leases l
         WHERE l.org_id = o.id AND l.deleted_at IS NULL
           AND app.lease_in_force(l.status, l.start_date, l.end_date)
       )::app.money_amount                                                  AS monthly_rent_expected,
       (SELECT COALESCE(SUM(b.outstanding_amount), 0) FROM app.v_charge_balances b
         WHERE b.org_id = o.id AND b.outstanding_amount > 0
       )::app.money_amount                                                  AS total_outstanding,
       (SELECT count(*) FROM app.v_charge_balances b
         WHERE b.org_id = o.id AND b.is_past_due)                           AS overdue_charges,
       (SELECT count(*) FROM app.tickets t
         WHERE t.org_id = o.id AND t.deleted_at IS NULL
           AND t.status <> ALL (ARRAY['resolved','closed','cancelled']::app.ticket_status[])
       )                                                                    AS open_tickets,
       -- "in scadenza" è un contratto in essere la cui fine cade nei prossimi 90
       -- giorni: lo stato 'expiring' non viene più memorizzato da nessuno.
       (SELECT count(*) FROM app.leases l
         WHERE l.org_id = o.id AND l.deleted_at IS NULL
           AND app.lease_in_force(l.status, l.start_date, l.end_date)
           AND l.end_date <= CURRENT_DATE + INTERVAL '90 days'
       )                                                                    AS leases_expiring_90d,
       (SELECT count(*) FROM app.utility_bills ub
         WHERE ub.org_id = o.id AND ub.deleted_at IS NULL
           AND ub.status = ANY (ARRAY['draft','pending_review']::app.bill_status[])
       )                                                                    AS bills_pending_review,
       -- La perdita è il **residuo**: `amount - paid_amount`, non `amount`.
       -- `outstanding_amount` qui vale zero per costruzione (migration `0024`), ed
       -- è la ragione per cui questo numero si somma dalle due colonne che restano
       -- vere. Nessun troncamento a zero: uno scarto negativo va visto.
       (SELECT COALESCE(SUM(b.amount - b.paid_amount), 0) FROM app.v_charge_balances b
         WHERE b.org_id = o.id AND b.status = 'written_off'
       )::app.money_amount                                                  AS written_off_total
  FROM app.organizations o
 WHERE o.deleted_at IS NULL;

CREATE INDEX payments_pending_ix ON app.payments (org_id, id)
    WHERE status = 'pending';
