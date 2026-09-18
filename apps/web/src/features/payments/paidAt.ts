import { todayIso } from '@/lib/format'

/**
 * La data del movimento: **si mostra e non si manda**, finché resta quella di oggi.
 *
 * Sembra un cavillo ed è la sostanza di `R34`. Prima il campo partiva **vuoto**, e
 * un campo vuoto significa «adesso»: il server mette l'istante vero, ore e minuti
 * compresi. Precompilarlo *e mandarlo* trasformerebbe un pagamento delle 18 in uno
 * delle **00:00**, perché `<input type="date">` produce una data senza ora e
 * `new Date('2026-09-10').toISOString()` è la mezzanotte. Si guadagnerebbe una
 * comodità e si perderebbe l'ora vera — e in un elenco ordinato per data due
 * dichiarazioni dello stesso giorno diventerebbero indistinguibili.
 *
 * Quindi: il campo **mostra oggi**, perché chi guarda deve vedere che cosa sta per
 * dichiarare; la richiesta **tace** finché quella data non viene cambiata.
 *
 * Il campo svuotato a mano tace anch'esso, ed è lo stesso significato di prima.
 */
export function paidAtBody(date: string): { paid_at?: string } {
  if (!date || date === todayIso()) return {}
  return { paid_at: new Date(date).toISOString() }
}
