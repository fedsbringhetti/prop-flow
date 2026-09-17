import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, within } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Dashboard } from '@/api/types'
import { SessionProvider } from '@/app/session'
import { t } from '@/i18n'
import { DashboardPage } from './DashboardPage'

/**
 * Il riepilogo vuoto è il **primo schermo di ogni proprietario nuovo**: tre gesti
 * nell'ordine in cui si fanno, e il rimando alla guida. Non c'è un ramo sul ruolo
 * qui dentro — il riepilogo è dello staff per rotta (`App.tsx`: per chi abita `/`
 * rimanda agli addebiti), e quel confine lo prova il test del guscio. Qui si prova
 * che cosa vede chi ci arriva.
 */
function dashboard(overrides: Partial<Dashboard> = {}): Dashboard {
  return {
    organization_id: 'org-1',
    properties_count: 0,
    active_leases: 0,
    monthly_rent_expected: '0.00',
    total_outstanding: '0.00',
    overdue_charges: 0,
    open_tickets: 0,
    leases_expiring_90d: 0,
    bills_pending_review: 0,
    written_off_total: '0.00',
    ...overrides,
  }
}

function renderPage(data: Dashboard) {
  vi.stubGlobal('fetch', async (input: RequestInfo | URL) => {
    const url = String(input)
    const body = url.endsWith('/auth/refresh')
      ? {
          access_token: 'token',
          token_type: 'Bearer',
          expires_in: 900,
          user_id: 'utente-1',
          organization_id: 'org-1',
          role: 'owner',
        }
      : data
    return new Response(JSON.stringify(body), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    })
  })

  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <MemoryRouter initialEntries={['/']}>
      <SessionProvider>
        <QueryClientProvider client={queryClient}>
          <DashboardPage />
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

describe('il riepilogo vuoto è il primo accesso', () => {
  it('racconta il giro in tre gesti, nell’ordine, e rimanda alla guida', async () => {
    renderPage(dashboard())

    expect(await screen.findByText(t.dashboard.empty)).toBeInTheDocument()
    expect(screen.getByText(t.dashboard.emptyHint)).toBeInTheDocument()

    // Tre passi in un elenco ordinato: l'ordine è il contenuto.
    const steps = within(screen.getByRole('list')).getAllByRole('link')
    expect(steps.map((step) => step.textContent)).toEqual([
      t.property.create,
      t.lease.create,
      t.dashboard.inviteStep,
    ])
    expect(steps.map((step) => step.getAttribute('href'))).toEqual([
      '/immobili/nuovo',
      '/contratti/nuovo',
      '/contratti',
    ])
    expect(screen.getByRole('link', { name: t.guide.read })).toHaveAttribute('href', '/guida')
  })

  it('con un immobile non è più vuoto: i numeri, anche a zero', async () => {
    renderPage(dashboard({ properties_count: 1 }))

    expect(await screen.findByText(t.dashboard.propertiesCount)).toBeInTheDocument()
    expect(screen.queryByText(t.dashboard.empty)).not.toBeInTheDocument()
    expect(screen.queryByRole('link', { name: t.guide.read })).not.toBeInTheDocument()
  })
})
