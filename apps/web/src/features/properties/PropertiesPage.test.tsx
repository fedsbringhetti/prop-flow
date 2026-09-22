import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, within } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { SessionProvider } from '@/app/session'
import { PropertiesPage } from './PropertiesPage'

/**
 * `R26`: **un filtro offre solo ciò che può comparire.** «Archiviato» era
 * un'opzione con zero risultati per costruzione — archiviare scrive `deleted_at`
 * e l'elenco lo filtra — e chi la sceglieva non trovava l'immobile appena
 * archiviato e non capiva perché.
 *
 * È un test sul catalogo delle opzioni, non sulla rete: l'elenco risponde vuoto
 * e non conta. Conta che le opzioni siano **esattamente** queste, e nessun'altra:
 * `toEqual` su tutto l'elenco cade tanto per un'opzione in più quanto per una in
 * meno.
 */

function json(body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { 'content-type': 'application/json' },
  })
}

function renderPage() {
  vi.stubGlobal('fetch', async (input: RequestInfo | URL) => {
    const url = String(input)
    if (url.endsWith('/auth/refresh')) {
      return json({
        access_token: 'token',
        token_type: 'Bearer',
        expires_in: 900,
        user_id: 'utente-1',
        organization_id: 'org-1',
        role: 'owner',
      })
    }
    return json({ items: [] })
  })

  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })

  render(
    <MemoryRouter initialEntries={['/immobili']}>
      <SessionProvider>
        <QueryClientProvider client={queryClient}>
          <PropertiesPage />
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

describe('il filtro sullo stato degli immobili', () => {
  it('offre solo gli stati che un immobile in elenco può avere', async () => {
    renderPage()
    const select = await screen.findByLabelText('Stato')
    const options = within(select)
      .getAllByRole('option')
      .map((option) => option.textContent)
    expect(options).toEqual(['Tutti', 'Bozza', 'Disponibile', 'In manutenzione'])
  })
})
