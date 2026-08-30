-- =============================================================================
-- Riferimento delle segnalazioni per organizzazione, e note di risoluzione.
--
-- DIFETTO
-- `tickets.reference` nasceva da `app.ticket_reference_seq`, **una sola sequenza
-- per il cluster**: la prima segnalazione di qualunque organizzazione era
-- `TK-000001`, la millesima della piattaforma era `TK-001000` anche se per il suo
-- proprietario era la seconda. Il numero rivelava quante segnalazioni esistono
-- sull'intera piattaforma.
--
-- Non è un dato sensibile ed è una fuga modesta, ma è un dato che non si
-- intendeva dare, e il costo di correggerlo **cresce**: finché nessuno ha dati
-- reali è questa migration, dopo sarebbe rinumerare o convivere con due formati
-- per sempre. È la ragione per cui si fa adesso e non quando darà fastidio.
--
-- CORREZIONE
-- Un contatore per organizzazione, riempito da un trigger `BEFORE INSERT`.
--
-- **Perché un contatore e non una sequenza per organizzazione.** Una sequenza per
-- org richiederebbe DDL dinamico a ogni registrazione — `CREATE SEQUENCE` dentro
-- un trigger — e lascerebbe dietro di sé oggetti di schema che nessuna migration
-- ha creato e che nessuno cancella. Una riga in una tabella si crea, si legge e
-- si porta via con l'organizzazione.
--
-- **Perché un trigger e non l'applicazione.** Così `reference` resta fuori dalla
-- `GRANT UPDATE` della 0020 e fuori dalla portata del ruolo applicativo, che è
-- ciò che il controllo 12 delle invarianti protegge. L'`INSERT` del repository
-- non nomina la colonna e non deve saperne nulla; se un giorno la nominasse, il
-- trigger la sovrascriverebbe comunque.
--
-- **Conseguenza sulla concorrenza, da dire ad alta voce.** `ON CONFLICT DO UPDATE`
-- prende un lock di riga: due aperture concorrenti **nella stessa
-- organizzazione** si serializzano fino al commit. Per un'azione umana su una
-- casa è irrilevante, e in cambio la numerazione non ha buchi — una transazione
-- annullata non consuma un numero, cosa che una sequenza invece fa.
--
-- **L'unicità cambia forma.** `tickets_reference_uk` era su `(reference)`: con i
-- numeri per organizzazione due `TK-000001` in due organizzazioni diverse
-- sarebbero legittimi e l'indice li rifiuterebbe. Diventa `(org_id, reference)`.
--
-- Le righe già presenti non vengono rinumerate: il contatore parte dal numero più
-- alto già usato in quell'organizzazione, così nessun riferimento nuovo può
-- scontrarsi con uno vecchio.
--
-- NOTE DI RISOLUZIONE
-- `resolution_notes` esisteva a schema e nessuno la scriveva: chiudere una
-- segnalazione non lasciava da nessuna parte scritto **cosa** fosse stato fatto,
-- e l'unica traccia era il thread — una conversazione, non una conclusione. Ora
-- la valorizza la chiusura, quindi va aggiunta ai privilegi di colonna della
-- 0020. Resta fuori dall'elenco delle colonne strutturali del controllo 12: è
-- deliberatamente scrivibile.
-- =============================================================================

CREATE TABLE app.ticket_reference_counters (
    org_id      UUID PRIMARY KEY REFERENCES app.organizations (id) ON DELETE RESTRICT,
    -- Ultimo numero **usato**, non il prossimo: il trigger incrementa e legge in
    -- un solo statement, e una colonna che significa "già speso" non si presta a
    -- essere letta come "disponibile".
    last_number BIGINT NOT NULL CHECK (last_number >= 0)
);

-- Nessun `updated_at`: la colonna farebbe scattare il controllo 5 delle
-- invarianti, che pretende il trigger `set_updated_at`, e per un contatore la
-- data dell'ultimo incremento non dice nulla che `last_number` non dica già.

ALTER TABLE app.ticket_reference_counters ENABLE ROW LEVEL SECURITY;

-- Una sola policy, di lettura. Il contatore lo scrive il trigger, che è
-- `SECURITY DEFINER` e gira come proprietario: al ruolo applicativo restano
-- `INSERT` e `UPDATE` a livello di tabella (li concedono i privilegi predefiniti
-- della 0015) ma **nessuna policy che li ammetta**, quindi la RLS li rifiuta. È
-- il modo di lasciare il contatore ispezionabile senza renderlo manipolabile.
CREATE POLICY ticket_reference_counters_select ON app.ticket_reference_counters
    FOR SELECT USING (app.is_org_member(org_id));

-- Semi del contatore per i dati già presenti: il numero più alto già usato in
-- quell'organizzazione, non il conteggio delle righe. Con il conteggio, un'org
-- con `TK-000001` e `TK-000005` ripartirebbe da 3 e si scontrerebbe con
-- `TK-000005` due aperture dopo.
INSERT INTO app.ticket_reference_counters (org_id, last_number)
SELECT org_id,
       COALESCE(max((substring(reference from '^TK-([0-9]+)$'))::bigint), 0)
  FROM app.tickets
 GROUP BY org_id;

CREATE FUNCTION app.tg_ticket_reference() RETURNS trigger
  LANGUAGE plpgsql SECURITY DEFINER SET search_path = app, pg_temp AS $$
DECLARE v_number bigint;
BEGIN
  INSERT INTO app.ticket_reference_counters AS c (org_id, last_number)
  VALUES (NEW.org_id, 1)
  ON CONFLICT (org_id) DO UPDATE SET last_number = c.last_number + 1
  RETURNING c.last_number INTO v_number;

  -- Sovrascrive sempre, anche se il chiamante ha nominato la colonna: il
  -- riferimento non è un valore che l'applicazione possa proporre.
  NEW.reference := 'TK-' || to_char(v_number, 'FM000000');
  RETURN NEW;
END $$;

CREATE TRIGGER ticket_reference BEFORE INSERT
    ON app.tickets FOR EACH ROW EXECUTE FUNCTION app.tg_ticket_reference();

-- Il default globale non serve più, e lasciarlo farebbe consumare un valore
-- della sequenza a ogni inserimento per poi buttarlo via.
ALTER TABLE app.tickets ALTER COLUMN reference DROP DEFAULT;
DROP SEQUENCE app.ticket_reference_seq;

DROP INDEX app.tickets_reference_uk;
CREATE UNIQUE INDEX tickets_reference_uk ON app.tickets (org_id, reference);

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'propflow_app') THEN
    RAISE NOTICE 'ruolo propflow_app assente: privilegi non applicati (ambiente di sviluppo)';
    RETURN;
  END IF;

  -- Si aggiunge alle nove colonne già concesse dalla 0020: la chiusura registra
  -- come è stata risolta la segnalazione.
  GRANT UPDATE (resolution_notes) ON app.tickets TO propflow_app;
END $$;
