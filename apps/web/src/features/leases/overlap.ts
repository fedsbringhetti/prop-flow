import type { Lease } from '@/api/types'

/**
 * I contratti che si sovrappongono a un periodo, **come li conta il database**.
 *
 * Esiste perché il `409` arrivava solo all'attivazione, cioè dopo aver redatto il
 * contratto e aggiunto le parti: il vincolo è giusto e resta, ma dirlo prima è la
 * differenza fra un avviso e un lavoro da rifare.
 *
 * # Le due regole vanno replicate esattamente
 *
 * I vincoli `leases_no_overlap_property` e `leases_no_overlap_room` della `0005`
 * dicono due cose, e sbagliarne una rende l'avviso peggiore del silenzio:
 *
 * 1. **contano solo tre stati** — `pending_signature`, `active`, `expiring` — e
 *    non le bozze. Due bozze sullo stesso periodo sono legittime: sono ipotesi
 *    alternative, ed è la ragione per cui le bozze sono escluse *apposta*;
 * 2. **la dimensione è la stanza quando c'è**, altrimenti l'immobile. Confrontare
 *    tutto per immobile avviserebbe su due stanze diverse della stessa casa, che è
 *    il caso normale del co-living.
 *
 * # Il periodo è semiaperto
 *
 * `[inizio, fine)`, come `daterange(start_date, end_date, '[)')`: un contratto che
 * finisce il 1° gennaio e uno che comincia il 1° gennaio **non** si sovrappongono.
 * Fine assente significa indeterminato, cioè `infinity`.
 */
const BLOCKING: readonly Lease['status'][] = ['pending_signature', 'active', 'expiring']

/** Una data ISO, o `infinity` per un contratto senza fine. */
const FOREVER = '9999-12-31'

export interface Period {
  roomId: string | undefined
  start: string
  end: string | undefined
}

export function overlappingLeases(leases: readonly Lease[], period: Period): Lease[] {
  if (!period.start) return []
  const start = period.start
  const end = period.end || FOREVER

  return leases.filter((lease) => {
    if (!BLOCKING.includes(lease.status)) return false
    // Stanza contro stanza, immobile contro immobile: un contratto sull'immobile
    // intero e uno su una stanza **non** si escludono a vicenda nel database, e
    // avvisare qui direbbe una cosa che il vincolo non dice.
    if ((lease.room_id ?? undefined) !== period.roomId) return false

    const otherStart = lease.start_date
    const otherEnd = lease.end_date ?? FOREVER
    return start < otherEnd && otherStart < end
  })
}
