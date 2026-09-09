import { describe, expect, it } from 'vitest'

import type { Lease, LeaseStatus } from '@/api/types'
import { overlappingLeases } from './overlap'

/**
 * L'avviso deve dire **esattamente** ciò che il database rifiuterebbe.
 *
 * Un avviso che sbaglia in eccesso è peggio del silenzio: chi lo vede sulla
 * seconda stanza di un appartamento impara a ignorarlo, e la volta che è vero non
 * lo legge.
 */

/**
 * `end_date: null` significa **senza scadenza**, e non è un capriccio del test:
 * con `exactOptionalPropertyTypes` il campo si omette, non si mette a
 * `undefined`, ed è la stessa distinzione che l'API fa sul filo.
 */
function lease(overrides: Partial<Omit<Lease, 'end_date'>> & { end_date?: string | null }): Lease {
  const { end_date, ...resto }: Omit<Lease, 'end_date'> & { end_date?: string | null } = {
    id: crypto.randomUUID(),
    organization_id: 'org',
    property_id: 'immobile',
    lease_type: 'whole_property' as const,
    status: 'active' as LeaseStatus,
    effective_status: 'active' as LeaseStatus,
    start_date: '2026-01-01',
    end_date: '2027-01-01',
    notice_months: null,
    auto_renew: false,
    renewal_months: null,
    rent_amount: '800.00',
    currency: 'EUR',
    rent_frequency: 'monthly',
    rent_due_day: 1,
    deposit_amount: '0.00',
    utilities_included: false,
    istat_indexation: false,
    signed_at: null,
    terminated_at: null,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    ...overrides,
  }
  return end_date === null || end_date === undefined ? resto : { ...resto, end_date }
}

describe('la sovrapposizione, come la conta il database', () => {
  it('trova un periodo che si accavalla sullo stesso immobile', () => {
    const found = overlappingLeases([lease({})], {
      roomId: undefined,
      start: '2026-06-01',
      end: '2026-12-01',
    })
    expect(found).toHaveLength(1)
  })

  /** Il periodo è semiaperto: fine e inizio nello stesso giorno non si toccano. */
  it('non trova nulla quando uno finisce il giorno in cui l’altro comincia', () => {
    const found = overlappingLeases([lease({ end_date: '2026-06-01' })], {
      roomId: undefined,
      start: '2026-06-01',
      end: '2027-01-01',
    })
    expect(found).toHaveLength(0)
  })

  /**
   * **Le bozze non contano**, ed è deliberato nel vincolo: due ipotesi
   * alternative sullo stesso periodo sono lavoro normale.
   */
  it('ignora le bozze e gli stati che il vincolo non copre', () => {
    const periodo = { roomId: undefined, start: '2026-02-01', end: '2026-03-01' }
    expect(overlappingLeases([lease({ status: 'draft' })], periodo)).toHaveLength(0)
    expect(overlappingLeases([lease({ status: 'terminated' })], periodo)).toHaveLength(0)
    expect(overlappingLeases([lease({ status: 'cancelled' })], periodo)).toHaveLength(0)
    expect(overlappingLeases([lease({ status: 'pending_signature' })], periodo)).toHaveLength(1)
  })

  /**
   * **Il co-living**: due stanze diverse della stessa casa non si escludono, e
   * avvisare lì renderebbe l'avviso rumore.
   */
  it('confronta stanza con stanza', () => {
    const occupata = lease({ room_id: 'stanza-1', lease_type: 'room' })
    expect(
      overlappingLeases([occupata], { roomId: 'stanza-2', start: '2026-02-01', end: '2026-03-01' }),
    ).toHaveLength(0)
    expect(
      overlappingLeases([occupata], { roomId: 'stanza-1', start: '2026-02-01', end: '2026-03-01' }),
    ).toHaveLength(1)
  })

  /** Un contratto sull'immobile intero non si confronta con quello di una stanza. */
  it('non confonde l’immobile intero con una stanza', () => {
    const intero = lease({})
    expect(
      overlappingLeases([intero], { roomId: 'stanza-1', start: '2026-02-01', end: '2026-03-01' }),
    ).toHaveLength(0)
  })

  /** Senza fine il contratto dura per sempre, come `infinity` nel vincolo. */
  it('tratta la fine assente come indeterminata', () => {
    const senzaFine = lease({ end_date: null })
    expect(
      overlappingLeases([senzaFine], { roomId: undefined, start: '2030-01-01', end: undefined }),
    ).toHaveLength(1)
  })
})
