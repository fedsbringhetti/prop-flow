import { Badge, type BadgeTone } from '@/components/ui/badge'
import { t } from '@/i18n'
import type {
  BillStatus,
  ChargeStatus,
  LeaseStatus,
  PropertyStatus,
  RoomStatus,
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

export function PropertyStatusBadge({ status }: { status: PropertyStatus }) {
  return <Badge tone={propertyTone[status]}>{t.property.statuses[status]}</Badge>
}

const roomTone: Record<RoomStatus, BadgeTone> = {
  available: 'info',
  occupied: 'success',
  unavailable: 'muted',
}

export function RoomStatusBadge({ status }: { status: RoomStatus }) {
  return <Badge tone={roomTone[status]}>{t.room.statuses[status]}</Badge>
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
