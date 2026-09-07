import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { SessionProvider } from '@/app/session'
import { ExpenseGroupDetailPage } from './ExpenseGroupDetailPage'

/**
 * La scheda di un gruppo, dal caso per cui questa fetta esiste.
 *
 * **Un coinquilino esce senza saldare.** Il suo debito continua a comparire nei
 * saldi che leggono gli altri — è la migration `0027`, senza la quale se lo
 * porterebbe via e la somma smetterebbe di fare zero — e deve comparire **con il
 * suo nome**: non è più fra i residenti della casa, quindi l'unico posto in cui
 * quel nome esiste è la riga della sua membership, scritta quando è entrato.
 *
 * È la verifica che chiude la fetta, ridotta a ciò che si può provare senza due
 * account: che la schermata non mostri un identificatore, e che dica perché quella
 * persona è nei saldi e non fra i membri attivi.
 */

const GROUP = '00000000-0000-4000-8000-0000000000f1'
const PROPERTY = '00000000-0000-4000-8000-0000000000f2'
const GIULIA = '00000000-0000-4000-8000-00000000000a'
const MARCO = '00000000-0000-4000-8000-00000000000b'

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
        user_id: GIULIA,
        organization_id: '00000000-0000-4000-8000-0000000000ff',
        role: 'tenant',
      })
    }

    if (url.includes('/balances')) {
      // I creditori per primi, come li manda l'API: il client non riordina.
      return json({
        items: [
          {
            user_id: GIULIA,
            total_paid: '100.00',
            total_owed: '50.00',
            total_settled_out: '0.00',
            total_settled_in: '0.00',
            net_balance: '50.00',
          },
          {
            user_id: MARCO,
            total_paid: '0.00',
            total_owed: '50.00',
            total_settled_out: '0.00',
            total_settled_in: '0.00',
            net_balance: '-50.00',
          },
        ],
      })
    }
    if (url.includes('/expenses')) return json({ items: [] })
    if (url.includes('/settlements')) return json({ items: [] })
    // Marco ha lasciato la casa: fra i residenti non c'è più, ed è precisamente
    // la ragione per cui il nome si scrive all'ingresso.
    if (url.includes('/residents')) {
      return json({ items: [{ user_id: GIULIA, full_name: 'Giulia Bianchi' }] })
    }

    if (url.includes(`/expense-groups/${GROUP}`)) {
      return json({
        id: GROUP,
        organization_id: '00000000-0000-4000-8000-0000000000ff',
        property_id: PROPERTY,
        kind: 'coliving',
        name: 'Casa di via Roma',
        currency: 'EUR',
        default_split_method: 'equal',
        is_archived: false,
        members: [
          {
            user_id: GIULIA,
            display_name: 'Giulia Bianchi',
            weight: '1.000',
            joined_at: '2026-01-10T09:00:00Z',
            left_at: null,
          },
          {
            user_id: MARCO,
            display_name: 'Marco Neri',
            weight: '1.000',
            joined_at: '2026-01-10T09:00:00Z',
            left_at: '2026-08-31T18:00:00Z',
          },
        ],
        created_at: '2026-01-10T09:00:00Z',
        updated_at: '2026-01-10T09:00:00Z',
      })
    }

    return json({})
  })

  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })

  render(
    <MemoryRouter initialEntries={[`/spese/${GROUP}`]}>
      <SessionProvider>
        <QueryClientProvider client={queryClient}>
          <Routes>
            <Route path="/spese/:id" element={<ExpenseGroupDetailPage />} />
          </Routes>
        </QueryClientProvider>
      </SessionProvider>
    </MemoryRouter>,
  )
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('i saldi di un gruppo con un membro uscito', () => {
  it('mostra il debito di chi è uscito con il suo nome, e non con il suo identificatore', async () => {
    renderPage()

    // Compare due volte: fra i saldi e fra i membri. Se il nome si risolvesse dai
    // residenti non comparirebbe in nessuno dei due, perché Marco non abita più
    // lì — ed è il difetto che si vedrebbe solo sui gruppi vecchi.
    expect((await screen.findAllByText('Marco Neri')).length).toBeGreaterThan(0)
    expect(screen.queryByText(MARCO)).not.toBeInTheDocument()
  })

  it('dice perché quella persona è nei saldi e non fra i membri attivi', async () => {
    renderPage()

    // Un nome nei saldi che non è fra i membri attivi è la prima domanda che chi
    // guarda si fa: senza la nota sembrerebbe un errore.
    expect(await screen.findByText(/uscito, conto ancora aperto/i)).toBeInTheDocument()
  })

  it('verifica che i netti sommino a zero', async () => {
    renderPage()

    expect(await screen.findByText(/la somma dei saldi fa zero/i)).toBeInTheDocument()
  })
})
