import type { GroupBalance } from '@/api/types'
import { isNegative, isPositive, isZero, sumAmounts } from '@/lib/decimal'
import { formatAmount } from '@/lib/format'

/**
 * Un netto, con il verso leggibile **anche senza il colore**.
 *
 * Il numero si mostra com'è, segno compreso; a dire da che parte sta è la parola
 * accanto, che chi legge con un lettore di schermo sente e chi non distingue i
 * colori vede. Il colore aiuta e non porta l'informazione da solo.
 */
export function NetAmount({
  value,
  currency,
  display,
}: {
  value: string
  currency?: string
  /** Che cosa scrivere, quando non è `value`: serve dove il verso è già in una
   *  parola accanto e il segno lo contraddirebbe leggendolo. Il **tono** resta
   *  quello del valore vero. */
  display?: string
}) {
  const tone = isPositive(value) ? 'text-success' : isNegative(value) ? 'text-danger' : 'text-ink'
  return (
    <span className={tone}>{formatAmount(display ?? value, currency ? { currency } : {})}</span>
  )
}

/**
 * Se i netti di un gruppo sommano a zero, **verificato sugli interi**.
 *
 * È l'invariante che rende leggibili questi numeri, e dalla migration `0027` vale
 * anche quando qualcuno è uscito con un conto aperto: la vista lo tiene finché il
 * suo netto non è zero, altrimenti il suo debito sparirebbe con lui e il credito
 * di chi aveva anticipato resterebbe senza contropartita.
 *
 * `parseFloat` è vietato e qui si vedrebbe subito perché: sommare `"66.67"` e
 * `"-66.67"` come binari dà uno zero che a volte non è zero, e questa riga
 * direbbe che i conti non tornano su un gruppo in cui tornano.
 */
export function balancesSumToZero(items: readonly GroupBalance[]): boolean {
  return isZero(sumAmounts(items.map((item) => item.net_balance)))
}
