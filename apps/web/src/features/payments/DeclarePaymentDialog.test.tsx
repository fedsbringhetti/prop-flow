import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router'
import { describe, expect, it, vi } from 'vitest'

import { todayIso } from '@/lib/format'

import { SessionProvider } from '@/app/session'
import type { Charge } from '@/api/types'
import { DeclarePaymentDialog } from './DeclarePaymentDialog'

/**
 * La dichiarazione di pagamento, dai lati che si rompono in silenzio.
 *
 * È la stessa forma della registrazione di un incasso, e i test sono gemelli
 * apposta: le due rotte producono righe diverse ma condividono l'unica cosa il cui
 * difetto non si vede, cioè **la chiave di idempotenza generata all'intento e
 * riusata a ogni tentativo**. Se cambiasse, un doppio invio scriverebbe due
 * dichiarazioni, e il proprietario si troverebbe due righe da confermare per un
 * bonifico solo.
 *
 * Il resto verifica ciò che è **proprio** di questa rotta: il corpo non porta
 * `payer_user_id` — la policy esige che sia chi chiama, e passarlo darebbe zero
 * righe inserite — e il form dice, prima dell'invio, che ritirare non si può.
 */

const charge: Charge = {
  id: 'addebito-1',
  organization_id: 'org-1',
  lease_id: 'contratto-1',
  charge_type: 'rent',
  status: 'issued',
  effective_status: 'issued',
  period_start: null,
  period_end: null,
  amount: '800.00',
  paid_amount: '0.00',
  outstanding_amount: '800.00',
  currency: 'EUR',
  due_date: '2026-09-15',
  issued_at: '2026-09-01T10:00:00Z',
  created_at: '2026-09-01T10:00:00Z',
  updated_at: '2026-09-01T10:00:00Z',
}

interface Attempt {
  key: string | null
  body: unknown
}

function renderDialog(outcomes: readonly number[]) {
  const attempts: Attempt[] = []
  let sent = 0

  vi.stubGlobal('fetch', async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)
    const headers = new Headers(init?.headers)

    if (url.endsWith('/auth/refresh')) {
      return new Response(JSON.stringify({ code: 'auth.token_invalid', status: 401 }), {
        status: 401,
        headers: { 'content-type': 'application/json' },
      })
    }
    if (url.includes('/payment-declarations')) {
      attempts.push({
        key: headers.get('idempotency-key'),
        body: JSON.parse(String(init?.body ?? '{}')) as unknown,
      })
      const status = outcomes[sent] ?? 201
      sent += 1
      if (status >= 400) {
        return new Response(JSON.stringify({ code: 'internal.error', status }), {
          status,
          headers: { 'content-type': 'application/json' },
        })
      }
      return new Response(JSON.stringify({ id: 'pagamento-1', status: 'pending' }), {
        status,
        headers: { 'content-type': 'application/json' },
      })
    }
    return new Response('{}', { status: 200, headers: { 'content-type': 'application/json' } })
  })

  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })

  render(
    <MemoryRouter>
      <SessionProvider>
        <QueryClientProvider client={queryClient}>
          <DeclarePaymentDialog charge={charge} />
        </QueryClientProvider>
      </SessionProvider>
    </MemoryRouter>,
  )

  return attempts
}

const openDialog = (user: ReturnType<typeof userEvent.setup>) =>
  user.click(screen.getByRole('button', { name: /^ho pagato$/i }))

const submit = (user: ReturnType<typeof userEvent.setup>) =>
  user.click(screen.getAllByRole('button', { name: /^ho pagato$/i }).at(-1) as HTMLElement)

describe('dichiarare di aver pagato', () => {
  it('riusa la stessa chiave a ogni tentativo dello stesso invio', async () => {
    const user = userEvent.setup()
    // Il primo tentativo fallisce con un `500`; il secondo riesce.
    const attempts = renderDialog([500, 201])

    await openDialog(user)
    await screen.findByLabelText(/quanto hai pagato/i)

    await submit(user)
    await waitFor(() => expect(attempts).toHaveLength(1))
    await screen.findByRole('alert')

    // Premere di nuovo è **riprovare**, non pagare una seconda volta: l'importo
    // non è cambiato, e l'intento nemmeno.
    await submit(user)
    await waitFor(() => expect(attempts).toHaveLength(2))

    expect(attempts[0]?.key).toBeTruthy()
    expect(attempts[1]?.key).toBe(attempts[0]?.key)
  })

  it('non manda `payer_user_id`: chi dichiara è sempre chi chiama', async () => {
    const user = userEvent.setup()
    const attempts = renderDialog([201])

    await openDialog(user)
    await screen.findByLabelText(/quanto hai pagato/i)
    await submit(user)

    await waitFor(() => expect(attempts).toHaveLength(1))
    const body = attempts[0]?.body as Record<string, unknown>
    // `payments_insert` esige `payer_user_id = app.current_user_id()` a chi non è
    // staff: mandarlo produrrebbe zero righe inserite, cioè
    // `auth.denied_by_policy` — il codice riservato ai nostri difetti.
    expect(body).not.toHaveProperty('payer_user_id')
    expect(body).toMatchObject({ amount: '800.00', method: 'bank_transfer' })
  })

  it('parte dal residuo e lo manda come stringa, mai come numero', async () => {
    const user = userEvent.setup()
    const attempts = renderDialog([201])

    await openDialog(user)
    const amount = await screen.findByLabelText(/quanto hai pagato/i)
    // Il residuo è già nel campo, dove si legge e si corregge: il caso comune è
    // pagare tutto quello che si deve.
    expect(amount).toHaveValue('800.00')

    await user.clear(amount)
    // La virgola è quello che una tastiera italiana produce: va normalizzata.
    await user.type(amount, '184,20')
    await submit(user)

    await waitFor(() => expect(attempts).toHaveLength(1))
    const body = attempts[0]?.body as { amount: unknown }
    expect(body.amount).toBe('184.20')
    expect(typeof body.amount).toBe('string')
  })

  /**
   * **Il caso che si romperebbe in silenzio**, ed è `R34` letta dal verso giusto.
   *
   * Precompilare la data *e mandarla* sembra la stessa cosa e non lo è:
   * `<input type="date">` produce un giorno senza ora, quindi un pagamento delle 18
   * diventerebbe delle **00:00** — e in un elenco ordinato per data due
   * dichiarazioni dello stesso giorno diventerebbero indistinguibili. Il rimedio è
   * nel difetto: il campo mostra oggi, la richiesta tace.
   *
   * Nessuno se ne accorgerebbe guardando la schermata, perché la schermata è
   * *identica* nei due casi. Solo questo test lo vede.
   */
  it('mostra oggi nel campo e non manda `paid_at` finché non si cambia', async () => {
    const user = userEvent.setup()
    const attempts = renderDialog([201])

    await openDialog(user)
    const date = await screen.findByLabelText(/data del movimento/i)
    // A schermo c'è, così chi dichiara vede quale giorno sta dichiarando.
    expect(date).toHaveValue(todayIso())

    await submit(user)
    await waitFor(() => expect(attempts).toHaveLength(1))
    // Nel corpo no: il server mette l'istante vero, ore e minuti compresi.
    expect(attempts[0]?.body).not.toHaveProperty('paid_at')
  })

  it('manda `paid_at` quando la data viene cambiata davvero', async () => {
    const user = userEvent.setup()
    const attempts = renderDialog([201])

    await openDialog(user)
    const date = await screen.findByLabelText(/data del movimento/i)
    await user.clear(date)
    await user.type(date, '2026-08-31')

    await submit(user)
    await waitFor(() => expect(attempts).toHaveLength(1))
    const body = attempts[0]?.body as { paid_at?: string }
    expect(body.paid_at).toBe(new Date('2026-08-31').toISOString())
  })

  it('non perde ciò che è stato digitato quando la chiamata fallisce', async () => {
    const user = userEvent.setup()
    renderDialog([500])

    await openDialog(user)
    await screen.findByLabelText(/quanto hai pagato/i)
    await user.type(screen.getByLabelText(/causale/i), 'Bonifico del 3')

    await submit(user)
    await screen.findByRole('alert')

    expect(screen.getByLabelText(/causale/i)).toHaveValue('Bonifico del 3')
    expect(screen.getByLabelText(/quanto hai pagato/i)).toHaveValue('800.00')
  })

  it('dice che non si può ritirare **prima** dell’invio', async () => {
    const user = userEvent.setup()
    renderDialog([201])

    await openDialog(user)

    // `payments_update` è staff soltanto: il ritiro non esiste. Va detto dove si
    // decide, non scoperto dopo cercando un bottone che non c'è.
    expect(await screen.findByText(/non la puoi ritirare/i)).toBeInTheDocument()
  })

  it('si invia da sola tastiera', async () => {
    const user = userEvent.setup()
    const attempts = renderDialog([201])

    await openDialog(user)

    // Il primo campo prende il fuoco da sé: chi entra con la tastiera comincia a
    // digitare, non a cercare dove digitare.
    const amount = await screen.findByLabelText(/quanto hai pagato/i)
    await waitFor(() => expect(amount).toHaveFocus())

    // `Invio` in un campo di testo invia il form: è il gesto che chiunque usa, e
    // quello che si perde nel momento in cui qualcuno intercetta `keydown`.
    await user.keyboard('{Enter}')

    await waitFor(() => expect(attempts).toHaveLength(1))
  })
})
