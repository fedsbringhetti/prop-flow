import { describe, expect, it } from 'vitest'

import type { LeaseParty } from '@/api/types'
import {
  assignedShare,
  hasPrimary,
  invitableParties,
  proposedHandover,
  remainingShare,
} from './shares'

/**
 * La quota proposta è **la stessa aritmetica del server**, e questo test esiste
 * perché le due definizioni possono divergere in silenzio: se il form proponesse
 * più di quanto resta, l'attivazione rifiuterebbe con un numero che chi compila
 * non ha scelto.
 */

function party(overrides: Partial<LeaseParty>): LeaseParty {
  return {
    id: crypto.randomUUID(),
    lease_id: 'contratto',
    role: 'tenant',
    full_name: 'Parte',
    share_percent: '0.00',
    is_primary: false,
    joined_at: null,
    left_at: null,
    ...overrides,
  }
}

describe('la quota che resta da assegnare', () => {
  it('è cento quando non c’è nessuno', () => {
    expect(remainingShare([])).toBe('100.00')
  })

  it('scala di ciò che è già assegnato, senza passare da un float', () => {
    const parties = [party({ share_percent: '33.33' }), party({ share_percent: '33.34' })]
    expect(assignedShare(parties)).toBe('66.67')
    expect(remainingShare(parties)).toBe('33.33')
  })

  /**
   * **Il garante non paga il canone.** È `shares_rent` nel dominio, e contarlo qui
   * proporrebbe una quota più bassa di quella vera: l'attivazione somma solo
   * inquilini e coinquilini, e rifiuterebbe.
   */
  it('non conta il garante', () => {
    const parties = [
      party({ share_percent: '60.00' }),
      party({ share_percent: '100.00', role: 'guarantor' }),
    ]
    expect(remainingShare(parties)).toBe('40.00')
  })

  it('non conta chi è uscito', () => {
    const parties = [party({ share_percent: '50.00', left_at: '2026-01-31' })]
    expect(remainingShare(parties)).toBe('100.00')
  })

  /** Mai negativa: il server fa lo stesso, e un campo con `-20` non si corregge. */
  it('non scende sotto zero quando le quote sono già oltre cento', () => {
    const parties = [party({ share_percent: '100.00' }), party({ share_percent: '20.00' })]
    expect(remainingShare(parties)).toBe('0.00')
  })
})

describe('l’intestatario principale', () => {
  it('è già preso quando una parte ce l’ha', () => {
    expect(hasPrimary([party({ is_primary: true })])).toBe(true)
  })

  it('torna libero quando chi ce l’aveva è uscito', () => {
    expect(hasPrimary([party({ is_primary: true, left_at: '2026-01-31' })])).toBe(false)
  })
})

/**
 * Il subentro propone le quote **com'erano** (ADR-056): chi entra con quella di chi
 * esce, chi resta con la sua. Sono i valori con cui i campi partono pieni.
 */
describe('le quote proposte dal subentro', () => {
  const anna = party({ full_name: 'Anna', share_percent: '60.00', is_primary: true })
  const marco = party({ full_name: 'Marco', role: 'cotenant', share_percent: '40.00' })
  const garante = party({ full_name: 'Garante', role: 'guarantor', share_percent: '0.00' })
  const uscito = party({ full_name: 'Uscito', share_percent: '30.00', left_at: '2026-01-31' })

  it('dà a chi entra la quota di chi esce, e lascia a chi resta la sua', () => {
    const proposal = proposedHandover([anna, marco, garante, uscito], anna.id)
    expect(proposal?.joining).toBe('60.00')
    expect(proposal?.staying.map((row) => [row.party.full_name, row.share_percent])).toEqual([
      ['Marco', '40.00'],
    ])
  })

  it('segue chi esce quando la scelta cambia', () => {
    expect(proposedHandover([anna, marco], marco.id)?.joining).toBe('40.00')
  })

  it('non propone niente per chi è già uscito', () => {
    expect(proposedHandover([anna, uscito], uscito.id)).toBeNull()
  })
})

/**
 * Da attivo l'invito raggiunge solo una parte che esiste: senza account e con
 * un'email a cui intestarlo. Un'email nuova creerebbe una parte (ADR-056).
 */
describe('chi si invita su un contratto attivo', () => {
  it('è chi è scritto sul contratto, con un’email e senza account', () => {
    const inAttesa = party({ full_name: 'In attesa', email: 'attesa@example.com' })
    const collegato = party({ email: 'dentro@example.com', user_id: 'utente' })
    const senzaEmail = party({ full_name: 'Senza email' })
    const uscito = party({ email: 'fuori@example.com', left_at: '2026-01-31' })
    expect(invitableParties([inAttesa, collegato, senzaEmail, uscito])).toEqual([inAttesa])
  })
})
