import type { ApiClient } from './client'
import type {
  BillOcrStatus,
  BillStatus,
  Charge,
  ChargeType,
  Dashboard,
  Invitation,
  Items,
  Lease,
  LeaseDetail,
  LeaseParty,
  LeasePartyRole,
  Page,
  Property,
  PropertyResident,
  PropertyStatus,
  PropertyType,
  RechargeResult,
  Room,
  StoredDocument,
  UtilityBill,
  UtilityType,
  Uuid,
} from './types'

/**
 * Le chiamate all'API, una funzione per endpoint.
 *
 * Funzioni e non metodi di una classe: si importano una per una, e il compilatore
 * dice quali sono usate. Ognuna prende il client come primo argomento, che è ciò
 * che permette ai test di passarne uno finto senza toccare un modulo globale.
 */

const LIMIT = 25

function query(params: Record<string, string | number | boolean | undefined>): string {
  const search = new URLSearchParams()
  for (const [key, value] of Object.entries(params)) {
    if (value === undefined || value === '') continue
    search.set(key, String(value))
  }
  const rendered = search.toString()
  return rendered ? `?${rendered}` : ''
}

// ---------------------------------------------------------------------------

export function getDashboard(client: ApiClient): Promise<Dashboard> {
  return client.get<Dashboard>('/dashboard')
}

// ---------------------------------------------------------------------------
// Immobili

export interface PropertyFilters {
  status?: PropertyStatus | undefined
  city?: string | undefined
  after?: Uuid | undefined
}

export function listProperties(
  client: ApiClient,
  filters: PropertyFilters = {},
): Promise<Page<Property>> {
  return client.get<Page<Property>>(
    `/properties${query({ limit: LIMIT, status: filters.status, city: filters.city, after: filters.after })}`,
  )
}

export function getProperty(client: ApiClient, id: Uuid): Promise<Property> {
  return client.get<Property>(`/properties/${id}`)
}

export interface CreatePropertyInput {
  label: string
  property_type: PropertyType
  address: {
    street: string
    city: string
    province?: string
    postal_code?: string
  }
}

export function createProperty(client: ApiClient, input: CreatePropertyInput): Promise<Property> {
  return client.post<Property>('/properties', input)
}

/**
 * Aggiornamento **parziale**, con la semantica del `PATCH` dell'API.
 *
 * `campo assente` = invariato, `null` = azzerato, `valore` = impostato. Ne segue
 * che il form non può mandare tutti i campi ogni volta con `undefined` al posto
 * dei vuoti: un campo svuotato deve arrivare come `null` esplicito, altrimenti
 * cancellare una nota è impossibile.
 */
export function updateProperty(
  client: ApiClient,
  id: Uuid,
  patch: Record<string, unknown>,
): Promise<Property> {
  return client.patch<Property>(`/properties/${id}`, patch)
}

/** Archiviazione: `204`, e non è una cancellazione. */
export function archiveProperty(client: ApiClient, id: Uuid): Promise<void> {
  return client.delete(`/properties/${id}`)
}

export function listResidents(client: ApiClient, id: Uuid): Promise<Items<PropertyResident>> {
  return client.get<Items<PropertyResident>>(`/properties/${id}/residents`)
}

// ---------------------------------------------------------------------------
// Stanze

export function listRooms(client: ApiClient, propertyId: Uuid): Promise<Items<Room>> {
  return client.get<Items<Room>>(`/properties/${propertyId}/rooms`)
}

export interface CreateRoomInput {
  name: string
  monthly_rent?: string
  size_sqm?: string
  has_private_bathroom?: boolean
  is_furnished?: boolean
}

export function createRoom(
  client: ApiClient,
  propertyId: Uuid,
  input: CreateRoomInput,
): Promise<Room> {
  return client.post<Room>(`/properties/${propertyId}/rooms`, input)
}

export function updateRoom(
  client: ApiClient,
  id: Uuid,
  patch: Record<string, unknown>,
): Promise<Room> {
  return client.patch<Room>(`/rooms/${id}`, patch)
}

export function archiveRoom(client: ApiClient, id: Uuid): Promise<void> {
  return client.delete(`/rooms/${id}`)
}

// ---------------------------------------------------------------------------
// Contratti

export interface LeaseFilters {
  property_id?: Uuid | undefined
  /**
   * **Solo stati memorizzati.** `expiring` ed `expired` sono derivati e
   * risponderebbero `422` con `lease.derived_status_filter`: si filtra con
   * `in_force`.
   */
  status?: 'draft' | 'pending_signature' | 'active' | 'terminated' | 'cancelled' | undefined
  in_force?: boolean | undefined
  after?: Uuid | undefined
}

export function listLeases(client: ApiClient, filters: LeaseFilters = {}): Promise<Page<Lease>> {
  return client.get<Page<Lease>>(
    `/leases${query({
      limit: LIMIT,
      property_id: filters.property_id,
      status: filters.status,
      in_force: filters.in_force,
      after: filters.after,
    })}`,
  )
}

export function getLease(client: ApiClient, id: Uuid): Promise<LeaseDetail> {
  return client.get<LeaseDetail>(`/leases/${id}`)
}

export interface CreateLeaseInput {
  property_id: Uuid
  room_id?: Uuid
  start_date: string
  end_date?: string
  rent_amount: string
  deposit_amount?: string
  rent_due_day?: number
  notes?: string
}

export function createLease(client: ApiClient, input: CreateLeaseInput): Promise<Lease> {
  return client.post<Lease>('/leases', input)
}

export interface AddPartyInput {
  role: LeasePartyRole
  full_name: string
  email?: string
  share_percent?: string
  is_primary?: boolean
}

export function addLeaseParty(
  client: ApiClient,
  leaseId: Uuid,
  input: AddPartyInput,
): Promise<LeaseParty> {
  return client.post<LeaseParty>(`/leases/${leaseId}/parties`, input)
}

/** Attivazione e disdetta sono **sotto-risorse**, non un `PATCH` sullo stato. */
export function activateLease(client: ApiClient, id: Uuid): Promise<Lease> {
  return client.post<Lease>(`/leases/${id}/activate`, {})
}

export function terminateLease(client: ApiClient, id: Uuid, reason?: string): Promise<Lease> {
  return client.post<Lease>(`/leases/${id}/terminate`, reason ? { reason } : {})
}

// ---------------------------------------------------------------------------
// Inviti

export function inviteTenant(
  client: ApiClient,
  leaseId: Uuid,
  input: { email: string; full_name: string },
): Promise<Invitation> {
  return client.post<Invitation>(`/leases/${leaseId}/invitations`, input)
}

export function revokeInvitation(client: ApiClient, id: Uuid): Promise<void> {
  return client.delete(`/invitations/${id}`)
}

/**
 * Accetta un invito, **senza `X-Org-Id`**.
 *
 * È l'operazione che crea l'appartenenza: dichiarare l'organizzazione di cui non
 * si è ancora membri farebbe rispondere `403` a una richiesta legittima.
 */
export function acceptInvitation(
  client: ApiClient,
  token: string,
): Promise<{ organization_id: Uuid; role: string; lease_id?: Uuid }> {
  return client.post('/auth/accept-invitation', { token }, { withoutOrganization: true })
}

// ---------------------------------------------------------------------------
// Bollette

export interface BillFilters {
  property_id?: Uuid | undefined
  lease_id?: Uuid | undefined
  status?: BillStatus | undefined
  utility_type?: UtilityType | undefined
  after?: Uuid | undefined
}

export function listBills(client: ApiClient, filters: BillFilters = {}): Promise<Page<UtilityBill>> {
  return client.get<Page<UtilityBill>>(
    `/utility-bills${query({
      limit: LIMIT,
      property_id: filters.property_id,
      lease_id: filters.lease_id,
      status: filters.status,
      utility_type: filters.utility_type,
      after: filters.after,
    })}`,
  )
}

export function getBill(client: ApiClient, id: Uuid): Promise<UtilityBill> {
  return client.get<UtilityBill>(`/utility-bills/${id}`)
}

export interface RegisterBillInput {
  utility_type: UtilityType
  lease_id?: string
  room_id?: string
  supplier_name?: string
  supplier_vat?: string
  invoice_number?: string
  issue_date?: string
  due_date?: string
  period_start?: string
  period_end?: string
  total_amount: string
  consumption?: { value: string; unit: string }
  notes?: string
}

export function registerBill(
  client: ApiClient,
  propertyId: Uuid,
  input: RegisterBillInput,
): Promise<UtilityBill> {
  return client.post<UtilityBill>(`/properties/${propertyId}/utility-bills`, input)
}

/** Su una bolletta **validata** la correzione riporta lo stato a `draft`. */
export function updateBill(
  client: ApiClient,
  id: Uuid,
  patch: Record<string, unknown>,
): Promise<UtilityBill> {
  return client.patch<UtilityBill>(`/utility-bills/${id}`, patch)
}

export function validateBill(client: ApiClient, id: Uuid): Promise<UtilityBill> {
  return client.post<UtilityBill>(`/utility-bills/${id}/validate`, {})
}

export function voidBill(client: ApiClient, id: Uuid): Promise<UtilityBill> {
  return client.post<UtilityBill>(`/utility-bills/${id}/void`, {})
}

export interface RechargeInput {
  lease_id: Uuid
  amount: string
  due_date: string
  description?: string
}

export function rechargeBill(
  client: ApiClient,
  id: Uuid,
  input: RechargeInput,
): Promise<RechargeResult> {
  return client.post<RechargeResult>(`/utility-bills/${id}/recharge`, input)
}

export function getBillOcr(client: ApiClient, id: Uuid): Promise<BillOcrStatus> {
  return client.get<BillOcrStatus>(`/utility-bills/${id}/ocr`)
}

// ---------------------------------------------------------------------------
// Documenti

/** Il limite del corpo sulle rotte di caricamento: 25 MiB. */
export const UPLOAD_LIMIT_BYTES = 25 * 1024 * 1024

/**
 * Carica un allegato.
 *
 * Il `Content-Type` lo mette il browser, con il boundary del multipart: metterlo
 * a mano lo scrive senza boundary, e il server non riesce a leggere la richiesta.
 *
 * **La dimensione va controllata prima di inviare** — vedi `UPLOAD_LIMIT_BYTES` —
 * perché oltre il limite la risposta è un `413` che non dice quale limite né su
 * quale file.
 */
export function uploadDocument(
  client: ApiClient,
  owner: { type: 'properties' | 'rooms' | 'leases' | 'utility-bills' | 'charges'; id: Uuid },
  file: File,
  kind?: string,
): Promise<StoredDocument> {
  const form = new FormData()
  if (kind) form.set('kind', kind)
  form.set('file', file)
  return client.request<StoredDocument>(`/${owner.type}/${owner.id}/documents`, {
    method: 'POST',
    formData: form,
  })
}

export function listDocuments(
  client: ApiClient,
  owner: { type: 'properties' | 'rooms' | 'leases' | 'utility-bills' | 'charges'; id: Uuid },
): Promise<Page<StoredDocument>> {
  return client.get<Page<StoredDocument>>(`/${owner.type}/${owner.id}/documents`)
}

/**
 * L'URL da cui si scarica un documento.
 *
 * Non è una `fetch`: la rotta risponde `307` verso un URL pre-firmato, e seguirlo
 * con `fetch` scaricherebbe i byte in memoria per poi doverli riconsegnare al
 * browser. Un `<a href>` lascia fare al browser ciò che sa fare.
 *
 * **Il link non porta l'`Authorization`**, quindi funziona solo su una rotta che
 * l'accetti: qui è il browser a seguirlo con il proprio contesto, e il `307`
 * arriva dall'API che ha già autenticato la richiesta. In pratica va aperto in
 * una scheda dove la sessione esiste.
 */
export function documentContentPath(id: Uuid): string {
  return `/api/v1/documents/${id}/content`
}

// ---------------------------------------------------------------------------
// Addebiti

export interface ChargeFilters {
  lease_id?: Uuid | undefined
  charge_type?: ChargeType | undefined
  /** Solo stati memorizzati: `overdue` è derivato e darebbe `422`. */
  status?: 'issued' | 'partially_paid' | 'paid' | 'void' | 'written_off' | undefined
  /** Lo **scadenzario**: i tre stati dell'insoluto in un filtro solo. */
  outstanding?: boolean | undefined
  after?: Uuid | undefined
}

export function listCharges(client: ApiClient, filters: ChargeFilters = {}): Promise<Page<Charge>> {
  return client.get<Page<Charge>>(
    `/charges${query({
      limit: LIMIT,
      lease_id: filters.lease_id,
      charge_type: filters.charge_type,
      status: filters.status,
      outstanding: filters.outstanding,
      after: filters.after,
    })}`,
  )
}

export interface IssueChargeInput {
  charge_type: ChargeType
  amount: string
  due_date: string
  description?: string
  period_start?: string
  period_end?: string
}

export function issueCharge(
  client: ApiClient,
  leaseId: Uuid,
  input: IssueChargeInput,
): Promise<Charge> {
  return client.post<Charge>(`/leases/${leaseId}/charges`, input)
}

export function voidCharge(client: ApiClient, id: Uuid): Promise<Charge> {
  return client.post<Charge>(`/charges/${id}/void`, {})
}

export function writeOffCharge(client: ApiClient, id: Uuid): Promise<Charge> {
  return client.post<Charge>(`/charges/${id}/write-off`, {})
}
