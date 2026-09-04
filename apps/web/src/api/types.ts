/**
 * I tipi sul filo, trascritti dai DTO dell'API.
 *
 * Scritti a mano e non generati dall'`openapi.json`: la generazione andrebbe
 * rieseguita e ricommittata a ogni modifica dell'API, e un generatore in mezzo
 * nasconde proprio le due cose che qui contano — che **gli importi sono stringhe**
 * e che alcuni campi possono mancare. Sono un paio di centinaia di righe, si
 * leggono, e quando l'API cambia il compilatore dice dove.
 *
 * **Ogni importo è `string`.** Non è pigrizia di tipizzazione: è il contratto.
 * Vedi `@/lib/decimal`.
 */

export type Uuid = string
/** Data ISO, `2026-08-31`. */
export type IsoDate = string
/** Istante ISO con fuso, `2026-08-31T10:00:00Z`. */
export type IsoDateTime = string
/** Stringa decimale, `"184.20"`. Mai un `number`. */
export type Amount = string

export interface Page<T> {
  items: T[]
  /** Assente a fine risultati. Da rimandare come `after`. */
  next_cursor?: Uuid
}

export interface Items<T> {
  items: T[]
}

// ---------------------------------------------------------------------------
// Dashboard
// ---------------------------------------------------------------------------

export interface Dashboard {
  organization_id: Uuid
  properties_count: number
  active_leases: number
  monthly_rent_expected: Amount
  total_outstanding: Amount
  overdue_charges: number
  open_tickets: number
  leases_expiring_90d: number
  bills_pending_review: number
  /**
   * Quanto si è rinunciato a incassare: la somma dei **residui** degli addebiti
   * stornati.
   *
   * Non compensa `total_outstanding` e non gli si sottrae: quello dice quanto è
   * ancora dovuto, questo quanto non lo sarà mai. Un addebito **annullato** non
   * compare qui, perché non doveva essere chiesto e quindi non è una perdita.
   */
  written_off_total: Amount
}

// ---------------------------------------------------------------------------
// Immobili e stanze
// ---------------------------------------------------------------------------

export type PropertyType =
  | 'apartment'
  | 'house'
  | 'studio'
  | 'villa'
  | 'office'
  | 'retail'
  | 'garage'
  | 'land'
  | 'other'

export type PropertyStatus =
  | 'draft'
  | 'available'
  | 'partially_rented'
  | 'rented'
  | 'maintenance'
  | 'archived'

export type EnergyClass = 'A4' | 'A3' | 'A2' | 'A1' | 'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G'

export interface Address {
  street: string
  extra?: string | null
  city: string
  province?: string | null
  postal_code?: string | null
  country?: string | null
}

export interface Property {
  id: Uuid
  organization_id: Uuid
  owner_id: Uuid
  label: string
  code?: string
  property_type: PropertyType
  status: PropertyStatus
  is_coliving: boolean
  address: Address
  size_sqm: Amount | null
  rooms_count: number | null
  bathrooms_count: number | null
  floor: number | null
  has_elevator: boolean | null
  energy_class: EnergyClass | null
  notes?: string
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

export interface PropertyResident {
  /** Assente finché l'invitato non ha accettato: ha un nome e nessun id. */
  user_id?: Uuid
  full_name: string
  /** Assente sui contratti sull'immobile intero. */
  room_name?: string
}

export type RoomStatus = 'available' | 'occupied' | 'unavailable'

export interface Room {
  id: Uuid
  property_id: Uuid
  organization_id: Uuid
  name: string
  status: RoomStatus
  size_sqm: Amount | null
  floor: number | null
  has_private_bathroom: boolean
  is_furnished: boolean | null
  monthly_rent?: Amount
  currency: string
  utility_share_percent: Amount | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Contratti
// ---------------------------------------------------------------------------

export type LeaseType = 'whole_property' | 'room'

export type LeaseStatus =
  | 'draft'
  | 'pending_signature'
  | 'active'
  | 'expiring'
  | 'expired'
  | 'terminated'
  | 'cancelled'

export type RentFrequency = 'monthly' | 'quarterly' | 'semiannual' | 'annual'
export type LeasePartyRole = 'tenant' | 'cotenant' | 'guarantor'

export interface Lease {
  id: Uuid
  organization_id: Uuid
  property_id: Uuid
  room_id?: Uuid
  lease_type: LeaseType
  /** Lo stato **memorizzato**: cambia solo su azioni esplicite. */
  status: LeaseStatus
  /**
   * Lo stato **alla data odierna**: può valere `expiring` o `expired` mentre
   * `status` resta `active`. Il client non ricalcola le date, e questi due valori
   * **non si possono filtrare** — `?status=expiring` risponde `422`.
   */
  effective_status: LeaseStatus
  start_date: IsoDate
  /** Esclusiva: il contratto copre `[start_date, end_date)`. */
  end_date?: IsoDate
  notice_months: number | null
  auto_renew: boolean
  renewal_months: number | null
  rent_amount: Amount
  currency: string
  rent_frequency: RentFrequency
  rent_due_day: number
  deposit_amount: Amount
  utilities_included: boolean
  istat_indexation: boolean
  signed_at: IsoDateTime | null
  terminated_at: IsoDateTime | null
  termination_reason?: string
  notes?: string
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

export interface LeaseParty {
  id: Uuid
  lease_id: Uuid
  /** Assente finché l'invitato non ha accettato. */
  user_id?: Uuid
  role: LeasePartyRole
  full_name: string
  email?: string
  share_percent: Amount
  is_primary: boolean
  joined_at: IsoDate | null
  left_at: IsoDate | null
}

export type LeaseDetail = Lease & { parties: LeaseParty[] }

// ---------------------------------------------------------------------------
// Bollette, OCR e addebiti
// ---------------------------------------------------------------------------

export type UtilityType =
  | 'electricity'
  | 'gas'
  | 'water'
  | 'heating'
  | 'waste'
  | 'internet'
  | 'condo_fees'
  | 'other'

export type BillStatus =
  | 'draft'
  | 'pending_review'
  | 'validated'
  | 'allocated'
  | 'paid'
  | 'disputed'
  | 'void'

export interface Consumption {
  /** Fino a **quattro** decimali, mentre gli importi ne hanno due. */
  value: Amount
  unit: string
}

export interface UtilityBill {
  id: Uuid
  organization_id: Uuid
  property_id: Uuid
  room_id?: Uuid
  /** Da valorizzare perché un inquilino possa vedere la bolletta. */
  lease_id?: Uuid
  utility_type: UtilityType
  status: BillStatus
  supplier_name?: string
  supplier_vat?: string
  invoice_number?: string
  issue_date: IsoDate | null
  due_date: IsoDate | null
  period_start: IsoDate | null
  period_end: IsoDate | null
  total_amount: Amount
  taxable_amount: Amount | null
  vat_amount: Amount | null
  currency: string
  consumption?: Consumption
  validated_by: Uuid | null
  validated_at: IsoDateTime | null
  notes?: string
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

export type AiJobStatus = 'queued' | 'running' | 'succeeded' | 'failed' | 'cancelled'

export type OcrErrorCode =
  | 'unsupported_document'
  | 'provider_unavailable'
  | 'document_archived'
  | 'document_unavailable'
  | 'subject_not_processable'

export interface AiJob {
  id: Uuid
  status: AiJobStatus
  task: string
  provider_name: string
  model?: string
  attempts: number
  /** **Non** è un codice di errore dell'API: un job fallito è una lettura andata male. */
  error_code?: OcrErrorCode
  error_message?: string
  started_at?: IsoDateTime
  finished_at?: IsoDateTime
  created_at: IsoDateTime
}

/** Il nome della colonna della bolletta a cui la proposta si riferisce. */
export type ExtractedFieldName =
  | 'supplier_name'
  | 'supplier_vat'
  | 'invoice_number'
  | 'issue_date'
  | 'due_date'
  | 'period_start'
  | 'period_end'
  | 'total_amount'
  | 'taxable_amount'
  | 'vat_amount'
  | 'consumption'

export interface ExtractedField {
  field: string
  /** Come leggere `value`: `text`, `date`, `amount` o `quantity`. */
  type: 'text' | 'date' | 'amount' | 'quantity'
  /** **Sempre una stringa**, anche per gli importi e per le quantità. */
  value: string
  unit?: string
  /** Fra `"0"` e `"1"`, quattro decimali. Bassa non vuol dire sbagliata. */
  confidence: Amount
}

export interface BillOcrStatus {
  utility_bill_id: Uuid
  status: BillStatus
  /** `null` finché nessuno ha caricato una scansione. */
  job?: AiJob
  /** Il **minimo** fra le confidenze dei campi, non la media. */
  confidence?: Amount
  /**
   * `undefined` finché nessuna lettura è riuscita; un array **vuoto** significa
   * che la macchina ha letto e non ha trovato niente di interpretabile, che è un
   * esito diverso e va mostrato come tale.
   */
  extracted?: ExtractedField[]
}

export type ChargeType =
  | 'rent'
  | 'deposit'
  | 'utility_recharge'
  | 'late_fee'
  | 'maintenance'
  | 'adjustment'
  | 'other'

export type ChargeStatus =
  | 'draft'
  | 'issued'
  | 'partially_paid'
  | 'paid'
  | 'overdue'
  | 'void'
  | 'written_off'

export interface Charge {
  id: Uuid
  organization_id: Uuid
  lease_id: Uuid
  charge_type: ChargeType
  status: ChargeStatus
  /**
   * `overdue` quando la scadenza è passata e il residuo è positivo. **Non si
   * filtra**: `?status=overdue` risponde `422`, si usa `?outstanding=true`.
   */
  effective_status: ChargeStatus
  description?: string
  period_start: IsoDate | null
  period_end: IsoDate | null
  amount: Amount
  paid_amount: Amount
  /** `"0.00"` su un addebito annullato o stornato; può essere **negativo**. */
  outstanding_amount: Amount
  currency: string
  due_date: IsoDate
  issued_at: IsoDateTime | null
  utility_bill_id?: Uuid
  notes?: string
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

export interface RechargeResult {
  charge: Charge
  utility_bill: UtilityBill
}

// ---------------------------------------------------------------------------
// Pagamenti
// ---------------------------------------------------------------------------

export type PaymentMethod =
  | 'bank_transfer'
  | 'sepa_direct_debit'
  | 'card'
  | 'cash'
  | 'check'
  | 'other'

/**
 * Lo stato di un pagamento.
 *
 * **Quattro valori su sette l'API non li scrive mai**: `authorized` e `failed`
 * descrivono ciò che accade presso un provider, e con `manual` un provider non
 * c'è; `refunded` e `partially_refunded` appartengono ai rimborsi, che sono fuori
 * perimetro. Compaiono qui perché una riga letta potrebbe portarli, e un valore
 * sconosciuto deve rompere il typecheck invece di ricadere su un default.
 *
 * A differenza degli addebiti e dei contratti **nessuno di questi è derivato**:
 * si filtrano tutti, e nessuno risponde `422`.
 */
export type PaymentStatus =
  | 'pending'
  | 'authorized'
  | 'succeeded'
  | 'failed'
  | 'cancelled'
  | 'refunded'
  | 'partially_refunded'

export interface Payment {
  id: Uuid
  organization_id: Uuid
  /** L'addebito che il pagamento salda: **sempre un addebito**, in questa fetta. */
  charge_id: Uuid
  status: PaymentStatus
  amount: Amount
  currency: string
  method: PaymentMethod
  /**
   * Chi ha pagato, se l'incasso è stato **attribuito**.
   *
   * Assente non vuol dire «anonimo»: vuol dire che la riga non è visibile a chi
   * ha pagato, perché `payments_select` la mostra a `payer_user_id`. Il totale
   * incassato gli resta comunque visibile nel saldo dell'addebito.
   */
  payer_user_id?: Uuid
  reference?: string
  /**
   * Quando il denaro si è mosso. **Nullo su una dichiarazione rifiutata**: un
   * pagamento che non è avvenuto non ha una data in cui è avvenuto.
   */
  paid_at: IsoDateTime | null
  /** Quando una persona ha riconosciuto il movimento. */
  reconciled_at: IsoDateTime | null
  /** Sempre `"0.00"`: i rimborsi sono fuori perimetro. */
  refunded_amount: Amount
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

// ---------------------------------------------------------------------------
// Documenti e inviti
// ---------------------------------------------------------------------------

export type DocumentKind =
  | 'lease_contract'
  | 'id_document'
  | 'utility_bill'
  | 'invoice'
  | 'receipt'
  | 'photo'
  | 'inspection_report'
  | 'energy_certificate'
  | 'cadastral_extract'
  | 'insurance_policy'
  | 'other'

export interface StoredDocument {
  id: Uuid
  organization_id: Uuid
  kind: DocumentKind
  owner_type: string
  owner_id: Uuid
  original_filename: string
  mime_type: string
  size_bytes: number
  checksum_sha256: string
  is_encrypted: boolean
  uploaded_by: Uuid | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

export interface Invitation {
  id: Uuid
  organization_id: Uuid
  lease_id?: Uuid
  email: string
  role: string
  expires_at: IsoDateTime
  /**
   * Il token in chiaro, presente **solo** in questa risposta.
   *
   * È una credenziale: non va nei log e non va conservata. Il client la mostra
   * una volta perché il proprietario la consegni, e la dimentica.
   */
  accept_token: string
}
