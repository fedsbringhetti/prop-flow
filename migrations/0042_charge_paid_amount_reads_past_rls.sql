-- =============================================================================
-- app.charge_paid_amount: il totale incassato si somma oltre la RLS di chi legge.
--
-- DIFETTO
-- `app.v_charge_balances` è `security_invoker = true` dalla 0015, e la scelta è
-- giusta per l'addebito: chi non può vedere quella riga non ne legge il saldo. Ma
-- `paid_amount` è una somma su `app.payments`, e `payments_select` (0014) mostra
-- un pagamento a tre soggetti: lo staff, chi ha pagato (`payer_user_id`), chi ha
-- incassato (`payee_user_id`). Un incasso registrato **senza** `payer_user_id` —
-- il bonifico che arriva da un conto che non si sa a chi ricondurre, che è
-- deliberato e resta tale — è invisibile all'inquilino, quindi non entra nella
-- sua somma.
--
-- Misurato seminando i dati del collaudo: il proprietario registra 150,00 su un
-- addebito da 420,00 senza attribuirlo; l'inquilino, sulla **stessa riga**, legge
--
--   status             partially_paid      ← scritto dal service layer, uguale per tutti
--   paid_amount        0.00                ← sommato sotto la sua RLS
--   outstanding_amount 420.00
--
-- La riga si contraddice da sola: si dichiara pagata in parte e mostra il residuo
-- intero. Chi la legge pagherebbe due volte. È la voce R29.
--
-- CORREZIONE
-- Una funzione `SECURITY DEFINER` che somma senza la RLS del chiamante, e la vista
-- che la chiama al posto della `LATERAL` che sommava in linea. Vedi ADR-055.
--
-- CHE COSA RIVELA, E A CHI
-- Un **aggregato** — quanto è stato incassato su un addebito — a chi può vedere
-- **quell'addebito**, cioè a una parte del suo contratto o allo staff
-- (`charges_select`, via `app.can_access_lease`). La funzione riceve un `charge_id`
-- e restituisce un `numeric`: non chi ha pagato, non quando, non quante righe, non
-- il metodo. `payments_select` **non si tocca**, e l'elenco dei pagamenti continua
-- a mostrare all'inquilino i soli suoi.
--
-- È la forma di ADR-036: una funzione che **proietta** invece di una policy che si
-- allarga. Là erano tre colonne dei residenti, qui è un solo numero.
--
-- IL PREDICATO È LO STESSO DI PRIMA, PAROLA PER PAROLA
-- `succeeded` e `partially_refunded`, `SUM(amount - refunded_amount)`. **Non** sono
-- i tre stati di `payments_payable_ix`, che comprende anche `authorized`: denaro
-- autorizzato e non incassato, che farebbe scendere il saldo su un incasso mai
-- arrivato (0024). Spostare la somma dentro una funzione non è l'occasione per
-- cambiare che cosa si somma — cambierebbero due cose insieme, e il giorno in cui
-- un saldo risultasse sbagliato non si saprebbe quale.
--
-- `CREATE OR REPLACE VIEW` E NON `DROP`+`CREATE`
-- `v_owner_dashboard` legge da questa vista, e un `DROP` la porterebbe via con sé.
-- Nomi, ordine e tipi delle colonne restano identici, che è la condizione perché il
-- `REPLACE` sia ammesso. `security_invoker = true` è ripetuto per esteso: la vista
-- resta quella di chi legge, e cambia **una colonna** di ciascuna riga, non quali
-- righe si leggano.
--
-- IL `REVOKE ... FROM PUBLIC` È NUOVO QUI
-- `app.property_residents` (0030) concede per esplicito ai ruoli ma non toglie il
-- default di `PUBLIC`, e l'invariante 6 dello schema — che ferma un `SECURITY
-- DEFINER` senza `search_path` fissato — non se ne accorge. Una funzione che legge
-- oltre la Row Level Security ed è eseguibile da chiunque abbia una connessione è
-- esattamente la cosa che il `DEFINER` doveva evitare, quindi qui si toglie.
-- =============================================================================

CREATE FUNCTION app.charge_paid_amount(p_charge_id uuid)
RETURNS app.money_amount
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp AS
$$
  SELECT COALESCE(SUM(pm.amount - pm.refunded_amount), 0)::app.money_amount
    FROM app.payments pm
   WHERE pm.payable_type = 'charge'
     AND pm.payable_id = p_charge_id
     AND pm.status = ANY (ARRAY['succeeded','partially_refunded']::app.payment_status[]);
$$;

COMMENT ON FUNCTION app.charge_paid_amount IS
  'Quanto è stato incassato su un addebito, sommato **senza** la Row Level Security
   di chi chiama. Serve a `app.v_charge_balances`: la vista resta `security_invoker`,
   ma `payments_select` nasconde a un inquilino gli incassi registrati senza
   attribuirglieli, e sommarli sotto la sua RLS gli faceva leggere un residuo più
   alto del vero sulla riga che intanto si dichiarava pagata in parte (R29, ADR-055).
   Proietta un **aggregato** e nient''altro: non chi ha pagato, non quando, non
   quante righe. Il controllo di accesso resta su `charges_select`, che decide se
   quell''addebito sia leggibile; questa funzione non lo rifà, e per questo non va
   chiamata da sola su un `charge_id` che il chiamante non ha già potuto leggere.
   Gli stati sommati sono i due della vista, `succeeded` e `partially_refunded`:
   `authorized` è denaro non incassato.';

-- I privilegi su una funzione nuova non arrivano dal `GRANT ... ON ALL FUNCTIONS`
-- della 0015, che si applica solo a ciò che esisteva allora.
REVOKE ALL ON FUNCTION app.charge_paid_amount(uuid) FROM PUBLIC;

DO $$
DECLARE r text;
BEGIN
  FOREACH r IN ARRAY ARRAY['propflow_migrator', 'propflow_app', 'propflow_worker'] LOOP
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = r) THEN
      EXECUTE format(
        'GRANT EXECUTE ON FUNCTION app.charge_paid_amount(uuid) TO %I', r);
    END IF;
  END LOOP;
END $$;

-- Stesse colonne, stesso ordine, stessi tipi della 0024: cambia solo da dove
-- arriva `paid_amount`. La `LATERAL` resta — con la chiamata dentro invece della
-- somma — perché le tre colonne usano quel numero e chiamare la funzione tre volte
-- per riga sarebbe tre letture di `app.payments` dove ne basta una.
CREATE OR REPLACE VIEW app.v_charge_balances WITH (security_invoker = true) AS
SELECT c.id                AS charge_id,
       c.org_id,
       c.lease_id,
       c.charge_type,
       c.status,
       c.due_date,
       c.currency,
       c.amount,
       p.paid_amount                                             AS paid_amount,
       -- La pretesa è caduta: non è dovuto niente, a prescindere dai pagamenti.
       CASE
         WHEN c.status = ANY (ARRAY['void','written_off']::app.charge_status[])
           THEN 0
         ELSE c.amount - p.paid_amount
       END::app.money_amount                                     AS outstanding_amount,
       (c.status <> ALL (ARRAY['void','written_off']::app.charge_status[])
        AND c.due_date < CURRENT_DATE
        AND c.amount > p.paid_amount)                            AS is_past_due
  FROM app.charges c
  LEFT JOIN LATERAL (
        SELECT app.charge_paid_amount(c.id) AS paid_amount
       ) p ON true
 WHERE c.deleted_at IS NULL;
