import { activeLocale, intlLocale, type Locale } from '@/i18n'

/**
 * Date, importi e numeri passano da `Intl`, con la **lingua come parametro**.
 *
 * Formattarli a mano — `dd/mm/yyyy`, `€ 184,20` — è la scorciatoia che rende
 * italiana anche la versione inglese: il giorno della traduzione i testi si
 * spostano in un file e le date restano dov'erano. Per lo stesso motivo qui non
 * c'è un default implicito sulla lingua del browser: la lingua è quella
 * dell'interfaccia, non quella del sistema di chi guarda.
 *
 * # Perché gli importi si formattano da una **stringa**
 *
 * `Intl.NumberFormat.prototype.format` accetta una stringa numerica e la formatta
 * senza passare da un double. È ciò che permette di mostrare `"184.20"` così
 * com'è: convertirlo con `Number()` per darlo a `Intl` reintrodurrebbe
 * precisamente l'errore che le stringhe decimali dell'API esistono per evitare.
 */

function localeTag(locale: Locale): string {
  return intlLocale[locale]
}

const currencyFormatters = new Map<string, Intl.NumberFormat>()

function currencyFormatter(locale: Locale, currency: string): Intl.NumberFormat {
  const key = `${locale}:${currency}`
  const existing = currencyFormatters.get(key)
  if (existing) return existing
  const formatter = new Intl.NumberFormat(localeTag(locale), {
    style: 'currency',
    currency,
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
  currencyFormatters.set(key, formatter)
  return formatter
}

/**
 * Un importo, dalla stringa decimale che l'API manda.
 *
 * `Intl` accetta la stringa direttamente. Il ramo di riserva esiste per un motore
 * che non lo supporti ancora, e in quel caso **preferisce mostrare il valore
 * grezzo** invece di arrotondarlo passando da un float: un numero senza simbolo di
 * valuta è brutto, uno sbagliato è peggio.
 */
export function formatAmount(
  raw: string,
  options: { currency?: string; locale?: Locale } = {},
): string {
  const { currency = 'EUR', locale = activeLocale } = options
  try {
    return currencyFormatter(locale, currency).format(raw as unknown as number)
  } catch {
    return `${raw} ${currency}`
  }
}

/** Un numero decimale senza valuta: metrature, consumi, quote. */
export function formatDecimal(
  raw: string,
  options: { locale?: Locale; maximumFractionDigits?: number } = {},
): string {
  const { locale = activeLocale, maximumFractionDigits = 4 } = options
  try {
    return new Intl.NumberFormat(localeTag(locale), { maximumFractionDigits }).format(
      raw as unknown as number,
    )
  } catch {
    return raw
  }
}

/**
 * La dimensione di un file, **da `Intl` come tutto il resto**.
 *
 * `size_bytes` è un intero e non un decimale dell'API: qui un `number` è il tipo
 * giusto, e la regola sui `parseFloat` non c'entra — non c'è nessuna stringa
 * decimale da preservare.
 *
 * L'unità si sceglie a soglie e la si passa a `Intl` con `style: 'unit'`, che la
 * localizza da sé: scrivere `` `${(n / 1048576).toFixed(1)} MB` `` produrrebbe
 * `1.4 MB` con il punto anche in italiano, che è esattamente il difetto che questo
 * modulo esiste per non avere. Le soglie sono binarie perché il limite del
 * caricamento è 25 MiB, cioè binario: dirlo in megabyte decimali farebbe leggere
 * `26,2 MB` a un file rifiutato per averne superati 25.
 */
export function formatBytes(bytes: number, locale: Locale = activeLocale): string {
  const [value, unit]: [number, string] =
    bytes >= 1024 * 1024
      ? [bytes / (1024 * 1024), 'megabyte']
      : bytes >= 1024
        ? [bytes / 1024, 'kilobyte']
        : [bytes, 'byte']
  try {
    return new Intl.NumberFormat(localeTag(locale), {
      style: 'unit',
      unit,
      unitDisplay: 'short',
      maximumFractionDigits: value < 10 && unit !== 'byte' ? 1 : 0,
    }).format(value)
  } catch {
    // Un motore senza `style: 'unit'`: meglio il numero senza unità che nessun
    // numero. Vale la stessa scelta di `formatAmount`.
    return new Intl.NumberFormat(localeTag(locale)).format(Math.round(value))
  }
}

/** Un intero: conteggi, numeri di locali. */
export function formatCount(value: number, locale: Locale = activeLocale): string {
  return new Intl.NumberFormat(localeTag(locale)).format(value)
}

/**
 * Una percentuale espressa come frazione: `"0.9412"` diventa `94,12 %`.
 *
 * È la confidenza dell'OCR, che ha quattro decimali. Non si moltiplica per cento
 * a mano — `Intl` con `style: 'percent'` lo fa senza passare da un'operazione
 * nostra su un float.
 */
export function formatFraction(raw: string, locale: Locale = activeLocale): string {
  try {
    return new Intl.NumberFormat(localeTag(locale), {
      style: 'percent',
      maximumFractionDigits: 1,
    }).format(raw as unknown as number)
  } catch {
    return raw
  }
}

/**
 * Una data ISO (`2026-08-31`) nel formato della lingua.
 *
 * **Si spezza a mano invece di passare da `new Date(raw)`**, e non è pedanteria:
 * `new Date('2026-08-31')` interpreta la stringa come mezzanotte UTC, e a ovest di
 * Greenwich la data mostrata diventa il 30. Una scadenza sbagliata di un giorno è
 * il genere di difetto che nessuno attribuisce al fuso orario.
 */
export function formatDate(raw: string | null | undefined, locale: Locale = activeLocale): string {
  if (!raw) return '—'
  const [year, month, day] = raw.slice(0, 10).split('-').map(Number)
  if (!year || !month || !day) return raw
  return new Intl.DateTimeFormat(localeTag(locale), {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
  }).format(new Date(year, month - 1, day))
}

/** Un istante (`TIMESTAMPTZ`) con l'ora, nel fuso di chi guarda. */
export function formatDateTime(
  raw: string | null | undefined,
  locale: Locale = activeLocale,
): string {
  if (!raw) return '—'
  const instant = new Date(raw)
  if (Number.isNaN(instant.getTime())) return raw
  return new Intl.DateTimeFormat(localeTag(locale), {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(instant)
}

/** La data di oggi in formato ISO, che è quello che l'API accetta. */
export function todayIso(): string {
  const now = new Date()
  const month = `${now.getMonth() + 1}`.padStart(2, '0')
  const day = `${now.getDate()}`.padStart(2, '0')
  return `${now.getFullYear()}-${month}-${day}`
}
