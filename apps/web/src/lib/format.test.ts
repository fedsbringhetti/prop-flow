import { describe, expect, it } from 'vitest'

import {
  compareDecimals,
  isNegative,
  isPositive,
  isZero,
  subtractAmounts,
  sumAmounts,
} from './decimal'
import { formatAmount, formatDate, formatFraction } from './format'

describe('gli importi non passano mai da un float', () => {
  /**
   * Il caso che dà il nome alla regola.
   *
   * `parseFloat('0.1') + parseFloat('0.2')` fa `0.30000000000000004`. Non è una
   * curiosità: è un centesimo che compare o sparisce, e in un elenco di addebiti
   * si accumula fino a diventare la ragione per cui due totali non tornano.
   */
  it('somma i decimali senza l’errore che l’aritmetica nativa introduce', () => {
    expect(0.1 + 0.2).not.toBe(0.3)
    expect(sumAmounts(['0.10', '0.20'])).toBe('0.30')
  })

  it('somma una colonna lunga restando esatta', () => {
    const importi = Array.from({ length: 100 }, () => '10.01')
    expect(sumAmounts(importi)).toBe('1001.00')
  })

  it('regge importi che un double non rappresenta più', () => {
    expect(sumAmounts(['999999999999.99', '0.01'])).toBe('1000000000000.00')
  })

  /**
   * Il residuo da riaddebitare si calcola così, e non con un `-`.
   *
   * `184.20 - 90` in aritmetica nativa dà `94.19999999999999`, e quel valore
   * finirebbe nel campo di un form e da lì nella richiesta.
   */
  it('sottrae senza l’errore che l’aritmetica nativa introduce', () => {
    expect(184.2 - 90).not.toBe(94.2)
    expect(subtractAmounts('184.20', '90.00')).toBe('94.20')
    expect(subtractAmounts('184.20', '90')).toBe('94.20')
  })

  /** Riaddebitare più del totale è possibile e voluto: il residuo va sotto zero. */
  it('lascia il residuo negativo invece di fermarlo a zero', () => {
    expect(subtractAmounts('100.00', '150.00')).toBe('-50.00')
  })

  it('confronta senza convertire', () => {
    expect(compareDecimals('9.90', '10.00')).toBe(-1)
    expect(compareDecimals('10.00', '10')).toBe(0)
    expect(compareDecimals('0.0000', '0.00')).toBe(0)
    expect(compareDecimals('-5.00', '0.00')).toBe(-1)
  })

  /** Un residuo negativo è un credito verso l'inquilino, e va riconosciuto. */
  it('riconosce il credito da sovrapagamento', () => {
    expect(isNegative('-120.00')).toBe(true)
    expect(isPositive('-120.00')).toBe(false)
    expect(isZero('0.00')).toBe(true)
    expect(isZero('0')).toBe(true)
  })

  it('ignora un valore che non è un decimale invece di sollevare', () => {
    expect(sumAmounts(['12.00', 'in corso di digitazione'])).toBe('12.00')
  })
})

describe('la formattazione riceve la lingua e non la indovina', () => {
  it('mostra un importo con le convenzioni italiane', () => {
    const formatted = formatAmount('184.20')
    expect(formatted).toContain('184,20')
    expect(formatted).toContain('€')
  })

  /**
   * `Intl` formatta la **stringa** senza passare da un double: è ciò che permette
   * di mostrare un importo lungo per intero. Se questo test cadesse, il ripiego
   * mostrerebbe il valore grezzo — brutto, ma non sbagliato.
   */
  it('formatta un importo che un double arrotonderebbe', () => {
    expect(formatAmount('999999999999.99')).toContain('999.999.999.999,99')
  })

  it('mostra la confidenza come percentuale, senza moltiplicare a mano', () => {
    expect(formatFraction('0.9412')).toContain('94,1')
    expect(formatFraction('0.4000')).toContain('40')
  })

  /**
   * `new Date('2026-08-31')` è mezzanotte **UTC**: a ovest di Greenwich la data
   * mostrata diventerebbe il 30. Una scadenza sbagliata di un giorno è il genere
   * di difetto che nessuno attribuisce al fuso orario.
   */
  it('non sposta una data di un giorno', () => {
    expect(formatDate('2026-08-31')).toBe('31/08/2026')
    expect(formatDate('2026-01-01')).toBe('01/01/2026')
  })

  it('mostra un trattino dove non c’è una data', () => {
    expect(formatDate(null)).toBe('—')
    expect(formatDate(undefined)).toBe('—')
  })
})
