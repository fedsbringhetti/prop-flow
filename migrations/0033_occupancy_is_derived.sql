-- =============================================================================
-- L'occupazione si deriva, e la vista smette di dire zero a tutti.
--
-- IL DIFETTO, MISURATO E NON DEDOTTO
-- `v_property_occupancy` conta le stanze così:
--
--   count(r.id) FILTER (WHERE r.status = 'occupied')  AS rooms_occupied
--
-- e `app.room_status = 'occupied'` **non lo scrive nessuno**: né uno use case né
-- un trigger. `rooms_occupied` e `occupancy_percent` sono quindi zero per ogni
-- immobile del sistema, sempre, da quando la vista esiste. Nessuno se n'era
-- accorto perché quella vista la legge un test solo, e legge `active_leases`, che
-- passa da `lease_in_force` ed è giusta.
--
-- È lo stesso difetto che ADR-019 ha corretto sui contratti — una vista che conta
-- uno stato che mente — rovesciato: là lo stato era stantio, qui non viene scritto
-- affatto.
--
-- PERCHÉ NON UN TRIGGER, CHE SAREBBE LA VIA OVVIA
-- Due `UPDATE`, uno sull'attivazione e uno sulla disdetta, e la colonna resterebbe
-- vera. Non resterebbe vera: **l'occupazione dipende dal calendario**. Un contratto
-- attivato oggi che comincia il primo del mese prossimo non occupa niente adesso, e
-- uno in essere fino a domani smette di occupare senza che nessuno tocchi niente.
-- Un trigger non scatta al passare del tempo; servirebbe uno scheduler notturno,
-- che è precisamente ciò che ADR-019 ha già scartato — un componente da far girare,
-- monitorare e recuperare dopo un fermo, e comunque una finestra in cui il database
-- dice una cosa e il calendario un'altra.
--
-- Quindi si deriva alla lettura, con la **stessa** definizione di «in essere» che
-- il progetto ha già: `app.lease_in_force`, e nessuna seconda.
--
-- UNA STANZA È OCCUPATA ANCHE DA UN CONTRATTO SULL'IMMOBILE INTERO
-- Non è un dettaglio: chi affitta tutto l'appartamento occupa ogni stanza che c'è
-- dentro, e contare solo i contratti che nominano la stanza direbbe che un
-- appartamento interamente affittato ha zero stanze occupate — cioè lo stesso zero
-- di adesso, per un'altra ragione.
--
-- `SECURITY DEFINER`, COME LA `0030`
-- La funzione legge `app.leases`, che sotto Row Level Security è ristretta a
-- `can_access_lease`: per un inquilino sarebbe vera solo sulla **propria** stanza e
-- falsa su quelle dei coinquilini, cioè una risposta diversa a seconda di chi
-- guarda per un fatto che non dipende da chi guarda. Il confine resta nel corpo,
-- come in `app.property_residents`: risponde solo a chi vede già l'immobile.
-- =============================================================================

-- ---------------------------------------------------------------------------
-- Una stanza è occupata se un contratto in essere la impegna
-- ---------------------------------------------------------------------------

CREATE FUNCTION app.room_is_occupied(p_room_id uuid)
RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 200 AS
$$
  SELECT EXISTS (
    SELECT 1
      FROM app.rooms r
      JOIN app.leases l
        ON l.deleted_at IS NULL
       -- Il contratto sulla stanza, oppure quello sull'immobile intero: chi
       -- affitta tutta la casa occupa anche le sue stanze.
       AND (l.room_id = r.id OR (l.room_id IS NULL AND l.property_id = r.property_id))
     WHERE r.id = p_room_id
       AND r.deleted_at IS NULL
       -- Il confine, dentro la funzione: chi non vede l'immobile non riceve
       -- risposta. Stessa forma di `app.property_residents`.
       AND app.can_access_property(r.property_id)
       AND app.lease_in_force(l.status, l.start_date, l.end_date)
  )
$$;

COMMENT ON FUNCTION app.room_is_occupied IS
  'Una stanza è occupata se un contratto in essere la impegna — il suo, o quello
   sull''immobile intero. Derivata e non memorizzata: `app.room_status = ''occupied''`
   non lo scrive nessuno, e non potrebbe restare vero perché l''occupazione dipende
   dal calendario (ADR-019).';

-- ---------------------------------------------------------------------------
-- Lo stato effettivo di un immobile
--
-- Restituisce **NULL** quando non c'è niente da derivare, e sta al chiamante
-- ripiegare sullo stato memorizzato: un immobile senza contratti in essere non è
-- «disponibile» per forza — potrebbe essere una bozza o in manutenzione, che sono
-- giudizi di chi amministra e non conseguenze.
-- ---------------------------------------------------------------------------

CREATE FUNCTION app.property_occupancy_status(p_property_id uuid)
RETURNS app.property_status
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  WITH stanze AS (
    SELECT app.room_is_occupied(r.id) AS occupata
      FROM app.rooms r
     WHERE r.property_id = p_property_id AND r.deleted_at IS NULL
  ),
  intero AS (
    SELECT EXISTS (
      SELECT 1 FROM app.leases l
       WHERE l.property_id = p_property_id
         AND l.room_id IS NULL
         AND l.deleted_at IS NULL
         AND app.lease_in_force(l.status, l.start_date, l.end_date)
    ) AS affittato
  )
  SELECT CASE
    -- Il confine vale anche qui: senza, si saprebbe se un immobile è affittato
    -- senza poterlo vedere.
    WHEN NOT app.can_access_property(p_property_id) THEN NULL
    -- Un contratto sull'immobile intero lo affitta tutto, stanze o no.
    WHEN (SELECT affittato FROM intero) THEN 'rented'::app.property_status
    WHEN (SELECT count(*) FROM stanze) = 0 THEN NULL
    WHEN (SELECT count(*) FROM stanze WHERE occupata) = 0 THEN NULL
    WHEN (SELECT count(*) FROM stanze WHERE NOT occupata) = 0
      THEN 'rented'::app.property_status
    ELSE 'partially_rented'::app.property_status
  END
$$;

COMMENT ON FUNCTION app.property_occupancy_status IS
  'Lo stato che discende dai contratti: ''rented'' o ''partially_rented'', e NULL
   quando non c''è niente da derivare — nessuna stanza occupata non significa
   ''available'', che è un giudizio. Chi legge ripiega sullo stato memorizzato.';

-- ---------------------------------------------------------------------------
-- La vista torna a contare ciò che intendeva contare
--
-- `CREATE OR REPLACE` impone colonne identiche in nome, tipo e ordine: cambia solo
-- il predicato, come nella `0017`.
-- ---------------------------------------------------------------------------

CREATE OR REPLACE VIEW app.v_property_occupancy WITH (security_invoker = true) AS
SELECT p.id AS property_id,
       p.org_id,
       p.is_coliving,
       count(r.id)                                            AS rooms_total,
       count(r.id) FILTER (WHERE app.room_is_occupied(r.id))   AS rooms_occupied,
       CASE WHEN count(r.id) > 0
            THEN round(100.0 * count(r.id) FILTER (WHERE app.room_is_occupied(r.id))
                       / count(r.id), 2)
       END                                                    AS occupancy_percent,
       count(DISTINCT l.id) FILTER (
         WHERE app.lease_in_force(l.status, l.start_date, l.end_date)
       )                                                      AS active_leases,
       SUM(l.rent_amount) FILTER (
         WHERE app.lease_in_force(l.status, l.start_date, l.end_date)
       )::app.money_amount                                    AS active_monthly_rent
  FROM app.properties p
  LEFT JOIN app.rooms  r ON r.property_id = p.id AND r.deleted_at IS NULL
  LEFT JOIN app.leases l ON l.property_id = p.id AND l.deleted_at IS NULL
 WHERE p.deleted_at IS NULL
 GROUP BY p.id, p.org_id, p.is_coliving;

-- ---------------------------------------------------------------------------
-- Privilegi
--
-- I privilegi su una funzione nuova non arrivano dal `GRANT ... ON ALL FUNCTIONS`
-- della `0015`, che si applica solo a ciò che esisteva allora. Il ramo su
-- `pg_roles` è quello della `0030`: su un database appena creato i ruoli possono
-- non esserci ancora.
-- ---------------------------------------------------------------------------
DO $$
DECLARE r text;
BEGIN
  FOREACH r IN ARRAY ARRAY['propflow_app', 'propflow_worker'] LOOP
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = r) THEN
      EXECUTE format('GRANT EXECUTE ON FUNCTION app.room_is_occupied(uuid) TO %I', r);
      EXECUTE format(
        'GRANT EXECUTE ON FUNCTION app.property_occupancy_status(uuid) TO %I', r);
    END IF;
  END LOOP;
END $$;
