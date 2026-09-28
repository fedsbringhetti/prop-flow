import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, within } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { SessionProvider } from '@/app/session'
import { NewPropertyPage } from './NewPropertyPage'

/**
 * `R44`: **il form di creazione offre lo stato**, e propone «Disponibile».
 *
 * Prima non lo offriva affatto: si atterrava sempre in bozza, lo stato si
 * cambiava solo dal dialogo «Modifica» della scheda — che non è dove si va a
 * cercarlo — e il primo segnale che qualcosa mancava era il rifiuto
 * dell'attivazione del primo contratto.
 *
 * È un test sul catalogo delle opzioni e sul valore proposto, non sulla rete:
 * `toEqual` su tutto l'elenco cade tanto per un'opzione in più — «Affittato», che
 * si calcola e non si sceglie (ADR-019) — quanto per una in meno.
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
    <MemoryRouter initialEntries={['/immobili/nuovo']}>
      <SessionProvider>
        <QueryClientProvider client={queryClient}>
          <NewPropertyPage />
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

describe('lo stato nel form di creazione di un immobile', () => {
  it('offre i tre stati che si scelgono, e non quelli che si calcolano', async () => {
    renderPage()
    const select = await screen.findByLabelText('Stato dichiarato')
    const options = within(select)
      .getAllByRole('option')
      .map((option) => option.textContent)
    expect(options).toEqual(['Bozza', 'Disponibile', 'In manutenzione'])
  })

  it('propone «Disponibile», che è lo stato in cui l’immobile serve a qualcosa', async () => {
    renderPage()
    const select = await screen.findByLabelText<HTMLSelectElement>('Stato dichiarato')
    expect(select.value).toBe('available')
  })

  it('dice che cosa comporta scegliere «Bozza», prima di sceglierlo', async () => {
    renderPage()
    expect(
      await screen.findByText(
        'Bozza è per chi sta ancora compilando: un immobile in bozza non si può affittare.',
      ),
    ).toBeInTheDocument()
  })
})
