import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useEffect, useMemo, type ReactNode } from 'react'

import { ApiError } from '@/api/problem'
import { useSession } from './session'

/**
 * Le regole di rilettura, decise una volta.
 *
 * # Non si riprova ciò che non cambierà
 *
 * Un `404`, un `403` o un `422` non diventano veri riprovando: ritentarli
 * moltiplica il carico e allunga l'attesa prima di mostrare l'errore. Si riprova
 * ciò che è transitorio — un `5xx`, una rete caduta — e una volta sola, perché la
 * seconda dice quasi sempre la stessa cosa della prima.
 *
 * # Il `401` non si riprova **qui**
 *
 * Lo gestisce il client dell'API, che rinnova la sessione e riesegue la richiesta.
 * Un secondo ritentativo a questo livello produrrebbe una seconda chiamata di
 * refresh — cioè, con l'API che ruota i token, il modo più rapido di far revocare
 * la catena. È lo stesso motivo per cui il singolo volo esiste.
 */
function shouldRetry(failureCount: number, error: unknown): boolean {
  if (failureCount >= 1) return false
  if (!(error instanceof ApiError)) return false
  if (error.isOffline) return true
  return error.status >= 500
}

export function QueryProvider({ children }: { children: ReactNode }) {
  const { store } = useSession()

  const client = useMemo(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            retry: shouldRetry,
            // Trenta secondi: un gestionale si guarda mentre si lavora, e
            // rileggere a ogni ritorno sulla scheda fa lampeggiare elenchi che
            // non sono cambiati. Le mutazioni invalidano esplicitamente ciò che
            // hanno toccato, quindi la freschezza non dipende da questo numero.
            staleTime: 30_000,
            refetchOnWindowFocus: false,
          },
          mutations: { retry: false },
        },
      }),
    [],
  )

  // Quando la sessione cade, la cache va svuotata: i dati di chi è uscito non
  // devono comparire a chi entra dopo sullo stesso browser.
  useEffect(
    () =>
      store.subscribe(() => {
        if (store.current === null) client.clear()
      }),
    [store, client],
  )

  return <QueryClientProvider client={client}>{children}</QueryClientProvider>
}

/**
 * Le chiavi delle query, in un posto solo.
 *
 * Sparse fra i componenti diventano stringhe che si scrivono a memoria, e una
 * invalidazione che sbaglia una chiave non fallisce: mostra dati vecchi, che è
 * l'errore più difficile da vedere.
 */
export const keys = {
  /** Le proprie organizzazioni. **Non dipende da quella attiva**: è l'unica
   *  lettura che non appartenga a nessuna di esse. */
  organizations: ['organizations'] as const,
  dashboard: ['dashboard'] as const,
  properties: (filters?: unknown) => ['properties', filters ?? null] as const,
  property: (id: string) => ['property', id] as const,
  rooms: (propertyId: string) => ['rooms', propertyId] as const,
  residents: (propertyId: string) => ['residents', propertyId] as const,
  leases: (filters?: unknown) => ['leases', filters ?? null] as const,
  lease: (id: string) => ['lease', id] as const,
  bills: (filters?: unknown) => ['bills', filters ?? null] as const,
  bill: (id: string) => ['bill', id] as const,
  billOcr: (id: string) => ['bill-ocr', id] as const,
  documents: (ownerType: string, ownerId: string) => ['documents', ownerType, ownerId] as const,
  charges: (filters?: unknown) => ['charges', filters ?? null] as const,
  charge: (id: string) => ['charge', id] as const,
  /** La collezione: la coda delle dichiarazioni ne è il filtro `pending`. */
  payments: (filters?: unknown) => ['payments', filters ?? null] as const,
  /** I movimenti di un addebito. Chiave distinta dalla collezione, perché
   *  un'invalidazione dell'una non deve rileggere l'altra. */
  chargePayments: (chargeId: string) => ['charge-payments', chargeId] as const,
}
