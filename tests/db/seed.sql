-- =============================================================================
-- Seed deterministico per i test di isolamento RLS.
-- Eseguito come propflow_migrator (owner delle tabelle → esente da RLS).
--
-- Due organizzazioni indipendenti, ognuna con un proprietario e un inquilino.
-- =============================================================================

\set ON_ERROR_STOP on

BEGIN;

-- Organizzazioni -------------------------------------------------------------
INSERT INTO app.organizations (id, name, slug, kind) VALUES
  ('a0000000-0000-4000-8000-000000000001', 'Org Alfa', 'org-alfa', 'personal'),
  ('b0000000-0000-4000-8000-000000000001', 'Org Beta', 'org-beta', 'personal');

-- Utenti ---------------------------------------------------------------------
INSERT INTO app.users (id, email, full_name, status, password_hash) VALUES
  ('a1000000-0000-4000-8000-000000000001', 'owner.alfa@example.com',  'Owner Alfa',  'active', 'x'),
  ('a1000000-0000-4000-8000-000000000002', 'tenant.alfa@example.com', 'Tenant Alfa', 'active', 'x'),
  ('b1000000-0000-4000-8000-000000000001', 'owner.beta@example.com',  'Owner Beta',  'active', 'x'),
  ('b1000000-0000-4000-8000-000000000002', 'tenant.beta@example.com', 'Tenant Beta', 'active', 'x');

INSERT INTO app.org_members (org_id, user_id, role, status, joined_at) VALUES
  ('a0000000-0000-4000-8000-000000000001', 'a1000000-0000-4000-8000-000000000001', 'owner',  'active', now()),
  ('a0000000-0000-4000-8000-000000000001', 'a1000000-0000-4000-8000-000000000002', 'tenant', 'active', now()),
  ('b0000000-0000-4000-8000-000000000001', 'b1000000-0000-4000-8000-000000000001', 'owner',  'active', now()),
  ('b0000000-0000-4000-8000-000000000001', 'b1000000-0000-4000-8000-000000000002', 'tenant', 'active', now());

-- Immobili -------------------------------------------------------------------
INSERT INTO app.properties (id, org_id, owner_id, label, address_street, city, status, is_coliving) VALUES
  ('a2000000-0000-4000-8000-000000000001', 'a0000000-0000-4000-8000-000000000001',
   'a1000000-0000-4000-8000-000000000001', 'Alfa 1', 'Via Alfa 1', 'Milano', 'rented', true),
  -- secondo immobile della stessa org in cui l'inquilino NON abita
  ('a2000000-0000-4000-8000-000000000002', 'a0000000-0000-4000-8000-000000000001',
   'a1000000-0000-4000-8000-000000000001', 'Alfa 2', 'Via Alfa 2', 'Milano', 'available', false),
  ('b2000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-000000000001',
   'b1000000-0000-4000-8000-000000000001', 'Beta 1', 'Via Beta 1', 'Roma', 'rented', false);

-- Contratti ------------------------------------------------------------------
INSERT INTO app.leases (id, org_id, property_id, lease_type, status, start_date, end_date,
                        rent_amount, deposit_amount) VALUES
  ('a3000000-0000-4000-8000-000000000001', 'a0000000-0000-4000-8000-000000000001',
   'a2000000-0000-4000-8000-000000000001', 'whole_property', 'active',
   CURRENT_DATE - 30, CURRENT_DATE + 335, 800.00, 1600.00),
  ('b3000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-000000000001',
   'b2000000-0000-4000-8000-000000000001', 'whole_property', 'active',
   CURRENT_DATE - 30, CURRENT_DATE + 335, 950.00, 1900.00);

INSERT INTO app.lease_parties (org_id, lease_id, user_id, role, full_name, is_primary) VALUES
  ('a0000000-0000-4000-8000-000000000001', 'a3000000-0000-4000-8000-000000000001',
   'a1000000-0000-4000-8000-000000000002', 'tenant', 'Tenant Alfa', true),
  ('b0000000-0000-4000-8000-000000000001', 'b3000000-0000-4000-8000-000000000001',
   'b1000000-0000-4000-8000-000000000002', 'tenant', 'Tenant Beta', true);

-- Addebiti + un pagamento parziale ------------------------------------------
INSERT INTO app.charges (id, org_id, lease_id, charge_type, status, amount, due_date, issued_at) VALUES
  ('a4000000-0000-4000-8000-000000000001', 'a0000000-0000-4000-8000-000000000001',
   'a3000000-0000-4000-8000-000000000001', 'rent', 'partially_paid', 800.00, CURRENT_DATE - 5, now()),
  ('b4000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-000000000001',
   'b3000000-0000-4000-8000-000000000001', 'rent', 'issued', 950.00, CURRENT_DATE + 10, now());

INSERT INTO app.payments (org_id, payable_type, payable_id, payer_user_id, amount, status,
                          paid_at, idempotency_key) VALUES
  ('a0000000-0000-4000-8000-000000000001', 'charge', 'a4000000-0000-4000-8000-000000000001',
   'a1000000-0000-4000-8000-000000000002', 300.00, 'succeeded', now(), 'seed-pay-alfa-1');

-- Ticket con nota interna ----------------------------------------------------
INSERT INTO app.tickets (id, org_id, property_id, lease_id, title, description, status, reported_by) VALUES
  ('a5000000-0000-4000-8000-000000000001', 'a0000000-0000-4000-8000-000000000001',
   'a2000000-0000-4000-8000-000000000001', 'a3000000-0000-4000-8000-000000000001',
   'Perdita sotto il lavello', 'Gocciola dal sifone', 'open',
   'a1000000-0000-4000-8000-000000000002');

INSERT INTO app.ticket_messages (org_id, ticket_id, author_id, author_kind, body, is_internal) VALUES
  ('a0000000-0000-4000-8000-000000000001', 'a5000000-0000-4000-8000-000000000001',
   'a1000000-0000-4000-8000-000000000002', 'tenant', 'Ho chiuso il rubinetto generale.', false),
  ('a0000000-0000-4000-8000-000000000001', 'a5000000-0000-4000-8000-000000000001',
   'a1000000-0000-4000-8000-000000000001', 'owner',  'Nota interna: chiamare idraulico X.', true);

-- Gruppo Splitwise co-living -------------------------------------------------
INSERT INTO app.expense_groups (id, org_id, property_id, kind, name, created_by) VALUES
  ('a6000000-0000-4000-8000-000000000001', 'a0000000-0000-4000-8000-000000000001',
   'a2000000-0000-4000-8000-000000000001', 'coliving', 'Alfa 1 — Coinquilini',
   'a1000000-0000-4000-8000-000000000002');

INSERT INTO app.expense_group_members (org_id, group_id, user_id) VALUES
  ('a0000000-0000-4000-8000-000000000001', 'a6000000-0000-4000-8000-000000000001',
   'a1000000-0000-4000-8000-000000000002'),
  ('a0000000-0000-4000-8000-000000000001', 'a6000000-0000-4000-8000-000000000001',
   'a1000000-0000-4000-8000-000000000001');

INSERT INTO app.expenses (id, org_id, group_id, paid_by_user_id, category, description, amount) VALUES
  ('a7000000-0000-4000-8000-000000000001', 'a0000000-0000-4000-8000-000000000001',
   'a6000000-0000-4000-8000-000000000001', 'a1000000-0000-4000-8000-000000000002',
   'utilities', 'Bolletta luce marzo', 100.00);

INSERT INTO app.expense_shares (org_id, expense_id, user_id, share_amount) VALUES
  ('a0000000-0000-4000-8000-000000000001', 'a7000000-0000-4000-8000-000000000001',
   'a1000000-0000-4000-8000-000000000002', 50.00),
  ('a0000000-0000-4000-8000-000000000001', 'a7000000-0000-4000-8000-000000000001',
   'a1000000-0000-4000-8000-000000000001', 50.00);

COMMIT;
