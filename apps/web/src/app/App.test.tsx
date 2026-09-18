import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { App } from './App'
import { SessionProvider } from './session'

/**
 * Il ripristino della sessione quando **la rete non c'è** (`R51`).
 *
 * Prima, un `refresh` che non arrivava al server era indistinguibile da «nessuna
 * sessione»: chi ricaricava in galleria vedeva il form di accesso, con il cookie
 * ancora buono e nessuna spiegazione. Sul telefono in 4G — il criterio di fine
 * del primo deploy — è il caso normale, non quello raro.
 *
 * Il test rende `App` intera, e non il solo `SessionProvider` con una sonda,
 * perché la cosa da provare è **che cosa si vede**: che il form non compaia, e
 * che al ritorno della rete un clic basti a entrare dove si stava.
 */

const ORG = '00000000-0000-4000-8000-000000000001'

interface Network {
  online: boolean
  refreshes: number
  /** Lascia rispondere il rinnovo tenuto in volo, aspettandolo se non è ancora partito. */
  release: () => Promise<void>
}

/**
 * `fetch` con la rete che può cadere e tornare.
 *
 * Il rinnovo che riesce resta **in volo** finché il test non lo rilascia: è
 * l'unico modo di guardare lo schermo *durante* la riprova, che è dove sta la
 * trappola — un `restoring` che non torna a `true` fa saltare dall'errore al
 * contenuto senza un caricamento in mezzo.
 */
function stubNetwork(): Network {
  let pending: (() => void) | null = null
  let arrived: (() => void) | null = null
  // Fra il clic e la `fetch` ci sono il lock fra le schede e il singolo volo:
  // rilasciare prima che la richiesta sia partita non rilascerebbe niente.
  const arrival = new Promise<void>((resolve) => {
    arrived = resolve
  })
  const network: Network = {
    online: false,
    refreshes: 0,
    release: async () => {
      await arrival
      pending?.()
    },
  }
  vi.stubGlobal('fetch', async (input: RequestInfo | URL) => {
    const url = String(input)
    if (url.endsWith('/auth/refresh')) {
      network.refreshes += 1
      // Quello che un browser lancia quando la richiesta non parte nemmeno.
      if (!network.online) throw new TypeError('Failed to fetch')
      await new Promise<void>((resolve) => {
        pending = resolve
        arrived?.()
      })
      return new Response(
        JSON.stringify({
          access_token: 'token',
          token_type: 'Bearer',
          expires_in: 900,
          user_id: 'utente-1',
          organization_id: ORG,
          role: 'owner',
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      )
    }
    if (url.endsWith('/organizations')) {
      return new Response(
        JSON.stringify({
          items: [{ id: ORG, name: 'Immobili di Mario Rossi', slug: 'immobili-4f21ab', role: 'owner' }],
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      )
    }
    return new Response('{}', { status: 200, headers: { 'content-type': 'application/json' } })
  })
  return network
}

beforeEach(() => {
  localStorage.clear()
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('il ripristino della sessione senza rete', () => {
  it('non manda al form di accesso: mostra l’errore, e «Riprova» entra quando la rete torna', async () => {
    const network = stubNetwork()
    const user = userEvent.setup()

    render(
      <MemoryRouter initialEntries={['/immobili']}>
        <SessionProvider>
          <App />
        </SessionProvider>
      </MemoryRouter>,
    )

    // L'errore, con la riprova — e **non** il form di accesso.
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('Non riesco a raggiungere il server')
    expect(screen.queryByRole('heading', { name: 'Entra in PropFlow' })).toBeNull()
    expect(screen.queryByLabelText(/email/i)).toBeNull()
    expect(network.refreshes).toBe(1)

    // Niente «Esci»: senza rete un logout non arriva al server.
    expect(screen.queryByRole('button', { name: 'Esci' })).toBeNull()

    // La rete torna. Un clic, e **prima** il caricamento: l'errore sparisce e al
    // suo posto c'è l'attesa, non un salto dall'uno al contenuto.
    network.online = true
    await user.click(screen.getByRole('button', { name: 'Riprova' }))
    expect(await screen.findByRole('status')).toHaveTextContent('Caricamento')
    expect(screen.queryByRole('alert')).toBeNull()

    // Poi il rinnovo risponde, e si è dentro, dove si stava andando.
    await network.release()
    await screen.findByRole('navigation', { name: 'Navigazione principale' })
    expect(network.refreshes).toBe(2)
    expect(screen.queryByRole('alert')).toBeNull()
    expect(screen.queryByRole('heading', { name: 'Entra in PropFlow' })).toBeNull()
  })

  it('un server che non c’è — il proxy risponde 502 — è lo stesso errore, non il form', async () => {
    // Con il proxy acceso e l'API spenta `fetch` **non** lancia: risponde il
    // proxy, con un `502` e un corpo che non è JSON. È ciò che un ricaricamento
    // durante un deploy incontra, ed è il caso che `R51` aveva visto davvero.
    vi.stubGlobal('fetch', async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url.endsWith('/auth/refresh')) {
        return new Response('<html>502 Bad Gateway</html>', {
          status: 502,
          headers: { 'content-type': 'text/html' },
        })
      }
      return new Response('{}', { status: 200, headers: { 'content-type': 'application/json' } })
    })

    render(
      <MemoryRouter initialEntries={['/immobili']}>
        <SessionProvider>
          <App />
        </SessionProvider>
      </MemoryRouter>,
    )

    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('Il server non risponde in questo momento')
    expect(screen.getByRole('button', { name: 'Riprova' })).toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: 'Entra in PropFlow' })).toBeNull()
  })

  it('un rifiuto del server resta «nessuna sessione»: il form di accesso compare', async () => {
    vi.stubGlobal('fetch', async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url.endsWith('/auth/refresh')) {
        return new Response(
          JSON.stringify({ status: 401, code: 'auth.token_missing', title: 'Unauthorized' }),
          { status: 401, headers: { 'content-type': 'application/problem+json' } },
        )
      }
      return new Response('{}', { status: 200, headers: { 'content-type': 'application/json' } })
    })

    render(
      <MemoryRouter initialEntries={['/immobili']}>
        <SessionProvider>
          <App />
        </SessionProvider>
      </MemoryRouter>,
    )

    await screen.findByRole('heading', { name: 'Entra in PropFlow' })
    expect(screen.queryByRole('alert')).toBeNull()
  })
})
