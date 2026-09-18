import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
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
    visible_to_residents: false,
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
function renderActions(ticket: Ticket): { calls: string[] } {
  // Le rotte chiamate, per poter dire che una **non** lo è stata: `vi.stubGlobal`
  // con una funzione qualunque non produce una spia, e `toHaveBeenCalledWith`
  // fallirebbe dicendo che l'oggetto non è una spia invece di dire cosa è
  // successo.
  const calls: string[] = []
  vi.stubGlobal('fetch', async (input: RequestInfo | URL) => {
    const url = String(input)
    calls.push(url)
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

  return { calls }
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

/**
 * **Riaprire chiede il motivo, e non è un campo di comodo.**
 *
 * Da quando un thread chiuso non riceve più messaggi, riaprire è l'unica via per
 * dire «non è risolto»: senza il motivo non resterebbe scritto da nessuna parte
 * perché la segnalazione è tornata viva, e l'API risponde `422`.
 *
 * Il test verifica che il bottone **apra un form** e non esegua la riapertura al
 * primo clic: la regressione da temere è che qualcuno lo riporti a una conferma
 * secca, che chiamerebbe l'API con un corpo vuoto e prenderebbe un `422` che
 * l'utente non sa come risolvere.
 */
describe('la riapertura', () => {
  it('chiede il motivo invece di riaprire al primo clic', async () => {
    const user = userEvent.setup()
    const { calls } = renderActions(ticketWith('closed', ME))

    await user.click(await screen.findByRole('button', { name: /^riapri$/i }))

    expect(await screen.findByLabelText(/perché la riapri/i)).toBeInTheDocument()
    expect(calls.some((url) => url.includes('/reopen'))).toBe(false)
  })
})

/**
 * **La correzione ha la stessa condizione dell'annullamento.**
 *
 * Non è una coincidenza da semplificare: nell'API è la stessa funzione — lo staff
 * sempre, chi ha segnalato finché nessuno ci ha messo mano — e due condizioni
 * scritte due volte sono due regole che divergeranno.
 */
describe('«Correggi»', () => {
  it('compare allo staff su una chiusa, perché un titolo sbagliato lì lo resta per sempre', async () => {
    renderActions(ticketWith('closed', ME))
    expect(await screen.findByRole('button', { name: /^correggi$/i })).toBeInTheDocument()
    // Accanto ai costi, che su una chiusa ci sono anch'essi: la fattura del
    // fornitore arriva dopo. È la coppia che dice che la condizione è quella
    // giusta e non «solo finché è in lavorazione».
    expect(screen.getByRole('button', { name: /modifica i costi/i })).toBeInTheDocument()
  })

  it('non compare su una annullata, che non si tocca più — e non compare nemmeno il resto', async () => {
    renderActions(ticketWith('cancelled', null))

    // Su una annullata `TicketActions` non offre **niente**: nessuna transizione
    // parte da lì e nemmeno i costi si toccano (`409 ticket.cancelled`). Non c'è
    // quindi un bottone a cui appendersi per aspettare la sessione, e si aspetta
    // che l'unica cosa che la sessione potrebbe far comparire continui a mancare.
    await waitFor(() => {
      expect(screen.queryByRole('button', { name: /^correggi$/i })).not.toBeInTheDocument()
    })
    expect(screen.queryAllByRole('button')).toHaveLength(0)
  })
})
