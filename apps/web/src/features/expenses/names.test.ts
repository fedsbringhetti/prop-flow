import { describe, expect, it } from 'vitest'

import type { ExpenseGroupMember, PropertyResident } from '@/api/types'
import { t } from '@/i18n'
import { ownName, resolveName } from './names'

/**
 * I nomi, che sono **la** decisione di questa fetta.
 *
 * Nessuna risposta del modulo delle spese porta un nome: saldi, quote, spese e
 * regolamenti identificano le persone con il solo `user_id`, e `display_name`
 * sulla membership è facoltativo. Una schermata di saldi è fatta di nomi e numeri
 * e di nient'altro, quindi sbagliare qui produce una tabella di UUID.
 *
 * Il caso che conta è l'ultimo: **chi ha lasciato la casa e sta ancora saldando**.
 * Non è più fra i residenti, quindi qualunque soluzione che risolva i nomi
 * incrociandoli con `GET /properties/{id}/residents` funziona in ogni altro caso e
 * fallisce in quello — cioè si romperebbe **solo sui gruppi vecchi con qualcuno
 * uscito**, che è il difetto più difficile da notare fra quelli possibili qui.
 */

const GIULIA = '00000000-0000-4000-8000-00000000000a'
const MARCO = '00000000-0000-4000-8000-00000000000b'
const IGNOTO = '00000000-0000-4000-8000-00000000000c'

function member(userId: string, displayName?: string): ExpenseGroupMember {
  return {
    user_id: userId,
    ...(displayName === undefined ? {} : { display_name: displayName }),
    weight: '1.000',
    joined_at: '2026-01-10T09:00:00Z',
    left_at: null,
  }
}

const residents: PropertyResident[] = [{ user_id: GIULIA, full_name: 'Giulia Bianchi' }]

describe('i nomi di un gruppo di spesa', () => {
  it('usa il nome scritto all’ingresso', () => {
    expect(resolveName(GIULIA, [member(GIULIA, 'Giulia')], residents)).toBe('Giulia')
  })

  it('ripiega sui residenti quando la membership non porta un nome', () => {
    // Un gruppo creato da `curl`, o da una versione futura che dimentica la
    // regola: finché la persona abita ancora lì il nome si recupera.
    expect(resolveName(GIULIA, [member(GIULIA)], residents)).toBe('Giulia Bianchi')
  })

  it('non mostra mai l’identificatore quando non c’è nessun nome', () => {
    const resolved = resolveName(IGNOTO, [member(IGNOTO)], residents)
    expect(resolved).toBe(t.expense.memberUnknown)
    // Un identificatore in faccia a chi legge non è informazione, è una resa.
    expect(resolved).not.toContain(IGNOTO)
  })

  it('nomina chi ha lasciato la casa e sta ancora saldando', () => {
    // Marco è uscito dal gruppo **e** dall'immobile: non compare fra i residenti,
    // e il suo debito compare invece nei saldi che leggono gli altri. È il caso
    // per cui il nome si scrive all'ingresso, ed è il soggetto della verifica che
    // chiude questa fetta.
    const uscito: ExpenseGroupMember = {
      ...member(MARCO, 'Marco Neri'),
      left_at: '2026-08-31T18:00:00Z',
    }
    expect(resolveName(MARCO, [uscito], residents)).toBe('Marco Neri')
  })

  it('senza i residenti risolve comunque con ciò che ha', () => {
    // `property_id` sul gruppo è facoltativo: un gruppo senza immobile non ha
    // nemmeno un elenco da interrogare, e la schermata non deve aspettarlo.
    expect(resolveName(GIULIA, [member(GIULIA, 'Giulia')], undefined)).toBe('Giulia')
    expect(resolveName(GIULIA, undefined, undefined)).toBe(t.expense.memberUnknown)
  })

  it('propone a chi crea il gruppo il proprio nome fra i residenti', () => {
    expect(ownName(GIULIA, residents)).toBe('Giulia Bianchi')
    // Chi non compare fra i residenti — un proprietario che abiti in una delle
    // proprie case — lo scrive a mano invece di restare senza nome.
    expect(ownName(MARCO, residents)).toBe('')
    expect(ownName(null, residents)).toBe('')
  })
})
