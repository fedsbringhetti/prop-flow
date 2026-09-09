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

const propertyTone: Record<PropertyStatus, BadgeTone> = {
  draft: 'neutral',
  available: 'info',
  partially_rented: 'info',
  rented: 'success',
  maintenance: 'warning',
  archived: 'muted',
}

/**
 * Lo stato di un immobile, **dichiarato ed effettivo insieme**.
 *
 * Quando divergono si mostrano tutti e due, con il derivato in evidenza: un
 * immobile «in manutenzione» che risulta affittato è un'anomalia che chi
 * amministra deve vedere, e un badge solo la nasconderebbe qualunque dei due
 * scegliesse. È la stessa forma del badge dei contratti.
 */
export function PropertyStatusBadge({
  status,
  effectiveStatus,
}: {
  status: PropertyStatus
  effectiveStatus?: PropertyStatus
}) {
  if (effectiveStatus === undefined || status === effectiveStatus) {
    return <Badge tone={propertyTone[status]}>{t.property.statuses[status]}</Badge>
  }
  return (
    <span className="inline-flex items-center gap-1">
      <Badge tone={propertyTone[effectiveStatus]}>{t.property.statuses[effectiveStatus]}</Badge>
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
 * Lo stato di una stanza, dichiarato ed effettivo.
 *
 * «Occupata» non lo dichiara nessuno: discende dal contratto in essere. Una stanza
 * dichiarata **non disponibile** e insieme occupata è il caso che questo badge
 * esiste per rendere visibile.
 */
export function RoomStatusBadge({
  status,
  effectiveStatus,
}: {
  status: RoomStatus
  effectiveStatus?: RoomStatus
}) {
  if (effectiveStatus === undefined || status === effectiveStatus) {
    return <Badge tone={roomTone[status]}>{t.room.statuses[status]}</Badge>
  }
  return (
    <span className="inline-flex items-center gap-1">
      <Badge tone={roomTone[effectiveStatus]}>{t.room.statuses[effectiveStatus]}</Badge>
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
 * Lo stato **effettivo**, non quello memorizzato.
 *
 * `effective_status` può valere `expiring` o `expired` mentre `status` resta
 * `active`, perché nessuno scheduler riscrive le righe: mostrare `status`
 * direbbe «attivo» di un contratto scaduto ieri. Quando i due divergono si
 * mostrano entrambi — chi guarda deve poter capire perché il filtro «attivo»
 * restituisce un contratto marcato «scaduto».
 */
export function LeaseStatusBadge({
  status,
  effectiveStatus,
}: {
  status: LeaseStatus
  effectiveStatus: LeaseStatus
}) {
  if (status === effectiveStatus) {
    return <Badge tone={leaseTone[status]}>{t.lease.statuses[status]}</Badge>
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

/** Come per i contratti: `overdue` è derivato e non è mai memorizzato. */
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
