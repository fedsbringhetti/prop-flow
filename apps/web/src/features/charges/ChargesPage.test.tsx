import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Charge } from '@/api/types'
import { SessionProvider } from '@/app/session'
import { ChargesPage } from './ChargesPage'

/**
 * Il giro in blocco su una schermata vera: tre addebiti selezionati, esiti diversi.
 *
 * È il criterio di fine della sezione 7 letto alla lettera — «si selezionano tre
 * addebiti di cui uno già annullato: due si annullano, il terzo sparisce anch'esso,
 * niente in rosso» — più il caso della rete che cade a metà: la riga non fatta
 * **resta selezionata** con il motivo accanto, e si riprova con un clic.
 *
 * Il finto server rifiuta **come rifiuta il vero**: `409 charge.invalid_transition`
 * sull'addebito già annullato, e una `fetch` che solleva per quello che la rete non
 * raggiunge. Dopo il giro l'elenco viene riletto, e le righe fatte non ci sono più.
 */

const ME = 'utente-1'

function charge(id: string, status: Charge['status'] = 'issued'): Charge {
  return {
    id,
    organization_id: 'org-1',
    lease_id: 'contratto-1',
    charge_type: 'rent',
    status,
    effective_status: status,
    period_start: null,
    period_end: null,
    amount: '800.00',
    paid_amount: '0.00',
    outstanding_amount: status === 'issued' ? '800.00' : '0.00',
    currency: 'EUR',
    due_date: '2026-09-30',
    issued_at: '2026-09-01T09:00:00Z',
    created_at: '2026-09-01T09:00:00Z',
    updated_at: '2026-09-01T09:00:00Z',
  }
}

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

function problem(status: number, code: string): Response {
  return new Response(JSON.stringify({ type: code, title: code, status, code }), {
    status,
    headers: { 'content-type': 'application/problem+json' },
  })
}

/**
 * Tre addebiti emessi, per come li vede la pagina. Sul server `beta` è **già
 * annullato** da qualcun altro — la pagina lo ha caricato prima — quindi al giro
 * risponde `409 charge.invalid_transition`; `gamma` non è raggiungibile. Alla
 * rilettura le righe restano tutte, come fa l'elenco vero: un addebito annullato
 * resta visibile, con il suo stato. A sparire è la **selezione**, non la riga.
 */
function renderPage() {
  const voids: string[] = []
  let listReads = 0
  const status = new Map<string, Charge['status']>([
    ['alfa', 'issued'],
    ['beta', 'void'],
    ['gamma', 'issued'],
  ])
  let unreachable = new Set(['gamma'])

  vi.stubGlobal('fetch', async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)
    if (url.endsWith('/auth/refresh')) {
      return json({
        access_token: 'token',
        token_type: 'Bearer',
        expires_in: 900,
        user_id: ME,
        organization_id: 'org-1',
        role: 'owner',
      })
    }
    const void_ = /\/charges\/([^/]+)\/void$/.exec(url)
    if (void_ && init?.method === 'POST') {
      const id = void_[1] ?? ''
      voids.push(id)
      if (unreachable.has(id)) throw new TypeError('Failed to fetch')
      if (status.get(id) !== 'issued') return problem(409, 'charge.invalid_transition')
      status.set(id, 'void')
      return json(charge(id, 'void'))
    }
    if (url.includes('/charges')) {
      listReads += 1
      // La prima lettura è la fotografia vecchia, in cui `beta` era ancora emesso.
      const stale = listReads === 1
      return json({
        items: [...status].map(([id, current]) =>
          charge(id, stale && id === 'beta' ? 'issued' : current),
        ),
      })
    }
    return json({})
  })

  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })

  render(
    <MemoryRouter initialEntries={['/addebiti']}>
      <SessionProvider>
        <QueryClientProvider client={queryClient}>
          <ChargesPage />
        </QueryClientProvider>
      </SessionProvider>
    </MemoryRouter>,
  )

  return {
    voids,
    reads: () => listReads,
    reconnect: () => {
      unreachable = new Set()
    },
  }
}

beforeEach(() => {
  // Il lock del rinnovo fra schede lascia un marcatore in `localStorage`: un
  // test che parte con quello del precedente aspetta che scada.
  localStorage.clear()
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('annullare tre addebiti in blocco', () => {
  it('due passano, «era già così» sparisce senza rosso, la rete caduta resta selezionata con il motivo', async () => {
    const user = userEvent.setup()
    const api = renderPage()

    await user.click(await screen.findByLabelText('Seleziona tutte le righe caricate'))
    expect(screen.getByText('3 selezionate su 3 caricate')).toBeInTheDocument()

    const bar = screen.getByRole('region', { name: 'Righe selezionate' })
    await user.click(within(bar).getByRole('button', { name: /annulla/i }))
    const dialog = await screen.findByRole('dialog')
    expect(within(dialog).getByText('Annullare 3 addebiti?')).toBeInTheDocument()
    // Non «Annulla» a sinistra e «Annulla» a destra: il bottone che non fa
    // niente dice un'altra cosa.
    expect(within(dialog).getByRole('button', { name: 'Lascia com’è' })).toBeInTheDocument()
    await user.click(within(dialog).getByRole('button', { name: 'Annulla' }))

    // Il giro è passato su tutte e tre, nell'ordine della tabella, una volta
    // ciascuna: né fermo al primo errore, né ritentato.
    await waitFor(() => expect(api.voids).toEqual(['alfa', 'beta', 'gamma']))

    // Le due righe fatte — o già così — sono uscite dalla selezione e ora sono
    // annullate; `gamma` resta selezionata, con il motivo accanto, e `beta` non è
    // mai stata rossa: un solo motivo in tutta la tabella.
    await waitFor(() => expect(screen.getByText('1 selezionate su 1 caricate')).toBeInTheDocument())
    expect(within(screen.getByRole('table')).getAllByText('Annullato')).toHaveLength(2)
    expect(screen.getByRole('alert')).toHaveTextContent('2 fatte. 1 non fatte')
    const reasons = screen.getAllByRole('status')
    expect(reasons).toHaveLength(1)
    expect(reasons[0]).toHaveTextContent(/non riesco a raggiungere il server/i)

    // L'elenco è stato riletto **una** volta dopo il giro, non tre.
    expect(api.reads()).toBe(2)

    // Torna la rete: si riprova con un clic, senza riselezionare niente.
    api.reconnect()
    await user.click(within(bar).getByRole('button', { name: /annulla/i }))
    await user.click(within(await screen.findByRole('dialog')).getByRole('button', { name: 'Annulla' }))
    await waitFor(() => expect(api.voids).toEqual(['alfa', 'beta', 'gamma', 'gamma']))
    await waitFor(() => expect(screen.queryByRole('region', { name: 'Righe selezionate' })).not.toBeInTheDocument())
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })

  it('la riga singola passa dallo stesso giro, con la conferma al singolare', async () => {
    const user = userEvent.setup()
    const api = renderPage()

    // Il bottone per riga porta il nome della riga nell'etichetta accessibile.
    const perRow = await screen.findAllByRole('button', { name: /^annulla — /i })
    expect(perRow).toHaveLength(3)
    await user.click(perRow[0] as HTMLElement)
    const dialog = await screen.findByRole('dialog')
    expect(within(dialog).getByText('Annullare questo addebito?')).toBeInTheDocument()
    await user.click(within(dialog).getByRole('button', { name: 'Annulla' }))

    await waitFor(() => expect(api.voids).toEqual(['alfa']))
    await waitFor(() =>
      expect(within(screen.getByRole('table')).getAllByText('Annullato')).toHaveLength(2),
    )
  })
})
