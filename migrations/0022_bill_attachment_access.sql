-- =============================================================================
-- app.can_access_attachment riallineata alla policy della bolletta.
--
-- DIFETTO
-- La funzione risolve `utility_bill` su `app.can_access_property(b.property_id)`,
-- mentre `utility_bills_select` chiede staff **oppure** essere parte del contratto
-- indicato:
--
--   utility_bills_select  →  is_org_staff(org_id)
--                            OR (lease_id IS NOT NULL AND can_access_lease(lease_id))
--   can_access_attachment →  can_access_property(b.property_id)
--
-- `can_access_property` è vera per **ogni** inquilino dell'immobile. In un
-- co-living con contratti per stanza, il coinquilino A soddisfa quindi la seconda
-- e non la prima: **non vede la riga** della bolletta imputata al contratto di B,
-- e ne scaricherebbe la scansione — la fattura con il nome, l'indirizzo e i
-- consumi di B.
--
-- Finché nessuna bolletta aveva allegati la cosa non si vedeva. È questa fetta ad
-- aprire le rotte `/api/v1/utility-bills/{id}/documents`, ed è quindi questa fetta
-- a doverla correggere: la migration viene **prima** del bordo HTTP, non dopo.
--
-- CORREZIONE
-- La regola diventa quella che si legge in una riga: **il documento di una
-- bolletta si vede se si vede la bolletta.** Il ramo ripete la condizione della
-- policy invece di chiamarla, perché una policy non è invocabile; il modo di
-- tenerle allineate è il test di integrazione che prova entrambe le vie sullo
-- stesso co-living.
--
-- `deleted_at IS NULL` compare qui e **non** è il filtro di business che ADR-008
-- vieta nelle policy di SELECT: questa è una funzione, non una policy, e senza
-- quella riga l'allegato di una bolletta annullata resterebbe scaricabile mentre
-- la bolletta è sparita da entrambe le parti. La policy `documents_select` che la
-- chiama ha il proprio `deleted_at IS NULL` sul documento, non sulla bolletta.
--
-- CONSEGUENZE SUI DATI ESISTENTI
-- Nessuna: **nessun documento ha oggi `owner_type = 'utility_bill'`**, perché
-- l'enum di dominio non lo contemplava e il caricamento lo rifiutava. La
-- correzione restringe l'accesso — si vede meno, non di più — quindi non può
-- aprire nulla che fosse chiuso.
--
-- IL RAMO `charge` NON SI TOCCA, ED È CORRETTO COSÌ
-- Risolve su `can_access_lease(c.lease_id)`, che è **esattamente** la condizione
-- di `charges_select`. Le due parti già coincidono, e allinearle di nuovo per
-- simmetria significherebbe riscrivere una riga giusta.
--
-- Diventerà invece disallineato il giorno in cui qualcuno introducesse la bozza
-- vera dell'addebito: restringere `charges_select` con
-- `status <> 'draft' OR is_org_staff(org_id)` senza toccare questa funzione
-- lascerebbe l'inquilino a scaricare gli allegati di una bozza che non vede. Le
-- due migration vanno insieme, e questo commento è il posto in cui sta scritto.
-- =============================================================================

CREATE OR REPLACE FUNCTION app.can_access_attachment(
    p_owner_type app.attachment_owner_type, p_owner_id uuid
) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = app, pg_temp COST 500 AS
$$
  SELECT CASE p_owner_type
    WHEN 'property'     THEN app.can_access_property(p_owner_id)
    WHEN 'lease'        THEN app.can_access_lease(p_owner_id)
    WHEN 'ticket'       THEN app.can_access_ticket(p_owner_id)
    WHEN 'user'         THEN p_owner_id = app.current_user_id()
    WHEN 'room'         THEN EXISTS (SELECT 1 FROM app.rooms r
                                      WHERE r.id = p_owner_id
                                        AND app.can_access_property(r.property_id))
    WHEN 'expense'      THEN EXISTS (SELECT 1 FROM app.expenses e
                                      WHERE e.id = p_owner_id
                                        AND app.can_access_expense_group(e.group_id))
    -- Allineata a utility_bills_select: staff oppure parte del contratto imputato.
    WHEN 'utility_bill' THEN EXISTS (SELECT 1 FROM app.utility_bills b
                                      WHERE b.id = p_owner_id
                                        AND b.deleted_at IS NULL
                                        AND b.org_id = app.current_org_id()
                                        AND (app.is_org_staff(b.org_id)
                                             OR (b.lease_id IS NOT NULL
                                                 AND app.can_access_lease(b.lease_id))))
    WHEN 'charge'       THEN EXISTS (SELECT 1 FROM app.charges c
                                      WHERE c.id = p_owner_id
                                        AND app.can_access_lease(c.lease_id))
    -- organization / payment / ticket_message: solo staff
    ELSE false
  END
$$;
