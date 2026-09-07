import { describe, expect, it } from 'vitest'

import type { GroupBalance } from '@/api/types'
import { balancesSumToZero } from './balance'

/**
 * La somma dei netti fa zero, e va verificata **sugli interi**.
 *
 * È l'invariante che rende leggibili i saldi di un gruppo, e dalla migration
 * `0027` vale anche quando qualcuno è uscito con un conto aperto. Il secondo caso
 * è la ragione per cui `parseFloat` è vietato: una bolletta di 122,80 € divisa fra
 * quattro coinquilini somma a zero esattamente, e in virgola mobile binaria no —
 * il client direbbe che i conti non tornano su un gruppo in cui tornano.
 */

function balance(userId: string, net: string): GroupBalance {
  return {
    user_id: userId,
    total_paid: '0.00',
    total_owed: '0.00',
    total_settled_out: '0.00',
    total_settled_in: '0.00',
    net_balance: net,
  }
}

describe('i saldi di un gruppo', () => {
  it('sommano a zero su una spesa divisa in due', () => {
    expect(balancesSumToZero([balance('a', '50.00'), balance('b', '-50.00')])).toBe(true)
  })

  it('sommano a zero su una bolletta divisa fra quattro, dove il float non ci arriva', () => {
    // 122,80 € anticipati da uno e divisi in quattro: 30,70 a testa, e il netto di
    // chi ha pagato è 92,10.
    const items = [
      balance('a', '92.10'),
      balance('b', '-30.70'),
      balance('c', '-30.70'),
      balance('d', '-30.70'),
    ]
    expect(balancesSumToZero(items)).toBe(true)
    // La prova che il modulo decimale serve, e non è un'ipotesi: la stessa somma
    // con l'aritmetica nativa produce uno zero che non è zero.
    expect(92.1 - 30.7 - 30.7 - 30.7).not.toBe(0)
  })

  it('vede la somma rotta quando manca una riga', () => {
    expect(balancesSumToZero([balance('a', '92.10'), balance('b', '-30.70')])).toBe(false)
  })

  it('è vera su un gruppo senza spese', () => {
    expect(balancesSumToZero([])).toBe(true)
  })
})
