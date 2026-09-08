import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Ticket, TicketStatus } from '@/api/types'
import { SessionProvider } from '@/app/session'
import { TicketActions } from './TicketActions'

/**
 * **«Prendi in carico» sparisce solo a chi ce l'ha.**
 *
 * La condizione è di quelle che regrediscono in silenzio: `assigned` resta fra gli
 * stati assegnabili **apposta**, perché il bottone serve anche a *riassegnare* a
 * qualcun altro. Chi passasse di qui per «togliere il bottone sulle assegnate»
 * toglierebbe la riassegnazione a tutti, e nessuno se ne accorgerebbe finché non
 * serve — cioè quando l'organizzazione ha due persone.
 *
 * Tre casi, e sono i tre che contano:
 *
 * 1. **aperta e di nessuno** → il bottone c'è;
 * 2. **assegnata a me** → non c'è, ed è la voce `R20`;
 * 3. **assegnata a qualcun altro** → c'è di nuovo, ed è la riassegnazione.
 */

const ME = 'utente-1'
const SOMEONE_ELSE = 'utente-2'

function ticketWith(status: TicketStatus, assignedTo: string | null): Ticket {
  return {
    id: 'segnalazione-1',
    organization_id: 'org-1',
    reference: 'TK-000001',
    property_id: 'immobile-1',
    title: 'Il rubinetto perde',
    description: 'Perde dal miscelatore.',
    category: 'plumbing',
    priority: 'medium',
    status,
    reported_by: 'utente-inquilino',
    reported_by_kind: 'tenant',
    assigned_to: assignedTo,
    assigned_at: assignedTo ? '2026-09-08T10:00:00Z' : null,
    estimated_cost: null,
    actual_cost: null,
    currency: 'EUR',
    closed_at: null,
    created_at: '2026-09-08T09:00:00Z',
    updated_at: '2026-09-08T09:00:00Z',
  }
}

/**
 * Il guscio minimo: una sessione di **staff**, perché è a lui che il bottone si
 * mostra. `auth/refresh` risponde con il ruolo `owner` e con `ME` come utente
 * corrente, che è il confronto su cui tutta la voce si regge.
 */
function renderActions(ticket: Ticket) {
  vi.stubGlobal('fetch', async (input: RequestInfo | URL) => {
    const url = String(input)
    if (url.endsWith('/auth/refresh')) {
      return new Response(
        JSON.stringify({
          access_token: 'token',
          token_type: 'Bearer',
          expires_in: 900,
          user_id: ME,
          organization_id: 'org-1',
          role: 'owner',
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      )
    }
    return new Response('{}', { status: 200, headers: { 'content-type': 'application/json' } })
  })

  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })

  render(
    <MemoryRouter>
      <SessionProvider>
        <QueryClientProvider client={queryClient}>
          <TicketActions ticket={ticket} />
        </QueryClientProvider>
      </SessionProvider>
    </MemoryRouter>,
  )
}

beforeEach(() => {
  localStorage.clear()
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('«Prendi in carico»', () => {
  it('compare su una segnalazione aperta che non ha nessuno', async () => {
    renderActions(ticketWith('open', null))
    expect(await screen.findByRole('button', { name: /prendi in carico/i })).toBeInTheDocument()
  })

  it('non compare su ciò che si ha già in carico', async () => {
    renderActions(ticketWith('assigned', ME))

    // Si aspetta che la sessione sia arrivata prima di concludere che manca: un
    // «non c'è» misurato mentre il guscio sta ancora leggendo il token passerebbe
    // anche con la condizione sbagliata.
    expect(await screen.findByRole('button', { name: /chiudi/i })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /prendi in carico/i })).not.toBeInTheDocument()
  })

  it('compare su ciò che è in carico a qualcun altro, perché è la riassegnazione', async () => {
    renderActions(ticketWith('assigned', SOMEONE_ELSE))
    expect(await screen.findByRole('button', { name: /prendi in carico/i })).toBeInTheDocument()
  })
})
