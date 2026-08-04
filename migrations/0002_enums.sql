-- =============================================================================
-- ENUM di dominio. Estendibili con ALTER TYPE ... ADD VALUE (non bloccante).
-- Regola: gli enum descrivono STATI e CLASSIFICAZIONI, mai permessi calcolati.
-- =============================================================================

-- Identity & RBAC ------------------------------------------------------------
CREATE TYPE app.org_kind      AS ENUM ('personal', 'business', 'condominium');
CREATE TYPE app.platform_role AS ENUM ('user', 'support', 'admin');
CREATE TYPE app.user_status   AS ENUM ('pending', 'active', 'suspended', 'deactivated');
CREATE TYPE app.org_role      AS ENUM ('owner', 'manager', 'tenant');
CREATE TYPE app.member_status AS ENUM ('invited', 'active', 'suspended', 'left');

-- Patrimonio -----------------------------------------------------------------
CREATE TYPE app.property_type AS ENUM
  ('apartment', 'house', 'studio', 'villa', 'office', 'retail', 'garage', 'land', 'other');
CREATE TYPE app.property_status AS ENUM
  ('draft', 'available', 'partially_rented', 'rented', 'maintenance', 'archived');
CREATE TYPE app.room_status AS ENUM ('available', 'occupied', 'unavailable');
CREATE TYPE app.energy_class AS ENUM ('A4','A3','A2','A1','A','B','C','D','E','F','G');

-- Contratti ------------------------------------------------------------------
CREATE TYPE app.lease_type AS ENUM ('whole_property', 'room');
CREATE TYPE app.lease_status AS ENUM
  ('draft', 'pending_signature', 'active', 'expiring', 'expired', 'terminated', 'cancelled');
CREATE TYPE app.rent_frequency AS ENUM ('monthly', 'quarterly', 'semiannual', 'annual');
CREATE TYPE app.lease_party_role AS ENUM ('tenant', 'cotenant', 'guarantor');

-- Documenti ------------------------------------------------------------------
CREATE TYPE app.document_kind AS ENUM
  ('lease_contract', 'id_document', 'utility_bill', 'invoice', 'receipt', 'photo',
   'inspection_report', 'energy_certificate', 'cadastral_extract', 'insurance_policy', 'other');
CREATE TYPE app.document_status AS ENUM ('uploaded', 'processing', 'processed', 'failed', 'archived');

-- Riferimento polimorfico (valori = nome tabella al singolare) ---------------
CREATE TYPE app.attachment_owner_type AS ENUM
  ('organization', 'user', 'property', 'room', 'lease', 'ticket', 'ticket_message',
   'utility_bill', 'charge', 'payment', 'expense');

-- Ticket & manutenzione ------------------------------------------------------
CREATE TYPE app.ticket_category AS ENUM
  ('plumbing', 'electrical', 'heating_cooling', 'appliance', 'structural', 'pest',
   'cleaning', 'connectivity', 'security', 'other');
CREATE TYPE app.ticket_priority AS ENUM ('low', 'medium', 'high', 'urgent');
CREATE TYPE app.ticket_status AS ENUM
  ('open', 'triaged', 'assigned', 'in_progress', 'waiting_tenant', 'waiting_owner',
   'resolved', 'closed', 'cancelled');
CREATE TYPE app.ticket_actor AS ENUM ('tenant', 'owner', 'manager', 'system', 'ai');

-- Utenze & contabilità -------------------------------------------------------
CREATE TYPE app.utility_type AS ENUM
  ('electricity', 'gas', 'water', 'heating', 'waste', 'internet', 'condo_fees', 'other');
CREATE TYPE app.bill_status AS ENUM
  ('draft', 'pending_review', 'validated', 'allocated', 'paid', 'disputed', 'void');
CREATE TYPE app.charge_type AS ENUM
  ('rent', 'deposit', 'utility_recharge', 'late_fee', 'maintenance', 'adjustment', 'other');
CREATE TYPE app.charge_status AS ENUM
  ('draft', 'issued', 'partially_paid', 'paid', 'overdue', 'void', 'written_off');

-- Pagamenti (provider-agnostici) --------------------------------------------
CREATE TYPE app.payable_type AS ENUM ('charge', 'expense_share', 'settlement');
CREATE TYPE app.payment_method AS ENUM
  ('bank_transfer', 'sepa_direct_debit', 'card', 'cash', 'check', 'other');
CREATE TYPE app.payment_status AS ENUM
  ('pending', 'authorized', 'succeeded', 'failed', 'cancelled', 'refunded', 'partially_refunded');

-- Splitwise / co-living ------------------------------------------------------
CREATE TYPE app.expense_group_kind AS ENUM ('coliving', 'household', 'property');
CREATE TYPE app.expense_split_method AS ENUM ('equal', 'weighted', 'percentage', 'exact', 'by_usage');
CREATE TYPE app.expense_category AS ENUM
  ('utilities', 'groceries', 'cleaning', 'connectivity', 'maintenance', 'rent', 'furniture', 'other');
CREATE TYPE app.settlement_status AS ENUM ('pending', 'confirmed', 'rejected', 'cancelled');

-- AI (provider astratto) -----------------------------------------------------
CREATE TYPE app.ai_capability AS ENUM ('ocr', 'vision', 'llm', 'embedding');
CREATE TYPE app.ai_job_status AS ENUM ('queued', 'running', 'succeeded', 'failed', 'cancelled');

-- Infrastruttura -------------------------------------------------------------
CREATE TYPE app.event_status AS ENUM ('pending', 'dispatched', 'failed', 'discarded');
CREATE TYPE app.audit_action AS ENUM ('insert', 'update', 'delete');
