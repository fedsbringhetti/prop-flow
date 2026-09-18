import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router'
import { describe, expect, it, vi } from 'vitest'

import { SessionProvider } from '@/app/session'
import { todayIso } from '@/lib/format'
import type { Charge } from '@/api/types'
import { RecordPaymentDialog } from './RecordPaymentDialog'

/**
 * La registrazione di un incasso, dai due lati che si rompono in silenzio.
 *
 * 1. **la chiave di idempotenza è la stessa a ogni tentativo.** Non è la suite: è
 *    il doppio invio, fatto davvero, con l'header guardato prima e dopo. Se
 *    cambiasse, il secondo tentativo scriverebbe un **secondo pagamento** e il
 *    saldo scenderebbe due volte, senza alcun errore da nessuna parte;
 * 2. **il form non perde ciò che è stato digitato** quando la chiamata fallisce.
 *    È la sola cosa che distingue un errore recuperabile da un lavoro da rifare;
 * 3. **si arriva in fondo da sola tastiera**, e l'attribuzione è una scelta che
 *    non si può saltare.
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

const lease = {
  id: 'contratto-1',
  parties: [
    {
      id: 'parte-1',
      lease_id: 'contratto-1',
      user_id: 'utente-inquilino',
      role: 'tenant',
      full_name: 'Inquilino Pagante',
      share_percent: '100',
      is_primary: true,
      joined_at: null,
      left_at: null,
    },
  ],
}

interface Attempt {
  key: string | null
  body: unknown
}

/**
 * Monta la modale con un `fetch` finto che registra i tentativi.
 *
 * `outcomes` è la sequenza di esiti: il primo tentativo prende il primo, e così
 * via. È ciò che permette di riprodurre «fallisce, poi riesce» — cioè esattamente
 * lo scenario in cui una chiave rigenerata farebbe danno.
 */
function renderDialog(outcomes: readonly number[]) {
  const attempts: Attempt[] = []
  let recorded = 0

  vi.stubGlobal('fetch', async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)
    const headers = new Headers(init?.headers)

    if (url.endsWith('/auth/refresh')) {
      return new Response(JSON.stringify({ code: 'auth.token_invalid', status: 401 }), {
        status: 401,
        headers: { 'content-type': 'application/json' },
      })
    }
    if (url.includes('/leases/')) {
      return new Response(JSON.stringify(lease), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      })
    }
    if (url.includes('/payments')) {
      attempts.push({
        key: headers.get('idempotency-key'),
        body: JSON.parse(String(init?.body ?? '{}')) as unknown,
      })
      const status = outcomes[recorded] ?? 201
      recorded += 1
      if (status >= 400) {
        return new Response(JSON.stringify({ code: 'internal.error', status }), {
          status,
          headers: { 'content-type': 'application/json' },
        })
      }
      return new Response(JSON.stringify({ id: 'pagamento-1', status: 'succeeded' }), {
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
          <RecordPaymentDialog charge={charge} />
        </QueryClientProvider>
      </SessionProvider>
    </MemoryRouter>,
  )

  return attempts
}

/** Compila i due campi obbligatori: l'importo e l'attribuzione. */
async function fillIn(user: ReturnType<typeof userEvent.setup>, amount: string) {
  await user.type(await screen.findByLabelText(/importo incassato/i), amount)
  await waitFor(() =>
    expect(screen.getByRole('option', { name: 'Inquilino Pagante' })).toBeInTheDocument(),
  )
  await user.selectOptions(screen.getByLabelText(/a chi attribuirlo/i), 'utente-inquilino')
}

describe('registrare un incasso', () => {
  it('riusa la stessa chiave a ogni tentativo dello stesso invio', async () => {
    const user = userEvent.setup()
    // Il primo tentativo fallisce con un `500`; il secondo riesce.
    const attempts = renderDialog([500, 201])

    await user.click(screen.getByRole('button', { name: /registra un incasso/i }))
    await fillIn(user, '300.00')

    await user.click(screen.getByRole('button', { name: /^registra un incasso$/i }))
    await waitFor(() => expect(attempts).toHaveLength(1))
    await screen.findByRole('alert')

    // Il secondo invio è **lo stesso intento**: l'importo non è cambiato e chi
    // preme di nuovo sta riprovando, non pagando una seconda volta.
    await user.click(screen.getByRole('button', { name: /^registra un incasso$/i }))
    await waitFor(() => expect(attempts).toHaveLength(2))

    expect(attempts[0]?.key).toBeTruthy()
    expect(attempts[1]?.key).toBe(attempts[0]?.key)
  })

  it('non perde ciò che è stato digitato quando la chiamata fallisce', async () => {
    const user = userEvent.setup()
    renderDialog([500])

    await user.click(screen.getByRole('button', { name: /registra un incasso/i }))
    await fillIn(user, '300.00')
    await user.type(screen.getByLabelText(/causale/i), 'Bonifico del 3')

    await user.click(screen.getByRole('button', { name: /^registra un incasso$/i }))
    await screen.findByRole('alert')

    // La modale è ancora aperta e i campi sono ancora pieni: un errore
    // recuperabile non deve costare di riscrivere tutto.
    expect(screen.getByLabelText(/importo incassato/i)).toHaveValue('300.00')
    expect(screen.getByLabelText(/causale/i)).toHaveValue('Bonifico del 3')
  })

  it('non invia finché l’attribuzione non è stata scelta', async () => {
    const user = userEvent.setup()
    const attempts = renderDialog([201])

    await user.click(screen.getByRole('button', { name: /registra un incasso/i }))
    await user.type(await screen.findByLabelText(/importo incassato/i), '300.00')

    // Nessuna scelta su «a chi attribuirlo»: il campo è obbligatorio proprio
    // perché ometterlo rende l'incasso invisibile a chi ha pagato, e sarebbe il
    // comportamento di chi semplicemente se ne dimentica.
    await user.click(screen.getByRole('button', { name: /^registra un incasso$/i }))

    expect(attempts).toHaveLength(0)
    expect(await screen.findByRole('alert')).toHaveTextContent(/obbligatorio/i)
  })

  it('si compila e si invia da sola tastiera', async () => {
    const user = userEvent.setup()
    const attempts = renderDialog([201])

    await user.click(screen.getByRole('button', { name: /registra un incasso/i }))

    // Il primo campo prende il fuoco da sé: chi entra con la tastiera comincia a
    // digitare, non a cercare dove digitare.
    const amount = await screen.findByLabelText(/importo incassato/i)
    await waitFor(() => expect(amount).toHaveFocus())
    await user.keyboard('300.00')

    await waitFor(() =>
      expect(screen.getByRole('option', { name: 'Inquilino Pagante' })).toBeInTheDocument(),
    )
    await user.selectOptions(screen.getByLabelText(/a chi attribuirlo/i), 'utente-inquilino')

    // `Invio` in un campo di testo invia il form: è il gesto che chiunque usa, e
    // quello che si perde nel momento in cui qualcuno intercetta `keydown`.
    amount.focus()
    await user.keyboard('{Enter}')

    await waitFor(() => expect(attempts).toHaveLength(1))
    expect(attempts[0]?.body).toMatchObject({
      amount: '300.00',
      method: 'bank_transfer',
      payer_user_id: 'utente-inquilino',
    })
  })

  /**
   * **La data si mostra e non si manda**, finché resta quella di oggi.
   *
   * Gemello del test sulla dichiarazione, e qui pesa di più: un bonifico si vede
   * sull'estratto conto giorni dopo, quindi cambiarla è il gesto normale — ma
   * *mandarla* senza che nessuno l'abbia toccata trasformerebbe un incasso
   * registrato alle 18 in uno delle 00:00.
   */
  it('mostra oggi nel campo e non manda `paid_at` finché non si cambia', async () => {
    const user = userEvent.setup()
    const attempts = renderDialog([201])

    await user.click(screen.getByRole('button', { name: /registra un incasso/i }))
    const date = await screen.findByLabelText(/data del movimento/i)
    expect(date).toHaveValue(todayIso())

    await fillIn(user, '300.00')
    await user.click(screen.getByRole('button', { name: /^registra un incasso$/i }))

    await waitFor(() => expect(attempts).toHaveLength(1))
    expect(attempts[0]?.body).not.toHaveProperty('paid_at')
  })

  it('manda l’importo come stringa, mai come numero', async () => {
    const user = userEvent.setup()
    const attempts = renderDialog([201])

    await user.click(screen.getByRole('button', { name: /registra un incasso/i }))
    // La virgola è quello che una tastiera italiana produce: va accettata e
    // normalizzata, non rifiutata.
    await fillIn(user, '184,20')
    await user.click(screen.getByRole('button', { name: /^registra un incasso$/i }))

    await waitFor(() => expect(attempts).toHaveLength(1))
    const sent = attempts[0]?.body as { amount: unknown }
    expect(sent.amount).toBe('184.20')
    expect(typeof sent.amount).toBe('string')
  })
})
