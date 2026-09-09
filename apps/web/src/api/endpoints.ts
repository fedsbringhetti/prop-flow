import type { ApiClient } from './client'
import type {
  BillOcrStatus,
  BillStatus,
  Charge,
  ChargeType,
  Dashboard,
  Expense,
  ExpenseCategory,
  ExpenseGroup,
  ExpenseGroupKind,
  ExpenseGroupMember,
  GroupBalance,
  Invitation,
  Items,
  Lease,
  LeaseDetail,
  LeaseParty,
  LeasePartyRole,
  Organization,
  Page,
  Payment,
  PaymentMethod,
  PaymentStatus,
  Property,
  PropertyResident,
  PropertyStatus,
  PropertyType,
  RechargeResult,
  Room,
  Settlement,
  SettlementStatus,
  SplitMethod,
  StoredDocument,
  Ticket,
  TicketCategory,
  TicketDetail,
  TicketMessage,
  TicketPriority,
  TicketStatus,
  WaitingOn,
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
// Organizzazioni

/**
 * Le organizzazioni di chi chiama, **con il nome**.
 *
 * È la sola rotta dell'API che risponda anche senza un'organizzazione attiva, e
 * non è una deroga: «quali organizzazioni sono mie» non appartiene a nessuna di
 * esse. `ApiClient` manda comunque `X-Org-Id` quando ce n'è una — la risposta non
 * cambia — perché l'eccezione sta nell'endpoint e non nel client, e un secondo
 * modo di non mandare l'header sarebbe un secondo modo di sbagliarlo.
 *
 * **Nessun cursore**: le organizzazioni di una persona sono la propria più quelle
 * di chi le affitta casa.
 */
export function getOrganizations(client: ApiClient): Promise<Items<Organization>> {
  return client.get<Items<Organization>>('/organizations')
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
  floor?: number
  has_private_bathroom?: boolean
  is_furnished?: boolean
  /** 0–100 come stringa decimale. Assente significa «in parti uguali». */
  utility_share_percent?: string
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

/**
 * Corregge una parte, con la semantica `PATCH`: assente = invariato, `null` =
 * azzerato.
 *
 * `email` la rifiuta l'API con `409 lease.party_email_locked` se la parte è già
 * collegata a un utente — è ciò a cui l'invito è intestato.
 */
export function correctLeaseParty(
  client: ApiClient,
  partyId: Uuid,
  patch: Record<string, unknown>,
): Promise<LeaseParty> {
  return client.patch<LeaseParty>(`/lease-parties/${partyId}`, patch)
}

/** Toglie una parte inserita per sbaglio: `204`. Rifiutata se ha un utente. */
export function removeLeaseParty(client: ApiClient, partyId: Uuid): Promise<void> {
  return client.delete(`/lease-parties/${partyId}`)
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

/**
 * Annulla un contratto mai entrato in vigore.
 *
 * **Senza corpo**, a differenza della disdetta: non c'è stato alcun rapporto,
 * quindi non c'è una ragione da registrare accanto a una data che non esiste.
 */
export function cancelLease(client: ApiClient, id: Uuid): Promise<Lease> {
  return client.post<Lease>(`/leases/${id}/cancel`, {})
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
// Segnalazioni

export interface OpenTicketInput {
  title: string
  description: string
  /** La stanza, se il guasto è lì. Deve appartenere all'immobile del percorso. */
  room_id?: Uuid
  /**
   * Il contratto a cui la segnalazione si riferisce.
   *
   * **Da valorizzare sempre che si possa**, anche quando è lo staff ad aprirla per
   * conto di qualcuno: `app.can_access_ticket` concede l'accesso a chi è parte del
   * contratto solo se `lease_id` non è nullo. Senza, la segnalazione non la vede
   * nessun inquilino — nemmeno quello che ci abita.
   */
  lease_id?: Uuid
  /** Assente vale `other`: classificare è il lavoro del triage. */
  category?: TicketCategory
  /** Assente vale `medium`. */
  priority?: TicketPriority
}

/**
 * Apre una segnalazione, **annidata sull'immobile**.
 *
 * Una segnalazione nasce sempre su un immobile; tutto il resto va per id. È
 * `ensure_member`, ed è **il** punto in cui l'inquilino prende la parola: fino a
 * qui poteva solo leggere.
 */
export function openTicket(
  client: ApiClient,
  propertyId: Uuid,
  input: OpenTicketInput,
): Promise<Ticket> {
  return client.post<Ticket>(`/properties/${propertyId}/tickets`, input)
}

export interface TicketFilters {
  property_id?: Uuid | undefined
  /**
   * Uno stato **memorizzato**. Nessuno è derivato, quindi nessuno dà `422` — ma
   * `resolved` l'API non lo scrive mai, e un menù costruito dall'unione dei
   * valori offrirebbe uno stato che non esiste.
   */
  status?: TicketStatus | undefined
  /**
   * La coda di lavorazione: i **sei** stati non terminali.
   *
   * **Non è zucchero per `status <> 'closed'`.** L'indice parziale
   * `tickets_open_ix` è definito su esattamente quei sei valori, e una
   * disuguaglianza non lo userebbe. `false` dà le chiuse, annullate e risolte.
   */
  open?: boolean | undefined
  after?: Uuid | undefined
}

export function listTickets(client: ApiClient, filters: TicketFilters = {}): Promise<Page<Ticket>> {
  return client.get<Page<Ticket>>(
    `/tickets${query({
      limit: LIMIT,
      property_id: filters.property_id,
      status: filters.status,
      open: filters.open,
      after: filters.after,
    })}`,
  )
}

/** Il dettaglio con lo **storico degli stati**. I messaggi stanno a parte. */
export function getTicket(client: ApiClient, id: Uuid): Promise<TicketDetail> {
  return client.get<TicketDetail>(`/tickets/${id}`)
}

/**
 * Il thread, **con il proprio cursore**.
 *
 * Separato dal dettaglio perché una conversazione non ha un tetto: incorporarla
 * significherebbe che il giorno in cui qualcuno scrive duecento messaggi il
 * dettaglio smette di rispondere.
 */
export function listTicketMessages(
  client: ApiClient,
  ticketId: Uuid,
  after?: Uuid,
): Promise<Page<TicketMessage>> {
  return client.get<Page<TicketMessage>>(`/tickets/${ticketId}/messages${query({ limit: LIMIT, after })}`)
}

/**
 * Scrive nel thread.
 *
 * `is_internal` lo può chiedere **solo lo staff**: a un inquilino l'API risponde
 * `403 org.staff_role_required` invece di scrivere la nota in chiaro nel thread.
 * Un messaggio poi **non si modifica e non si cancella**, nemmeno in modo soft:
 * `app.ticket_messages` ha due sole policy, `_select` e `_insert`.
 */
export function postTicketMessage(
  client: ApiClient,
  ticketId: Uuid,
  input: { body: string; is_internal?: boolean },
): Promise<TicketMessage> {
  return client.post<TicketMessage>(`/tickets/${ticketId}/messages`, input)
}

/**
 * Le **sette** transizioni, ognuna una sotto-risorsa.
 *
 * Non un `PATCH` sullo stato, e la ragione è che ognuna ha effetti propri —
 * `closed_at`, una riga di storico, un evento — che un `PATCH` produrrebbe di
 * soppiatto, rendendo invisibile che siano avvenuti. È la stessa scelta di
 * attivazione e disdetta dei contratti.
 *
 * **Una si chiama `/wait` e lo use case `SuspendTicket`**: cercare `/suspend` non
 * trova niente.
 */
export function triageTicket(
  client: ApiClient,
  id: Uuid,
  input: { category: TicketCategory; priority: TicketPriority },
): Promise<Ticket> {
  return client.post<Ticket>(`/tickets/${id}/triage`, input)
}

/**
 * Assegna la lavorazione.
 *
 * Il corpo vuole un `assignee_id`, e il client passa **sempre chi chiama**:
 * l'azione è «prendi in carico». Non è una semplificazione — un'organizzazione ha
 * esattamente uno staff, perché l'unico modo di diventarlo è `RegisterOwner` e
 * `InviteTenant` scrive `role: tenant` fisso: `OrgRole::Manager` è nell'enum e
 * nessuno lo scrive mai. Un menù delle persone sarebbe vuoto. Il giorno in cui
 * esisterà un invito che crea un gestore, quell'azione diventa un menù.
 */
export function assignTicket(client: ApiClient, id: Uuid, assigneeId: Uuid): Promise<Ticket> {
  return client.post<Ticket>(`/tickets/${id}/assign`, { assignee_id: assigneeId })
}

/** Avvia la lavorazione, **o la riprende** dopo una sospensione: un verbo solo. */
export function startTicket(client: ApiClient, id: Uuid): Promise<Ticket> {
  return client.post<Ticket>(`/tickets/${id}/start`, {})
}

/**
 * Sospende dichiarando su chi si aspetta. **Il percorso è `/wait`.**
 *
 * `waiting_tenant` e `waiting_owner` non sono una fase a sé ma una sospensione
 * della lavorazione: si entra da qui e si esce con `/start`.
 */
export function waitTicket(client: ApiClient, id: Uuid, waitingOn: WaitingOn): Promise<Ticket> {
  return client.post<Ticket>(`/tickets/${id}/wait`, { waiting_on: waitingOn })
}

/** Chiude. `resolution_notes` è facoltativo, ma il corpo va inviato. */
export function closeTicket(client: ApiClient, id: Uuid, resolutionNotes?: string): Promise<Ticket> {
  return client.post<Ticket>(
    `/tickets/${id}/close`,
    resolutionNotes ? { resolution_notes: resolutionNotes } : {},
  )
}

/**
 * Riapre: torna a `open` e **azzera `closed_at` e `resolution_notes`**.
 *
 * Spetta a **chi ha segnalato**, oltre che allo staff, e a un coinquilino l'API
 * risponde `403 ticket.not_yours` — non `org.staff_role_required`: non gli manca
 * un ruolo, gli manca l'essere stato lui a segnalare, e i due rimedi sono diversi.
 */
export function reopenTicket(client: ApiClient, id: Uuid): Promise<Ticket> {
  return client.post<Ticket>(`/tickets/${id}/reopen`, {})
}

/**
 * Annulla.
 *
 * Lo staff sempre; chi ha segnalato **solo finché è `open`**, cioè finché nessuno
 * ci ha messo mano — e da lì in poi l'API gli risponde `org.staff_role_required`,
 * che qui è il codice vero: la segnalazione è sua, ma annullarne una già in
 * lavorazione butterebbe via il lavoro di qualcun altro.
 */
export function cancelTicket(client: ApiClient, id: Uuid): Promise<Ticket> {
  return client.post<Ticket>(`/tickets/${id}/cancel`, {})
}

/**
 * L'**unico** `PATCH` del modulo, e sono solo i costi.
 *
 * Titolo e descrizione sono il testo con cui una persona ha descritto un guasto in
 * un momento preciso, e il thread esiste per correggere il tiro; categoria e
 * priorità si impostano con il triage, perché sono ciò che `triaged` significa;
 * immobile, stanza e contratto non sono più scrivibili nemmeno dal ruolo
 * applicativo (ADR-026). Vale la semantica generale del `PATCH`: assente =
 * invariato, `null` = azzerato.
 */
export function updateTicketCosts(
  client: ApiClient,
  id: Uuid,
  patch: Record<string, unknown>,
): Promise<Ticket> {
  return client.patch<Ticket>(`/tickets/${id}`, patch)
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
 * Gli owner a cui si può allegare un documento.
 *
 * Sono i **sei** che l'API espone, e `tickets` c'era nella rotta e non in questa
 * unione: la segnalazione con la foto del guasto non compilava, e il typecheck la
 * rifiutava prima ancora di poterla provare. I nomi sono quelli dei percorsi —
 * plurali, con `utility-bills` col trattino — e non quelli dell'enum
 * `attachment_owner_type`, che è singolare: qui si scrive un URL.
 */
export type DocumentOwner = {
  type: 'properties' | 'rooms' | 'leases' | 'tickets' | 'utility-bills' | 'charges'
  id: Uuid
}

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
  owner: DocumentOwner,
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
  owner: DocumentOwner,
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

/**
 * Archivia un allegato. Cancellazione **soft**, come tutto il resto.
 *
 * Il codice che l'API restituisce a chi non può è `org.staff_role_required`, ed è
 * **vero solo a metà**: la policy `documents_update` ammette lo staff *oppure* chi
 * ha caricato il documento. Chi mostra quel messaggio non deve quindi promettere
 * che serva un ruolo — all'uploader quel ruolo non serve. È un debito registrato,
 * e qui si evita almeno di ripeterlo all'utente.
 */
export function archiveDocument(client: ApiClient, id: Uuid): Promise<void> {
  return client.delete(`/documents/${id}`)
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

export function getCharge(client: ApiClient, id: Uuid): Promise<Charge> {
  return client.get<Charge>(`/charges/${id}`)
}

export function voidCharge(client: ApiClient, id: Uuid): Promise<Charge> {
  return client.post<Charge>(`/charges/${id}/void`, {})
}

/**
 * Rinuncia all'incasso.
 *
 * Ammesso anche su un addebito **parzialmente pagato**, a differenza
 * dell'annullamento: l'incassato resta incassato e si rinuncia al residuo. Su uno
 * `paid`, `void` o già stornato risponde `409 charge.invalid_transition`.
 */
export function writeOffCharge(client: ApiClient, id: Uuid): Promise<Charge> {
  return client.post<Charge>(`/charges/${id}/write-off`, {})
}

// ---------------------------------------------------------------------------
// Pagamenti

export interface PaymentFilters {
  status?: PaymentStatus | undefined
  lease_id?: Uuid | undefined
  after?: Uuid | undefined
}

/**
 * La collezione dei pagamenti dell'organizzazione.
 *
 * **Con `status: 'pending'` è la coda delle dichiarazioni**, ed è la ragione per
 * cui questa rotta esiste: senza, una dichiarazione si troverebbe solo aprendo gli
 * addebiti a uno a uno.
 *
 * A restringere le righe è la policy e non un ruolo: un inquilino che la chiami
 * riceve `200` con i **propri** pagamenti. Nessuno di questi stati è derivato,
 * quindi nessun filtro risponde `422` — è la differenza con gli addebiti, dove
 * `?status=overdue` è rifiutato.
 */
export function listPayments(
  client: ApiClient,
  filters: PaymentFilters = {},
): Promise<Page<Payment>> {
  return client.get<Page<Payment>>(
    `/payments${query({
      limit: LIMIT,
      status: filters.status,
      lease_id: filters.lease_id,
      after: filters.after,
    })}`,
  )
}

/** I pagamenti di un addebito. Su un addebito che non si vede risponde `404`. */
export function listChargePayments(
  client: ApiClient,
  chargeId: Uuid,
  after?: Uuid,
): Promise<Page<Payment>> {
  return client.get<Page<Payment>>(
    `/charges/${chargeId}/payments${query({ limit: LIMIT, after })}`,
  )
}

export function getPayment(client: ApiClient, id: Uuid): Promise<Payment> {
  return client.get<Payment>(`/payments/${id}`)
}

export interface RecordPaymentInput {
  amount: string
  method: PaymentMethod
  /** Assente significa «adesso». */
  paid_at?: string
  reference?: string
  /**
   * A chi attribuire l'incasso.
   *
   * **Non è anagrafica, ed è una scelta esplicita.** Ometterlo lascia il pagamento
   * visibile allo staff soltanto: chi ha pagato non lo vedrà nel proprio elenco,
   * pur vedendone l'effetto sul saldo. È lecito — un bonifico da un conto che non
   * si sa a chi ricondurre esiste — ma va scelto, non dimenticato.
   */
  payer_user_id?: string
}

/**
 * Registra un incasso: nasce **`succeeded`** e muove il saldo.
 *
 * `201` se è stato scritto adesso, `200` se la chiave era già stata usata e si sta
 * restituendo quello di prima. **Per il client i due sono lo stesso esito
 * riuscito**, ed è la ragione per cui `ApiClient` non li distingue: costringere a
 * distinguerli inviterebbe a trattare il secondo come un errore, che è
 * precisamente ciò che l'idempotenza esiste per evitare.
 *
 * La chiave si genera **all'intento** e si riusa a ogni tentativo: vedi
 * `@/api/idempotency`.
 */
export function recordPayment(
  client: ApiClient,
  chargeId: Uuid,
  input: RecordPaymentInput,
  idempotencyKey: string,
): Promise<Payment> {
  return client.post<Payment>(`/charges/${chargeId}/payments`, input, {
    headers: { 'idempotency-key': idempotencyKey },
  })
}

export interface DeclarePaymentInput {
  amount: string
  method: PaymentMethod
  paid_at?: string
  reference?: string
}

/**
 * L'inquilino dichiara di aver pagato: nasce **`pending`** e **non muove il
 * saldo**.
 *
 * **Non c'è `payer_user_id`**: chi dichiara è sempre chi chiama, perché
 * `payments_insert` lo esige a chi non è staff. Passarlo produrrebbe zero righe
 * inserite, cioè `auth.denied_by_policy`.
 */
export function declarePayment(
  client: ApiClient,
  chargeId: Uuid,
  input: DeclarePaymentInput,
  idempotencyKey: string,
): Promise<Payment> {
  return client.post<Payment>(`/charges/${chargeId}/payment-declarations`, input, {
    headers: { 'idempotency-key': idempotencyKey },
  })
}

/** Conferma: da qui il saldo si muove e lo stato dell'addebito si ricalcola. */
export function confirmPayment(client: ApiClient, id: Uuid): Promise<Payment> {
  return client.post<Payment>(`/payments/${id}/confirm`, {})
}

/**
 * Rifiuta: passa a `cancelled` e **il saldo resta dov'era**, perché una
 * dichiarazione `pending` non lo aveva mosso. Azzera anche `paid_at`.
 */
export function rejectPayment(client: ApiClient, id: Uuid): Promise<Payment> {
  return client.post<Payment>(`/payments/${id}/reject`, {})
}

// ---------------------------------------------------------------------------
// Spese co-living

export interface ExpenseGroupFilters {
  property_id?: Uuid | undefined
  is_archived?: boolean | undefined
  after?: Uuid | undefined
}

/**
 * I gruppi di cui si è **membri attivi**.
 *
 * A restringere le righe è `can_access_expense_group`, che esige `left_at IS NULL`:
 * chi è uscito da un gruppo non lo vede più, nemmeno con un conto aperto. Non è un
 * filtro che il client possa togliere.
 *
 * **Le risposte non portano i membri.** `members` è assente qui e presente sul
 * dettaglio: un elenco che provasse a mostrare chi c'è dentro mostrerebbe
 * `undefined`.
 */
export function listExpenseGroups(
  client: ApiClient,
  filters: ExpenseGroupFilters = {},
): Promise<Page<ExpenseGroup>> {
  return client.get<Page<ExpenseGroup>>(
    `/expense-groups${query({
      limit: LIMIT,
      property_id: filters.property_id,
      is_archived: filters.is_archived,
      after: filters.after,
    })}`,
  )
}

/** Il dettaglio, **con la composizione**: chi è uscito compreso. */
export function getExpenseGroup(client: ApiClient, id: Uuid): Promise<ExpenseGroup> {
  return client.get<ExpenseGroup>(`/expense-groups/${id}`)
}

export interface CreateExpenseGroupInput {
  name: string
  kind?: ExpenseGroupKind
  /** Facoltativo per l'API. Il form lo chiede sempre: senza, non c'è un elenco di
   *  residenti da cui pescare i membri. */
  property_id?: Uuid
  default_split_method?: SplitMethod
  /** Il peso di chi crea, con tre decimali. La doppia si esprime con `"2"`. */
  weight?: string
  /**
   * **Il proprio nome, e va mandato sempre.**
   *
   * È l'unico momento in cui si può: nessuna risposta di questo modulo porta un
   * nome, e la membership è l'unico posto in cui vive. Vedi
   * `@/features/expenses/names`.
   */
  display_name?: string
}

/**
 * Crea un gruppo, **con chi lo crea già dentro**.
 *
 * Le due scritture stanno nella stessa transazione. Non è una comodità: un
 * inquilino soddisfa `expense_groups_insert` ma non
 * `expense_group_members_insert`, perché appena creato il gruppo non ne è membro e
 * quindi non potrebbe iscrivere nemmeno sé stesso.
 */
export function createExpenseGroup(
  client: ApiClient,
  input: CreateExpenseGroupInput,
): Promise<ExpenseGroup> {
  return client.post<ExpenseGroup>('/expense-groups', input)
}

/**
 * Chiude il conto: niente più spese né regolamenti, `409
 * expense_group.archived` a chi ci prova.
 *
 * **Non è una cancellazione** — saldi e spese restano leggibili — e si può anche
 * con dei conti aperti: chi se n'è andato senza pagare non deve poter tenere il
 * gruppo in ostaggio. Risponde `200` con il gruppo, non `204`.
 */
export function archiveExpenseGroup(client: ApiClient, id: Uuid): Promise<ExpenseGroup> {
  return client.post<ExpenseGroup>(`/expense-groups/${id}/archive`, {})
}

export interface AddGroupMemberInput {
  user_id: Uuid
  room_id?: Uuid
  /**
   * **Sempre valorizzato dal client**, con il `full_name` del residente scelto.
   *
   * L'API lo accetta assente e allora il nome non esiste più da nessuna parte: i
   * saldi di quella persona sarebbero un UUID, e per chi ha lasciato la casa non
   * c'è modo di recuperarlo.
   */
  display_name?: string
  /** Tre decimali. Chi **rientra** lo ridichiara: la stanza può essere cambiata. */
  weight?: string
}

/**
 * Iscrive **o fa rientrare**: è la stessa rotta.
 *
 * `expense_group_members_group_user_uk` è unico su `(group_id, user_id)` e non è
 * parziale, quindi una seconda riga non può esistere e il rientro è un
 * aggiornamento. Su un membro **già attivo** risponde `409
 * expense_group.membership_exists`.
 *
 * **Non verifica che la persona abiti lì**, ed è voluto: `property_id` è
 * facoltativo, e chi ha lasciato la casa il mese scorso e sta ancora saldando è un
 * caso legittimo. Il client non deve reintrodurre quel controllo.
 */
export function addGroupMember(
  client: ApiClient,
  groupId: Uuid,
  input: AddGroupMemberInput,
): Promise<ExpenseGroupMember> {
  return client.post<ExpenseGroupMember>(`/expense-groups/${groupId}/members`, input)
}

/**
 * Esce dal gruppo: `200` **con la membership aggiornata**, non `204`.
 *
 * La riga resta con `left_at` valorizzato, perché le quote a suo nome restano e i
 * saldi partono da lì. **Si esce per sé stessi**: togliere qualcun altro richiede
 * lo staff, e a un membro l'API risponde `403
 * expense_group.membership_not_yours` — dalla migration `0029` è la RLS a dirlo,
 * non solo l'API.
 */
export function leaveGroup(
  client: ApiClient,
  groupId: Uuid,
  userId: Uuid,
): Promise<ExpenseGroupMember> {
  return client.request<ExpenseGroupMember>(`/expense-groups/${groupId}/members/${userId}`, {
    method: 'DELETE',
  })
}

/**
 * I saldi: quanto ognuno ha anticipato, quanto gli è stato imputato, e il netto.
 *
 * **Comprende chi è uscito con un conto aperto** (migration `0027`), ed è ciò che
 * fa sommare i netti a zero. I creditori arrivano per primi e il client **non
 * riordina**. Nessun cursore: sono i membri di un gruppo.
 */
export function getGroupBalances(client: ApiClient, groupId: Uuid): Promise<Items<GroupBalance>> {
  return client.get<Items<GroupBalance>>(`/expense-groups/${groupId}/balances`)
}

export interface ExpenseFilters {
  category?: ExpenseCategory | undefined
  after?: Uuid | undefined
}

/** L'elenco **senza le quote**: quelle si leggono dal dettaglio. */
export function listExpenses(
  client: ApiClient,
  groupId: Uuid,
  filters: ExpenseFilters = {},
): Promise<Page<Expense>> {
  return client.get<Page<Expense>>(
    `/expense-groups/${groupId}/expenses${query({
      limit: LIMIT,
      category: filters.category,
      after: filters.after,
    })}`,
  )
}

/** Il dettaglio **con le quote**, la cui somma è esattamente l'importo. */
export function getExpense(client: ApiClient, id: Uuid): Promise<Expense> {
  return client.get<Expense>(`/expenses/${id}`)
}

export interface RecordExpenseInput {
  description: string
  amount: string
  category?: ExpenseCategory
  /**
   * Chi ha anticipato. **Assente significa «io»**, e deve essere un membro
   * **attivo**: un estraneo dà `409 expense.payer_not_in_group`. Lo schema non lo
   * impone, e senza il controllo quel credito non comparirebbe in nessun saldo.
   */
  paid_by_user_id?: Uuid
  /** Assente significa «oggi». È una **data**, non un istante: nessun fuso. */
  expense_date?: string
  /** `equal` o `weighted`. Gli altri tre dell'enum danno `422`. */
  split_method?: SplitMethod
  notes?: string
}

/**
 * Registra una spesa e la ripartisce: risponde `201` **con le quote**.
 *
 * **Nessuna chiave di idempotenza**, a differenza delle due rotte che creano un
 * pagamento: questa non la richiede, e aggiungerla per simmetria manderebbe un
 * header che nessuno legge.
 */
export function recordExpense(
  client: ApiClient,
  groupId: Uuid,
  input: RecordExpenseInput,
): Promise<Expense> {
  return client.post<Expense>(`/expense-groups/${groupId}/expenses`, input)
}

export interface SettlementFilters {
  status?: SettlementStatus | undefined
  after?: Uuid | undefined
}

/** I regolamenti del gruppo. `?status=pending` è la coda di chi deve confermare. */
export function listSettlements(
  client: ApiClient,
  groupId: Uuid,
  filters: SettlementFilters = {},
): Promise<Page<Settlement>> {
  return client.get<Page<Settlement>>(
    `/expense-groups/${groupId}/settlements${query({
      limit: LIMIT,
      status: filters.status,
      after: filters.after,
    })}`,
  )
}

export interface DeclareSettlementInput {
  /** A chi si è pagato: si sceglie dall'elenco dei **creditori**. */
  to_user_id: Uuid
  amount: string
  method?: PaymentMethod
  /** Assente significa «adesso». La conferma **non la riscrive**. */
  settled_at?: string
  note?: string
}

/**
 * Dichiara di aver pagato un coinquilino: nasce `pending` e **non muove i saldi**.
 *
 * **Non c'è `from_user_id`**: chi dichiara è sempre chi chiama, perché
 * `settlements_insert` lo esige a chi non è staff.
 *
 * **Nessuna chiave di idempotenza**: questa rotta non la richiede, a differenza
 * delle due che creano un pagamento. Non è una svista dell'API ed è da non
 * replicare per simmetria.
 */
export function declareSettlement(
  client: ApiClient,
  groupId: Uuid,
  input: DeclareSettlementInput,
): Promise<Settlement> {
  return client.post<Settlement>(`/expense-groups/${groupId}/settlements`, input)
}

/**
 * Conferma: da qui i saldi si muovono.
 *
 * **Non ricalcola niente**, a differenza di `ConfirmPayment`: il saldo del gruppo
 * non è memorizzato da nessuna parte, si legge dalla vista. Il client invalida la
 * chiave dei saldi e basta.
 *
 * A confermare è **chi riceve** — è l'unico che sa se il bonifico è arrivato — e a
 * chiunque altro l'API risponde `403 settlement.not_addressed_to_you`.
 */
export function confirmSettlement(client: ApiClient, id: Uuid): Promise<Settlement> {
  return client.post<Settlement>(`/settlements/${id}/confirm`, {})
}

/** Rifiuta: il denaro non è arrivato. È di **chi riceve**, e azzera `settled_at`. */
export function rejectSettlement(client: ApiClient, id: Uuid): Promise<Settlement> {
  return client.post<Settlement>(`/settlements/${id}/reject`, {})
}

/**
 * Ritira la propria dichiarazione. È di **chi ha dichiarato**, e a chi riceve
 * l'API risponde `403 settlement.not_declared_by_you`.
 *
 * **Esiste qui e non fra i pagamenti**: `settlements_update` nomina anche
 * `from_user_id`, `payments_update` è staff soltanto. Fra pari il ritiro è lecito,
 * verso il proprietario no — e chi ha letto l'altra schermata si aspetta il
 * contrario.
 */
export function cancelSettlement(client: ApiClient, id: Uuid): Promise<Settlement> {
  return client.post<Settlement>(`/settlements/${id}/cancel`, {})
}
