import { Badge, type BadgeTone } from '@/components/ui/badge'
import { t } from '@/i18n'
import type {
  BillStatus,
  ChargeStatus,
  LeaseStatus,
  PaymentStatus,
  PropertyStatus,
  RoomStatus,
  TicketPriority,
  TicketStatus,
} from '@/api/types'

/**
 * Il colore di uno stato dice **che cosa richiede attenzione**.
 *
 * Le mappe sono esaustive per costruzione: `Record<Stato, Tono>` non compila se
 * l'API aggiunge un valore all'enum e qui non lo si traduce. È l'equivalente del
 * «un enum sconosciuto in lettura dà errore e non un default» del backend — un
 * colore di ripiego renderebbe muto proprio ciò che è cambiato.
 */

/**
 * **Quando lo stato dichiarato va mostrato accanto al derivato.**
 *
 * La regola vecchia era «ogni volta che differiscono», ed era scritta per
 * l'anomalia — una stanza in ristrutturazione e insieme occupata. Ma la divergenza
 * è **garantita** nel caso normale: nessuno riscrive `active` quando un contratto
 * scade, `occupied` non lo scrive nessuno mai, `rented` nemmeno. La parentesi
 * compariva quindi su quasi ogni riga, che è il modo di rendere invisibile proprio
 * il caso per cui esisteva.
 *
 * La regola nuova, in una riga: **il dichiarato compare solo quando è un giudizio
 * che il fatto contraddice.** `available` su una stanza e `active` su un contratto
 * sono il default che la derivazione esiste per superare, non una decisione di
 * qualcuno; `unavailable` e `maintenance` sì.
 *
 * `judgements` elenca quindi gli stati memorizzati che **qualcuno ha scelto**. È
 * un elenco e non un'esclusione perché la domanda giusta è «chi l'ha deciso», e un
 * elenco di ciò che si esclude si riempirebbe da sé di ogni valore nuovo.
 */
function isAnomaly<S extends string>(
  status: S,
  effectiveStatus: S | undefined,
  judgements: readonly S[],
): boolean {
  if (effectiveStatus === undefined || status === effectiveStatus) return false
  return judgements.includes(status)
}

const propertyTone: Record<PropertyStatus, BadgeTone> = {
  draft: 'neutral',
  available: 'info',
  partially_rented: 'info',
  rented: 'success',
  maintenance: 'warning',
  archived: 'muted',
}

/**
 * Gli stati di un immobile che **qualcuno ha scelto**.
 *
 * `draft`, `maintenance` e `archived` sono giudizi di chi amministra: un immobile
 * in manutenzione che risulta affittato è l'anomalia che il doppio badge esiste per
 * far vedere. `available` no — è il default da cui si parte, e la derivazione
 * esiste per superarlo. `rented` e `partially_rented` nemmeno: non si scelgono
 * dalla tendina (ADR-019), quindi una riga che li porti è un residuo, non una
 * decisione.
 */
const propertyJudgements = ['draft', 'maintenance', 'archived'] as const satisfies readonly PropertyStatus[]

/**
 * Lo stato di un immobile: **il fatto**, e il dichiarato solo se lo smentisce.
 */
export function PropertyStatusBadge({
  status,
  effectiveStatus,
}: {
  status: PropertyStatus
  effectiveStatus?: PropertyStatus
}) {
  const shown = effectiveStatus ?? status
  if (!isAnomaly(status, effectiveStatus, propertyJudgements)) {
    return <Badge tone={propertyTone[shown]}>{t.property.statuses[shown]}</Badge>
  }
  return (
    <span className="inline-flex items-center gap-1">
      <Badge tone={propertyTone[shown]}>{t.property.statuses[shown]}</Badge>
      <span className="text-xs text-ink-muted">({t.property.statuses[status]})</span>
    </span>
  )
}

const roomTone: Record<RoomStatus, BadgeTone> = {
  available: 'info',
  occupied: 'success',
  unavailable: 'muted',
}

/**
 * L'unico stato di una stanza che sia una decisione: **fuori uso**.
 *
 * `available` è il default, e `occupied` non lo scrive nessuno — discende dal
 * contratto in essere. Una stanza dichiarata *non disponibile* e insieme occupata è
 * il caso che questo badge esiste per rendere visibile: qualcuno l'ha messa in
 * ristrutturazione mentre ci abita una persona.
 */
const roomJudgements = ['unavailable'] as const satisfies readonly RoomStatus[]

/**
 * Lo stato di una stanza: **il fatto**, e il dichiarato solo se lo smentisce.
 */
export function RoomStatusBadge({
  status,
  effectiveStatus,
}: {
  status: RoomStatus
  effectiveStatus?: RoomStatus
}) {
  const shown = effectiveStatus ?? status
  if (!isAnomaly(status, effectiveStatus, roomJudgements)) {
    return <Badge tone={roomTone[shown]}>{t.room.statuses[shown]}</Badge>
  }
  return (
    <span className="inline-flex items-center gap-1">
      <Badge tone={roomTone[shown]}>{t.room.statuses[shown]}</Badge>
      <span className="text-xs text-ink-muted">({t.room.statuses[status]})</span>
    </span>
  )
}

const leaseTone: Record<LeaseStatus, BadgeTone> = {
  draft: 'neutral',
  pending_signature: 'warning',
  active: 'success',
  expiring: 'warning',
  expired: 'danger',
  terminated: 'muted',
  cancelled: 'muted',
}

/**
 * **Nessuno**, ed è un fatto del dominio e non una svista.
 *
 * `Lease::effective_status` restituisce lo stato memorizzato per ogni valore che
 * non sia `active`: la divergenza esiste **solo** a partire da `active`, che è il
 * default da cui si parte e non un giudizio. Un contratto non può quindi essere in
 * uno stato scelto che il calendario smentisce, e la parentesi qui non ha mai un
 * caso da mostrare.
 *
 * L'elenco resta scritto — vuoto — perché la regola sia leggibile qui come sugli
 * altri due, e perché il giorno in cui nascesse uno stato derivato da qualcosa che
 * non è `active` la domanda si ponga in questo punto.
 */
const leaseJudgements = [] as const satisfies readonly LeaseStatus[]

/**
 * Lo stato **effettivo**, non quello memorizzato.
 *
 * `effective_status` vale `expiring` o `expired` mentre `status` resta `active`,
 * perché nessuno scheduler riscrive le righe: mostrare `status` direbbe «attivo» di
 * un contratto scaduto ieri.
 *
 * **La ragione per cui il dichiarato non sparisce dal prodotto.** Chi filtra per
 * «attivo» e si vede tornare un contratto marcato «Scaduto» deve poter capire
 * perché: l'informazione serve, ed era il *posto* a essere sbagliato. Adesso vive
 * in due punti — l'avviso accanto al filtro in `LeasesPage`, dove la domanda nasce,
 * e il campo «Stato dichiarato» nella scheda del contratto, dove chi amministra lo
 * cerca — invece che ripetuta su venticinque righe.
 */
export function LeaseStatusBadge({
  status,
  effectiveStatus,
}: {
  status: LeaseStatus
  effectiveStatus: LeaseStatus
}) {
  if (!isAnomaly(status, effectiveStatus, leaseJudgements)) {
    return <Badge tone={leaseTone[effectiveStatus]}>{t.lease.statuses[effectiveStatus]}</Badge>
  }
  return (
    <span className="inline-flex items-center gap-1">
      <Badge tone={leaseTone[effectiveStatus]}>{t.lease.statuses[effectiveStatus]}</Badge>
      <span className="text-xs text-ink-muted">({t.lease.statuses[status]})</span>
    </span>
  )
}

const billTone: Record<BillStatus, BadgeTone> = {
  draft: 'neutral',
  pending_review: 'warning',
  validated: 'info',
  allocated: 'success',
  paid: 'success',
  disputed: 'danger',
  void: 'muted',
}

export function BillStatusBadge({ status }: { status: BillStatus }) {
  return <Badge tone={billTone[status]}>{t.bill.statuses[status]}</Badge>
}

const chargeTone: Record<ChargeStatus, BadgeTone> = {
  draft: 'neutral',
  issued: 'info',
  partially_paid: 'warning',
  paid: 'success',
  overdue: 'danger',
  void: 'muted',
  written_off: 'muted',
}

/**
 * `overdue` è derivato e non è mai memorizzato, come `expired` sui contratti.
 *
 * **La coppia resta**, e non per dimenticanza: `Charge::effective_status` diverge
 * solo da uno stato `is_outstanding()`, cioè da `issued` **o da
 * `partially_paid`** — e «parzialmente pagato» non si ricostruisce da «scaduto».
 * La regola di `isAnomaly` qui non si applica pari pari: sugli altri tre il
 * dichiarato è un giudizio o un default, qui è un fatto contabile che il derivato
 * nasconderebbe. Con `issued` la parentesi è invece lo stesso rumore che `R30`
 * descrive, e sistemarla è una voce a sé in `docs/12-da-fare.md`.
 */
export function ChargeStatusBadge({
  status,
  effectiveStatus,
}: {
  status: ChargeStatus
  effectiveStatus: ChargeStatus
}) {
  if (status === effectiveStatus) {
    return <Badge tone={chargeTone[status]}>{t.charge.statuses[status]}</Badge>
  }
  return (
    <span className="inline-flex items-center gap-1">
      <Badge tone={chargeTone[effectiveStatus]}>{t.charge.statuses[effectiveStatus]}</Badge>
      <span className="text-xs text-ink-muted">({t.charge.statuses[status]})</span>
    </span>
  )
}

/**
 * Lo stato di un pagamento.
 *
 * **Nessun valore è derivato**, a differenza degli addebiti e dei contratti: qui
 * `status` è sempre quello memorizzato, quindi non c'è una coppia da mostrare né
 * un filtro da rifiutare.
 *
 * `pending` è `warning` perché **aspetta una persona** — è la coda — e non perché
 * sia un problema. `cancelled` è `muted` e non `danger`: una dichiarazione
 * rifiutata è una cosa conclusa, non un guasto.
 *
 * I quattro stati che l'API non scrive mai hanno comunque il loro tono: una riga
 * che li portasse va mostrata, non nascosta dietro un colore di ripiego.
 */
const paymentTone: Record<PaymentStatus, BadgeTone> = {
  pending: 'warning',
  authorized: 'info',
  succeeded: 'success',
  failed: 'danger',
  cancelled: 'muted',
  refunded: 'muted',
  partially_refunded: 'info',
}

export function PaymentStatusBadge({ status }: { status: PaymentStatus }) {
  return <Badge tone={paymentTone[status]}>{t.payment.statuses[status]}</Badge>
}

/**
 * Lo stato di una segnalazione.
 *
 * **Nessuno è derivato**: è sempre quello memorizzato, quindi non c'è una coppia
 * da mostrare come per contratti e addebiti.
 *
 * I due `waiting_*` sono `warning` perché **aspettano una persona** — sono una
 * sospensione della lavorazione, non una fase a sé, e servono a dire perché una
 * segnalazione è ferma: senza un nome resterebbe una segnalazione che sembra
 * abbandonata. `resolved` ha comunque il suo tono, anche se l'API non lo scrive
 * mai: una riga che lo portasse va mostrata, non nascosta dietro un ripiego.
 */
const ticketTone: Record<TicketStatus, BadgeTone> = {
  open: 'info',
  triaged: 'info',
  assigned: 'info',
  in_progress: 'info',
  waiting_tenant: 'warning',
  waiting_owner: 'warning',
  resolved: 'success',
  closed: 'muted',
  cancelled: 'muted',
}

export function TicketStatusBadge({ status }: { status: TicketStatus }) {
  return <Badge tone={ticketTone[status]}>{t.ticket.statuses[status]}</Badge>
}

/**
 * La priorità.
 *
 * `medium` è `neutral` e non `info`: è il default di ogni segnalazione aperta
 * senza dichiararla, e colorare il caso normale toglie significato al colore
 * proprio dove serve.
 */
const priorityTone: Record<TicketPriority, BadgeTone> = {
  low: 'muted',
  medium: 'neutral',
  high: 'warning',
  urgent: 'danger',
}

export function TicketPriorityBadge({ priority }: { priority: TicketPriority }) {
  return <Badge tone={priorityTone[priority]}>{t.ticket.priorities[priority]}</Badge>
}
