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
