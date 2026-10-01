import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { LeaseDetail, LeaseParty } from '@/api/types'
import { SessionProvider } from '@/app/session'
import { LeaseDetailPage } from './LeaseDetailPage'

/**
 * La scheda del contratto, dal lato del subentro (ADR-056).
 *
 * Il bottone si offre solo dove il dominio lo ammette, le quote partono **com'erano**
 * — chi entra con quella di chi esce — e su un contratto in essere l'invito non ha
 * più un campo email libero: un'email nuova creava una parte a quota zero, ed era la
 * seconda porta di ADR-046.
 */

const LEASE = '00000000-0000-4000-8000-00000000001e'
const ORG = '00000000-0000-4000-8000-0000000000ff'
const ANNA = '00000000-0000-4000-8000-0000000000a1'
const MARCO = '00000000-0000-4000-8000-0000000000a2'

function party(overrides: Partial<LeaseParty>): LeaseParty {
  return {
    id: crypto.randomUUID(),
    lease_id: LEASE,
    role: 'tenant',
    full_name: 'Parte',
    share_percent: '0.00',
    is_primary: false,
    joined_at: null,
    left_at: null,
    ...overrides,
  }
}

const anna = party({
  id: ANNA,
  full_name: 'Anna Bianchi',
  email: 'anna@example.com',
  user_id: '00000000-0000-4000-8000-0000000000b1',
  share_percent: '60.00',
  is_primary: true,
  joined_at: '2026-01-10',
})
const marco = party({
  id: MARCO,
  full_name: 'Marco Neri',
  email: 'marco@example.com',
  role: 'cotenant',
  share_percent: '40.00',
})

function lease(overrides: Partial<LeaseDetail>): LeaseDetail {
  return {
    id: LEASE,
    organization_id: ORG,
    property_id: '00000000-0000-4000-8000-0000000000c1',
    lease_type: 'whole_property',
    status: 'active',
    effective_status: 'active',
    start_date: '2026-01-01',
    end_date: '2030-01-01',
    notice_months: null,
    auto_renew: false,
    renewal_months: null,
    rent_amount: '800.00',
    currency: 'EUR',
    rent_frequency: 'monthly',
    rent_due_day: 5,
    deposit_amount: '0.00',
    utilities_included: false,
    istat_indexation: false,
    property_label: 'Casa di via Roma',
    parties: [anna, marco],
    ...overrides,
  } as LeaseDetail
}

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

function renderPage(detail: LeaseDetail) {
  const requests: { url: string; body: unknown }[] = []
  vi.stubGlobal('fetch', async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)
    if (url.endsWith('/auth/refresh')) {
      return json({
        access_token: 'token',
        token_type: 'Bearer',
        expires_in: 900,
        user_id: '00000000-0000-4000-8000-0000000000d1',
        organization_id: ORG,
        role: 'owner',
      })
    }
    if (url.endsWith('/handover')) {
      requests.push({ url, body: JSON.parse(String(init?.body)) })
      return json({
        lease: detail,
        invitation: {
          id: '00000000-0000-4000-8000-0000000000e1',
          organization_id: ORG,
          lease_id: LEASE,
          email: 'giulia@example.com',
          role: 'tenant',
          expires_at: '2026-10-05T10:00:00Z',
          accept_token: 'codice-di-giulia',
        },
      })
    }
    if (url.endsWith(`/leases/${LEASE}`)) return json(detail)
    return json({ items: [] })
  })

  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
  render(
    <MemoryRouter initialEntries={[`/contratti/${LEASE}`]}>
      <SessionProvider>
        <QueryClientProvider client={queryClient}>
          <Routes>
            <Route path="/contratti/:id" element={<LeaseDetailPage />} />
          </Routes>
        </QueryClientProvider>
      </SessionProvider>
    </MemoryRouter>,
  )
  return requests
}

async function openHandover(user: ReturnType<typeof userEvent.setup>) {
  await user.click(await screen.findByRole('button', { name: 'Subentro' }))
  return screen.findByRole('dialog')
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('il subentro, sulla scheda del contratto', () => {
  it('non si offre su un contratto in bozza, dove le parti si aggiungono', async () => {
    renderPage(lease({ status: 'draft', effective_status: 'draft' }))

    expect(await screen.findByText('Anna Bianchi')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Subentro' })).not.toBeInTheDocument()
  })

  it('si offre sul contratto in essere, accanto a «Disdici»', async () => {
    renderPage(lease({}))

    expect(await screen.findByRole('button', { name: 'Subentro' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Disdici' })).toBeInTheDocument()
  })

  it('propone le quote com’erano: a chi entra quella di chi esce', async () => {
    const user = userEvent.setup()
    renderPage(lease({}))
    const dialog = await openHandover(user)

    await user.selectOptions(within(dialog).getByLabelText(/Chi esce/), ANNA)
    expect(within(dialog).getByLabelText(/Quota di chi entra/)).toHaveValue('60.00')
    expect(within(dialog).getByLabelText('Marco Neri')).toHaveValue('40.00')

    // Cambiando chi esce, la proposta lo segue.
    await user.selectOptions(within(dialog).getByLabelText(/Chi esce/), MARCO)
    expect(within(dialog).getByLabelText(/Quota di chi entra/)).toHaveValue('40.00')
    expect(within(dialog).getByLabelText('Anna Bianchi')).toHaveValue('60.00')
  })

  it('conferma nominando chi esce e chi entra, e manda le quote di chi resta', async () => {
    const user = userEvent.setup()
    const requests = renderPage(lease({}))
    const dialog = await openHandover(user)

    await user.selectOptions(within(dialog).getByLabelText(/Chi esce/), ANNA)
    await user.type(within(dialog).getByLabelText(/Nome e cognome/), 'Giulia Rossi')
    await user.type(within(dialog).getByLabelText(/Email/), 'giulia@example.com')
    await user.clear(within(dialog).getByLabelText('Marco Neri'))
    await user.type(within(dialog).getByLabelText('Marco Neri'), '50')
    await user.clear(within(dialog).getByLabelText(/Quota di Giulia Rossi/))
    await user.type(within(dialog).getByLabelText(/Quota di Giulia Rossi/), '50')
    await user.click(within(dialog).getByRole('button', { name: 'Continua' }))

    expect(
      await screen.findByText(/Anna Bianchi esce dal contratto oggi, e Giulia Rossi riceve/),
    ).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Conferma il subentro' }))

    expect(await screen.findByText('codice-di-giulia')).toBeInTheDocument()
    expect(requests).toEqual([
      {
        url: expect.stringContaining(`/leases/${LEASE}/handover`),
        body: {
          leaving_party_id: ANNA,
          joining: { full_name: 'Giulia Rossi', email: 'giulia@example.com', share_percent: '50' },
          shares: [{ party_id: MARCO, share_percent: '50' }],
        },
      },
    ])
  })
})

describe('le parti, dopo un subentro', () => {
  it('dicono chi è fuori e da quando, e chi è in attesa di accettare', async () => {
    renderPage(
      lease({
        parties: [
          { ...anna, is_primary: false, left_at: '2026-09-28' },
          party({ full_name: 'Giulia Rossi', email: 'giulia@example.com', share_percent: '60.00', is_primary: true }),
          { ...marco, user_id: '00000000-0000-4000-8000-0000000000b2' },
        ],
      }),
    )

    expect(await screen.findByText(/fuori dal 28\/09\/2026/)).toBeInTheDocument()
    expect(screen.getByText(/in attesa di accettazione/)).toBeInTheDocument()
  })
})

describe('l’invito su un contratto in essere', () => {
  it('raggiunge solo chi è già scritto sul contratto, senza un’email libera', async () => {
    const user = userEvent.setup()
    renderPage(lease({}))
    await user.click(await screen.findByRole('button', { name: 'Invita un inquilino' }))
    const dialog = await screen.findByRole('dialog')

    expect(within(dialog).queryByRole('textbox', { name: /Email/ })).not.toBeInTheDocument()
    const choice = within(dialog).getByLabelText(/Chi inviti/)
    // Marco è sul contratto senza account; Anna ha già accettato e non si reinvita.
    expect(within(choice).getByRole('option', { name: /Marco Neri/ })).toBeInTheDocument()
    expect(within(choice).queryByRole('option', { name: /Anna Bianchi/ })).not.toBeInTheDocument()
  })
})
