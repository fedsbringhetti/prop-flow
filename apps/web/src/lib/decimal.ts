/**
 * Gli importi sono stringhe, e restano stringhe.
 *
 * L'API li manda così apposta: `"184.20"` passato da un numero binario a virgola
 * mobile torna `184.19999999999999`, e in contabilità quella differenza si
 * accumula. `parseFloat` è quindi **vietato** su qualunque valore che arrivi
 * dall'API come stringa decimale — importi, percentuali, metrature, e la
 * confidenza dell'OCR, che ha quattro decimali.
 *
 * Qui dentro l'aritmetica passa da `bigint` su unità intere: due decimali per gli
 * importi, quattro per il consumo e per la confidenza. Nessuna divisione, perché
 * questa applicazione non ne fa: somma, confronta e mostra. Il giorno in cui
 * servisse ripartire un importo, la ripartizione con garanzia di somma la fa il
 * **server** — è il modulo delle spese co-living, che ha il vincolo differito che
 * lo garantisce.
 */

/** Un decimale a virgola fissa: `units` è il valore intero, `scale` i decimali. */
export interface FixedDecimal {
  units: bigint
  scale: number
}

const DECIMAL = /^-?\d+(\.\d+)?$/

/**
 * Legge una stringa decimale. `null` se non lo è.
 *
 * Non solleva: un valore non decimale può arrivare da un campo che l'utente sta
 * ancora digitando, e un'eccezione a ogni tasto renderebbe il form inutilizzabile.
 */
export function parseDecimal(raw: string): FixedDecimal | null {
  const value = raw.trim()
  if (!DECIMAL.test(value)) return null
  const [whole = '0', fraction = ''] = value.split('.')
  const negative = whole.startsWith('-')
  const digits = `${whole.replace('-', '')}${fraction}`
  const units = BigInt(digits)
  return { units: negative ? -units : units, scale: fraction.length }
}

/** Riporta due decimali alla stessa scala, per poterli sommare o confrontare. */
function align(a: FixedDecimal, b: FixedDecimal): [bigint, bigint, number] {
  const scale = Math.max(a.scale, b.scale)
  return [
    a.units * 10n ** BigInt(scale - a.scale),
    b.units * 10n ** BigInt(scale - b.scale),
    scale,
  ]
}

/** Rende la stringa canonica di un decimale, con la scala richiesta. */
export function toFixedString(value: FixedDecimal, scale: number): string {
  const scaled =
    scale >= value.scale
      ? value.units * 10n ** BigInt(scale - value.scale)
      : value.units / 10n ** BigInt(value.scale - scale)
  const negative = scaled < 0n
  const digits = (negative ? -scaled : scaled).toString().padStart(scale + 1, '0')
  const whole = digits.slice(0, digits.length - scale)
  const fraction = scale > 0 ? `.${digits.slice(digits.length - scale)}` : ''
  return `${negative ? '-' : ''}${whole}${fraction}`
}

/** Somma di importi, sempre a due decimali. Un valore non decimale è uno zero. */
export function sumAmounts(values: readonly string[]): string {
  let total: FixedDecimal = { units: 0n, scale: 2 }
  for (const raw of values) {
    const value = parseDecimal(raw)
    if (!value) continue
    const [a, b, scale] = align(total, value)
    total = { units: a + b, scale }
  }
  return toFixedString(total, 2)
}

/** `-1`, `0` o `1`. Confronta senza passare da un float. */
export function compareDecimals(left: string, right: string): number {
  const a = parseDecimal(left)
  const b = parseDecimal(right)
  if (!a || !b) return 0
  const [x, y] = align(a, b)
  return x < y ? -1 : x > y ? 1 : 0
}

/** Se il valore è strettamente maggiore di zero. */
export function isPositive(value: string): boolean {
  return compareDecimals(value, '0') > 0
}

/** Se il valore è strettamente minore di zero: un residuo negativo è un credito. */
export function isNegative(value: string): boolean {
  return compareDecimals(value, '0') < 0
}

/**
 * Il valore assoluto, **senza passare da un float**.
 *
 * Serve dove un debito va detto come quantità invece che come segno: `net_balance`
 * negativo è quanto una persona deve, e il campo in cui lo si sta per bonificare
 * chiede quel numero, non il suo opposto.
 */
export function absDecimal(raw: string): string {
  const value = parseDecimal(raw)
  if (!value) return raw
  return toFixedString(
    { units: value.units < 0n ? -value.units : value.units, scale: value.scale },
    value.scale,
  )
}

/** Se il valore è zero, con qualunque scala: `"0"`, `"0.00"` e `"0.0000"`. */
export function isZero(value: string): boolean {
  return compareDecimals(value, '0') === 0
}
