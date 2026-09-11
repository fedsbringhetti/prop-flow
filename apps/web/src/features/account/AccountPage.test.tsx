import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { SessionProvider } from '@/app/session'
import { AccountPage } from './AccountPage'

/**
 * Il proprio account, dai due punti che il piano segna come trappole.
 *
 * 1. **L'email si mostra e non si modifica**, con la riga che dice perché: un campo
 *    disabilitato senza spiegazione si legge come un difetto.
 * 2. **L'uscita chiede conferma dicendo che cosa resta**, e la propria
 *    organizzazione — di cui si è l'unico proprietario — non ha nemmeno il bottone,
 *    perché porterebbe a un `membership.last_owner`.
 *
 * Non si prova qui la revoca del cambio password: disconnette, e un test che la
 * seguisse fino in fondo verificherebbe il logout, non l'account. Si prova che la
 * conseguenza sia **scritta prima del bottone**, che è la cosa che si toglie
 * «migliorando».
 */

const ME = '00000000-0000-4000-8000-00000000000a'
const OWN_ORG = '00000000-0000-4000-8000-0000000000f1'
const RENTED_ORG = '00000000-0000-4000-8000-0000000000f2'

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

interface Call {
  method: string
  url: string
}

function renderPage(): Call[] {
  const calls: Call[] = []

  vi.stubGlobal('fetch', async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)
    const method = init?.method ?? 'GET'
    calls.push({ method, url })

    if (url.endsWith('/auth/refresh')) {
      return json({
        access_token: 'token',
        token_type: 'Bearer',
        expires_in: 900,
        user_id: ME,
        organization_id: OWN_ORG,
        role: 'owner',
      })
    }
    if (url.endsWith('/me')) {
      return json({
        id: ME,
        email: 'mario.rossi@example.test',
        full_name: 'Mario Rossi',
        locale: 'it-IT',
      })
    }
    if (url.endsWith('/organizations')) {
      return json({
        items: [
          { id: OWN_ORG, name: 'Immobili di Mario', slug: 'immobili-di-mario', role: 'owner' },
          { id: RENTED_ORG, name: 'Casa di Anna', slug: 'casa-di-anna', role: 'tenant' },
        ],
      })
    }
    if (url.includes('/leave')) {
      return json({ organization_id: RENTED_ORG, role: 'tenant' })
    }
    return json({}, 200)
  })

  render(
    <MemoryRouter>
      <SessionProvider>
        <QueryClientProvider
          client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
        >
          <AccountPage />
        </QueryClientProvider>
      </SessionProvider>
    </MemoryRouter>,
  )

  return calls
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('il proprio account', () => {
  it('mostra l’email disabilitata, con la riga che dice perché', async () => {
    renderPage()

    const email = (await screen.findByLabelText(/^email/i)) as HTMLInputElement
    expect(email.value).toBe('mario.rossi@example.test')
    expect(email).toBeDisabled()
    // La spiegazione, senza cui un campo spento sembra un guasto.
    expect(screen.getByText(/credenziale d’accesso/i)).toBeInTheDocument()
  })

  it('avverte della revoca prima del bottone del cambio password', async () => {
    renderPage()
    await screen.findByLabelText(/^email/i)

    expect(screen.getByText(/chiudi le sessioni aperte/i)).toBeInTheDocument()
  })

  it('esce da un’organizzazione altrui, e non offre l’uscita dalla propria', async () => {
    const user = userEvent.setup()
    const calls = renderPage()
    await screen.findByLabelText(/^email/i)

    // La propria — dove si è owner — compare senza il bottone «esci».
    expect(await screen.findByText('La tua organizzazione')).toBeInTheDocument()

    // Quella in cui si è inquilini ha il bottone: si preme, si conferma.
    await user.click(screen.getByRole('button', { name: /^esci$/i }))
    expect(await screen.findByText(/non vedrai più i contratti/i)).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: /sì, esci/i }))

    await waitFor(() =>
      expect(calls.some((c) => c.method === 'POST' && c.url.includes(`/organizations/${RENTED_ORG}/leave`))).toBe(
        true,
      ),
    )
  })
})
