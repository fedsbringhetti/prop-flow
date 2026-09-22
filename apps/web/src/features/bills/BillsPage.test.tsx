import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, within } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { SessionProvider } from '@/app/session'
import { BillsPage } from './BillsPage'

/**
 * `R26` sulle bollette: «Pagata» era un'opzione che nessuna transizione scrive —
 * `BillStatus::Paid` porta «mai scritto dall'applicazione», e la sezione 4 ha
 * dato alla bolletta una data, `paid_at`, non uno stato. Un filtro offre solo ciò
 * che può comparire.
 *
 * Come per gli immobili è un test sul catalogo, non sulla rete: `toEqual` su
 * tutto l'elenco cade per un'opzione in più come per una in meno.
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
    <MemoryRouter initialEntries={['/bollette']}>
      <SessionProvider>
        <QueryClientProvider client={queryClient}>
          <BillsPage />
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

describe('il filtro sullo stato delle bollette', () => {
  it('offre solo gli stati che una bolletta in elenco può avere', async () => {
    renderPage()
    const select = await screen.findByLabelText('Stato')
    const options = within(select)
      .getAllByRole('option')
      .map((option) => option.textContent)
    expect(options).toEqual(['Tutti', 'Bozza', 'Da rivedere', 'Validata', 'Riaddebitata'])
  })
})
