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

/** Il ruolo dentro un'organizzazione, come lo manda l'API. */
export type OrgRole = 'owner' | 'manager' | 'tenant'

export interface Page<T> {
  items: T[]
  /** Assente a fine risultati. Da rimandare come `after`. */
  next_cursor?: Uuid
}

export interface Items<T> {
  items: T[]
}

// ---------------------------------------------------------------------------
// Organizzazioni
// ---------------------------------------------------------------------------

/**
 * Un'organizzazione di cui si è membri, **con il nome**.
 *
 * È l'unica risposta dell'API che porti il nome di un'organizzazione a chi non ne
 * è staff, ed è la ragione per cui la rotta esiste: senza, un selettore può
 * mostrare soltanto l'identificatore. Per chi ne ha una è indistinguibile da un
 * elenco vero; per un inquilino sono due UUID fra cui indovinare.
 *
 * `role` è il ruolo **in questa** organizzazione: la stessa persona è proprietario
 * della propria e inquilino in quella di chi le affitta casa.
 */
export interface Organization {
  id: Uuid
  name: string
  slug: string
  role: OrgRole
}

/**
 * Il proprio account, letto da `GET /api/v1/me`.
 *
 * **`email` si mostra e non si modifica**: è la credenziale d'accesso, non un dato
 * anagrafico, e cambiarla richiede la verifica del nuovo indirizzo, che è fuori
 * perimetro. Il ruolo di piattaforma non c'è, e non è una dimenticanza: non è
 * affare di chi guarda il proprio profilo.
 */
export interface Account {
  id: Uuid
  email: string
  /** Assente finché la verifica dell'email non esiste: il campo c'è già perché il
   *  client non cambi forma il giorno in cui arriverà. */
  email_verified_at?: IsoDateTime
  full_name: string
  /** Assente quando non c'è un numero, non `null`: l'API lo omette. */
  phone?: string
  locale: string
}

/**
 * Che cosa resta dopo essere usciti da un'organizzazione.
 *
 * Chi esce dall'unica in cui abitava resta con la propria, personale e vuota: il
 * client lo dice **prima**, non lo lascia scoprire con un riepilogo vuoto.
 */
export interface LeftOrganization {
  organization_id: Uuid
  role: OrgRole
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
  /** Lo stato **memorizzato**: un giudizio di chi amministra. */
  status: PropertyStatus
  /**
   * Lo stato **alla data odierna**, che tiene conto dei contratti in essere: può
   * valere `rented` o `partially_rented` mentre `status` resta `draft`.
   *
   * Non si filtra — `?stato=rented` risponde `422` — e quando diverge da `status`
   * si mostrano **entrambi**: un immobile «in manutenzione» che risulta affittato
   * è un'anomalia da vedere.
   */
  effective_status: PropertyStatus
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
  /** Memorizzato: `available`, o `unavailable` se dichiarata fuori uso. */
  status: RoomStatus
  /**
   * Derivato dai contratti in essere: `occupied` quando qualcuno ci abita.
   *
   * Non lo scrive nessuno, e non potrebbe restare vero: l'occupazione dipende dal
   * calendario.
   */
  effective_status: RoomStatus
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
  /**
   * Come si chiama l'immobile, per distinguere una riga dall'altra.
   *
   * **Assente** sulle risposte a una singola operazione — attivazione, disdetta,
   * annullamento — dove chi chiama ha già il contratto sotto gli occhi, e quando
   * l'immobile è archiviato.
   */
  property_label?: string
  /** Assente sui contratti sull'immobile intero. */
  room_name?: string
  /**
   * Il nome dell'**intestatario**, cioè della parte marcata `is_primary`.
   *
   * Serve dove *dove* non basta: due contratti sulla stessa stanza in due anni
   * diversi hanno lo stesso immobile e la stessa stanza, e a distinguerli è chi ci
   * abita. **Assente** su una bozza a cui non è ancora stato aggiunto nessuno.
   */
  primary_tenant?: string
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
  phone?: string
  share_percent: Amount
  is_primary: boolean
  joined_at: IsoDate | null
  left_at: IsoDate | null
}

export type LeaseDetail = Lease & { parties: LeaseParty[] }

// ---------------------------------------------------------------------------
// Segnalazioni
// ---------------------------------------------------------------------------

export type TicketCategory =
  | 'plumbing'
  | 'electrical'
  | 'heating_cooling'
  | 'appliance'
  | 'structural'
  | 'pest'
  | 'cleaning'
  | 'connectivity'
  | 'security'
  | 'other'

export type TicketPriority = 'low' | 'medium' | 'high' | 'urgent'

/**
 * Lo stato di una segnalazione.
 *
 * **`resolved` c'è e l'API non lo scrive mai**: il flusso con conferma
 * dell'inquilino è stato scartato, perché senza scheduler un ticket risolto e mai
 * confermato resterebbe appeso per sempre. Compare qui perché una riga letta
 * potrebbe portarlo, e un valore sconosciuto deve rompere il typecheck invece di
 * ricadere su un default — ma un menù di filtri costruito da questa unione
 * offrirebbe uno stato che non esiste. I filtri si scrivono a mano.
 */
export type TicketStatus =
  | 'open'
  | 'triaged'
  | 'assigned'
  | 'in_progress'
  | 'waiting_tenant'
  | 'waiting_owner'
  | 'resolved'
  | 'closed'
  | 'cancelled'

/** Chi ha agito. `system` e `ai` sono del triage automatico, che non c'è ancora. */
export type TicketActor = 'tenant' | 'owner' | 'manager' | 'system' | 'ai'

/** Su chi si sta aspettando. È il corpo di `/wait`, non uno stato a sé. */
export type WaitingOn = 'tenant' | 'owner'

export interface Ticket {
  id: Uuid
  organization_id: Uuid
  /**
   * Riferimento leggibile assegnato dal **database**, `TK-000042`.
   *
   * È **per organizzazione**: due organizzazioni diverse hanno legittimamente lo
   * stesso riferimento, quindi non è un identificatore globale e non si indicizza
   * per lui. Per raggiungere una segnalazione serve `id`.
   */
  reference: string | null
  property_id: Uuid
  room_id?: Uuid
  /** Senza, non la vede nessun inquilino: `can_access_ticket` passa da qui. */
  lease_id?: Uuid
  title: string
  description: string
  category: TicketCategory
  priority: TicketPriority
  status: TicketStatus
  /**
   * `true` se la leggono anche i **residenti dell'immobile**, oltre a chi la vede
   * per contratto — e con essa il thread e le fotografie.
   *
   * Il confine è `app.can_access_ticket` e non questo campo: qui viaggia perché la
   * casella si mostra e si corregge. Non concede nessuna scrittura — un residente
   * non riapre, non annulla, non corregge.
   */
  visible_to_residents: boolean
  /**
   * Come si chiama l'immobile a cui la segnalazione si riferisce.
   *
   * **Assente** quando la riga dell'immobile è archiviata: la segnalazione resta,
   * il nome no. Si mette l'etichetta neutra dal catalogo, mai l'identificatore.
   */
  property_label?: string
  /** La stanza, quando la segnalazione ne riguarda una. */
  room_name?: string
  reported_by: Uuid | null
  reported_by_kind: TicketActor
  /**
   * Come si chiama chi ha segnalato.
   *
   * **Assente non è un errore**: la policy `users_select` non concede a chiunque la
   * riga di chiunque, e allora si mette un'etichetta neutra dal catalogo — mai
   * l'identificatore. Va mostrato **accanto** a `reported_by_kind`: il ruolo dice
   * da che parte del rapporto sta la persona, il nome dice chi è.
   */
  reported_by_name?: string
  assigned_to: Uuid | null
  /** Come si chiama chi ha in carico. Stessa regola sull'assenza. */
  assigned_to_name?: string
  assigned_at: IsoDateTime | null
  /**
   * **`null` per chi non è staff, anche quando un valore c'è.**
   *
   * È la cifra di una trattativa con un fornitore e non riguarda chi abita la
   * casa. Non è un confine di sicurezza — la Row Level Security lascia comunque
   * leggere la riga — ma una regola del contratto dell'API, decisa in
   * `authorization::discloses_estimated_cost`. Mostrare `0,00 €` sarebbe una
   * bugia: il trattino è la verità.
   */
  estimated_cost: Amount | null
  /** Lo vede **chiunque veda la segnalazione**: chi ha subìto il guasto sa quanto
   *  è costato ripararlo. */
  actual_cost: Amount | null
  currency: string
  closed_at: IsoDateTime | null
  /** Come è andata a finire. Una riapertura la **cancella**. */
  resolution_notes?: string
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

/**
 * Una transizione dello storico.
 *
 * Lo scrive un **trigger** del database, non l'API: la prima riga è sempre
 * `null → open` e la produce l'inserimento. Per questo `changed_by` può essere
 * `null` — fuori da un contesto di richiesta non c'è un utente corrente.
 */
export interface TicketStatusChange {
  from_status: TicketStatus | null
  to_status: TicketStatus
  changed_by: Uuid | null
  reason?: string
  changed_at: IsoDateTime
}

/** Il dettaglio: l'aggregato più lo storico. **I messaggi no**, hanno il loro
 *  cursore — una conversazione non ha un tetto. */
export type TicketDetail = Ticket & { history: TicketStatusChange[] }

export interface TicketMessage {
  id: Uuid
  ticket_id: Uuid
  author_id: Uuid | null
  author_kind: TicketActor
  /**
   * Come si chiama chi ha scritto.
   *
   * **Il ruolo da solo non basta in un co-living**: due coinquilini sono entrambi
   * `tenant`, e «me lo dice il proprietario o un coinquilino?» ha già risposta
   * mentre «quale coinquilino?» non ne ha. I due si mostrano insieme.
   */
  author_name?: string
  body: string
  /**
   * Nota interna, visibile al solo staff.
   *
   * **A un inquilino non arrivano affatto**: le filtra `ticket_messages_select`,
   * non questo campo. Il client non deve nasconderle — non le riceve.
   */
  is_internal: boolean
  created_at: IsoDateTime
}

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
  /** Quando è stata dichiarata pagata al fornitore. */
  paid_at?: IsoDateTime | null
  /**
   * Come si chiama l'immobile del **contratto imputato**, non quello della
   * bolletta: serve a nominare il collegamento, e il nome di un contratto è la
   * casa più la stanza.
   *
   * **Assente** per due ragioni diverse — nessun contratto imputato, o immobile
   * archiviato — e in entrambi i casi si mette l'etichetta neutra.
   */
  property_label?: string
  /** La stanza del contratto imputato, quando ne nomina una. */
  room_name?: string
  /**
   * Quanto è **già stato riaddebitato**. Il residuo è `total_amount` meno questo,
   * e si calcola con `@/lib/decimal`, mai con un `-`.
   *
   * **`null` per chi non è staff**: la somma si legge sotto RLS e a un inquilino
   * tornerebbe parziale, cioè un numero sbagliato che sembra un totale.
   */
  recharged_amount?: Amount | null
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
  /**
   * Come si chiama l'immobile del contratto a cui l'addebito è emesso.
   *
   * **Assente** quando l'immobile è archiviato: il contratto resta, il nome no.
   */
  property_label?: string
  /** La stanza, quando il contratto ne nomina una. */
  room_name?: string
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

// ---------------------------------------------------------------------------
// Spese co-living
// ---------------------------------------------------------------------------

/**
 * Che cosa il gruppo tiene insieme. Il co-living è il caso per cui il modulo
 * esiste; gli altri due sono lo stesso conto con un nome diverso.
 */
export type ExpenseGroupKind = 'coliving' | 'household' | 'property'

/**
 * Come una spesa si divide.
 *
 * **Tre valori su cinque l'API li rifiuta con `422`**: `percentage` ed `exact`
 * chiedono un input per membro che non esiste, `by_usage` letture di contatori che
 * non esistono. Compaiono qui perché una riga letta potrebbe portarli — un enum
 * sconosciuto deve rompere il typecheck invece di ricadere su un default — ma un
 * menù costruito da questa unione offrirebbe tre metodi che danno errore. Il menù
 * si scrive a mano con due voci, come i filtri di stato delle segnalazioni.
 */
export type SplitMethod = 'equal' | 'weighted' | 'percentage' | 'exact' | 'by_usage'

export type ExpenseCategory =
  | 'utilities'
  | 'groceries'
  | 'cleaning'
  | 'connectivity'
  | 'maintenance'
  | 'rent'
  | 'furniture'
  | 'other'

/**
 * Lo stato di un regolamento.
 *
 * **Tutti e quattro l'API li scrive**, a differenza degli stati di un pagamento:
 * `pending` ha **tre** uscite, perché oltre alla conferma e al rifiuto di chi
 * riceve c'è il ritiro di chi ha dichiarato — `settlements_update` nomina anche
 * `from_user_id`, mentre `payments_update` è staff soltanto.
 */
export type SettlementStatus = 'pending' | 'confirmed' | 'rejected' | 'cancelled'

/**
 * Una persona dentro un gruppo.
 *
 * **`display_name` è l'unico nome che questo modulo conosca**, ed è facoltativo:
 * lo scrive chi iscrive la persona, o resta assente per sempre. Saldi, quote,
 * spese e regolamenti identificano le persone con il solo `user_id`, e per chi ha
 * lasciato la casa non c'è nessun altro endpoint da cui recuperarlo — l'elenco dei
 * residenti copre chi abita **ora**. Vedi `@/features/expenses/names`.
 */
export interface ExpenseGroupMember {
  user_id: Uuid
  room_id?: Uuid
  display_name?: string
  /** Il peso della ripartizione `weighted`: **tre** decimali, non due. */
  weight: Amount
  joined_at: IsoDateTime
  /**
   * Quando è uscito. **La riga resta**: le quote a suo nome restano, e finché il
   * suo netto non è zero compare ancora fra i saldi del gruppo — ma il gruppo a
   * lui è invisibile, perché `can_access_expense_group` esige `left_at IS NULL`.
   */
  left_at: IsoDateTime | null
}

export interface ExpenseGroup {
  id: Uuid
  organization_id: Uuid
  /** **Facoltativo**: un gruppo può non avere un immobile, e allora non c'è un
   *  elenco di residenti da cui pescare i membri. */
  property_id?: Uuid
  kind: ExpenseGroupKind
  name: string
  currency: string
  /** Una **consuetudine**, non una regola: ogni spesa può dichiararne un'altra. */
  default_split_method: SplitMethod
  created_by?: Uuid
  /** Il conto è chiuso: niente più spese né regolamenti. **Non** è una
   *  cancellazione — saldi e spese restano leggibili. */
  is_archived: boolean
  /**
   * La composizione, chi è uscito compreso.
   *
   * **Assente nell'elenco, presente nel dettaglio**: `GET /expense-groups`
   * risponde senza, `GET /expense-groups/{id}` con. Un elenco che provasse a
   * mostrare chi c'è dentro mostrerebbe `undefined`.
   */
  members?: ExpenseGroupMember[]
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

export interface ExpenseShare {
  user_id: Uuid
  share_amount: Amount
  /** **Nullo su una divisione in parti uguali**, dove il peso non ha partecipato
   *  al calcolo: mostrarlo come `1` sarebbe inventare un dato. */
  share_weight?: Amount
}

export interface Expense {
  id: Uuid
  organization_id: Uuid
  group_id: Uuid
  /** Chi ha anticipato: è il verso del credito. Sempre un **membro attivo**. */
  paid_by_user_id: Uuid
  /**
   * Chi l'ha registrata, che può non essere chi ha anticipato. Serve a offrire
   * l'annullamento a chi può farlo — chi l'ha registrata, chi l'ha anticipata, o
   * lo staff — invece di un bottone che risponde `403 expense.not_yours`. Assente
   * su una riga scritta fuori da un contesto di richiesta.
   */
  created_by?: Uuid
  category: ExpenseCategory
  description: string
  amount: Amount
  currency: string
  /** Una **data**, non un istante: nessun fuso da cui sbagliare di un giorno. */
  expense_date: IsoDate
  split_method: SplitMethod
  notes?: string
  /**
   * Le quote. **Assenti nell'elenco, presenti nel dettaglio.**
   *
   * La loro somma è **esattamente** `amount`: lo garantisce il trigger
   * `expense_shares_balanced`. Il client le mostra e non le ricalcola.
   */
  shares?: ExpenseShare[]
  created_at: IsoDateTime
  updated_at: IsoDateTime
}

/**
 * Il saldo di una persona nel gruppo.
 *
 * `net_balance` positivo è **in credito**, negativo in debito, e **la somma dei
 * netti di un gruppo fa zero** — l'invariante che rende questi numeri leggibili.
 * Va verificata con `@/lib/decimal`: sommarli con l'aritmetica nativa produce uno
 * zero che a volte non è zero.
 */
export interface GroupBalance {
  user_id: Uuid
  total_paid: Amount
  total_owed: Amount
  total_settled_out: Amount
  total_settled_in: Amount
  net_balance: Amount
}

/**
 * Un trasferimento dichiarato fra due membri.
 *
 * Nasce `pending` e **non muove i saldi**: solo `confirmed` conta, e lo decide la
 * vista — `app.v_expense_group_balances` somma `WHERE st.status = 'confirmed'`.
 */
export interface Settlement {
  id: Uuid
  organization_id: Uuid
  group_id: Uuid
  /** Chi paga, cioè chi ha dichiarato. È **sempre chi chiama**: il corpo della
   *  dichiarazione non ha un `from_user_id`. */
  from_user_id: Uuid
  /** Chi riceve, cioè chi deve confermare. */
  to_user_id: Uuid
  amount: Amount
  currency: string
  status: SettlementStatus
  method: PaymentMethod
  note?: string
  /** Quando il denaro si è mosso. **Nullo su un rifiuto o un ritiro**, e la
   *  conferma **non lo riscrive**: è il fatto dichiarato, non il momento in cui
   *  qualcuno l'ha guardato. */
  settled_at: IsoDateTime | null
  confirmed_at: IsoDateTime | null
  confirmed_by: Uuid | null
  created_at: IsoDateTime
  updated_at: IsoDateTime
}
