import { it, type Catalog } from './it'

/**
 * Le lingue disponibili. Oggi una; il tipo esiste perché il giorno in cui saranno
 * due nessuna firma debba cambiare.
 */
export type Locale = 'it'

const catalogs: Record<Locale, Catalog> = { it }

/**
 * La lingua dell'interfaccia.
 *
 * Costante finché è una sola. **Il punto non è che sia configurabile**, è che
 * ogni formattazione la riceva come parametro: `Intl.DateTimeFormat` e
 * `Intl.NumberFormat` senza lingua esplicita seguono quella del browser, e
 * un'interfaccia inglese che mostra `31/08/2026` e `184,20 €` è un'interfaccia
 * tradotta a metà.
 */
export const activeLocale: Locale = 'it'

/** Il catalogo della lingua attiva. */
export const t: Catalog = catalogs[activeLocale]

/**
 * La lingua nella forma che `Intl` si aspetta.
 *
 * `it-IT` e non `it`: la seconda lascia a `Intl` la scelta della regione, e con
 * essa il separatore decimale e la posizione del simbolo di valuta.
 */
export const intlLocale: Record<Locale, string> = { it: 'it-IT' }
