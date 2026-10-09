import type { LeaseParty, LeasePartyRole } from '@/api/types'
import { compareDecimals, sumAmounts } from '@/lib/decimal'

/**
 * La quota che resta da assegnare, e chi la conta.
 *
 * **È la stessa aritmetica di `remaining_share` del server** (`invite_tenant.rs`),
 * ed è qui apposta: proponendo `100` a ogni parte, il form produceva il difetto
 * che ADR-046 descrive — il primo si prendeva tutto, all'invito successivo restava
 * zero, e nessuno lo diceva finché l'attivazione non rifiutava.
 *
 * Chi condivide il canone sono **inquilino e coinquilino**, non il garante: è
 * `LeasePartyRole::shares_rent` nel dominio, e le due definizioni devono
 * coincidere o il form proporrebbe numeri che l'attivazione rifiuta. Chi è uscito
 * (`left_at`) non conta, come nel dominio.
 *
 * L'aritmetica passa da `@/lib/decimal`: `parseFloat` è vietato, e `33.33` letto
 * come binario torna con una coda che si vedrebbe nel campo.
 */
const SHARES_RENT: readonly LeasePartyRole[] = ['tenant', 'cotenant']

export function assignedShare(parties: readonly LeaseParty[]): string {
  return sumAmounts(
    parties
      .filter((party) => SHARES_RENT.includes(party.role) && party.left_at === null)
      .map((party) => party.share_percent),
  )
}

/** `100 − assegnato`, con il minimo a zero come fa il server. */
export function remainingShare(parties: readonly LeaseParty[]): string {
  const assigned = assignedShare(parties)
  const remaining = sumAmounts(['100', `-${assigned}`])
  return compareDecimals(remaining, '0') < 0 ? '0.00' : remaining
}

/**
 * Se un intestatario principale c'è già.
 *
 * Il vincolo parziale `lease_parties_primary_uk` ne ammette **uno solo** per
 * contratto, e proporre la casella già spuntata alla seconda parte significava
 * offrire un `409` come default.
 */
export function hasPrimary(parties: readonly LeaseParty[]): boolean {
  return parties.some((party) => party.is_primary && party.left_at === null)
}

/** Chi è ancora nel contratto. Chi è uscito (`left_at`) resta fra le righe e non conta. */
export function stillOnLease(parties: readonly LeaseParty[]): LeaseParty[] {
  return parties.filter((party) => party.left_at === null)
}

/**
 * Le quote che il subentro propone: **com'erano** (ADR-056).
 *
 * Chi entra riceve la quota di chi esce, e chi resta tiene la sua. Sono i valori
 * con cui i campi **partono pieni** — non un `placeholder`, che è testo grigio in un
 * campo vuoto: confermando senza toccare niente si manderebbe niente. Si propongono
 * solo le quote di chi condivide il canone, come `SHARES_RENT`; il garante resta
 * fuori, com'è fuori dalla somma.
 *
 * `null` se chi esce non è fra chi è ancora nel contratto: il dialogo non ha niente
 * da proporre finché non lo si sceglie.
 */
export function proposedHandover(
  parties: readonly LeaseParty[],
  leavingId: string,
): { joining: string; staying: { party: LeaseParty; share_percent: string }[] } | null {
  const leaving = stillOnLease(parties).find((party) => party.id === leavingId)
  if (!leaving) return null
  return {
    joining: leaving.share_percent,
    staying: stillOnLease(parties)
      .filter((party) => party.id !== leavingId && SHARES_RENT.includes(party.role))
      .map((party) => ({ party, share_percent: party.share_percent })),
  }
}

/**
 * Chi si può invitare su un contratto che **non accetta più parti**.
 *
 * Da attivo l'invito raggiunge solo una parte che esiste, ancora senza account, e
 * con l'email a cui l'invito sarà intestato: un'email nuova creerebbe una parte, e
 * il server la rifiuta con `lease.parties_locked` (ADR-056). Per far entrare
 * qualcuno di nuovo c'è il subentro.
 */
export function invitableParties(parties: readonly LeaseParty[]): LeaseParty[] {
  return stillOnLease(parties).filter(
    (party) => party.user_id === undefined && party.email !== undefined,
  )
}
