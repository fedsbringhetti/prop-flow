import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router'
import { describe, expect, it, vi } from 'vitest'

import { SessionProvider } from '@/app/session'
import { PasswordResetPage } from './AuthPage'

/**
 * Il recupero della password, dai due lati che si rompono in silenzio.
 *
 * 1. **la schermata dice la verità, non una promessa.** L'API risponde `202` anche
 *    a un indirizzo che non ha un account, quindi «ti abbiamo mandato un'email»
 *    sarebbe falso proprio nel caso in cui non è partito niente — e la sua assenza
 *    diventerebbe un modo per sapere chi è registrato. Il test guarda il testo,
 *    perché è precisamente la riga che qualcuno «migliora»;
 * 2. **il codice si incolla e viaggia nel corpo**, mai nell'URL. Un token in un
 *    percorso finisce nella cronologia, nel `Referer` e nei log di ogni proxy
 *    davanti al client (ADR-045, ADR-052): il test verifica l'URL chiamato, non
 *    l'intenzione di chi l'ha scritto.
 */

interface Call {
  url: string
  body: unknown
}

function renderPage(): Call[] {
  const calls: Call[] = []

  vi.stubGlobal('fetch', async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)
    if (url.endsWith('/auth/refresh')) {
      return new Response(JSON.stringify({ code: 'auth.token_invalid', status: 401 }), {
        status: 401,
        headers: { 'content-type': 'application/json' },
      })
    }
    calls.push({ url, body: JSON.parse(String(init?.body ?? '{}')) as unknown })
    return new Response(null, { status: url.includes('/confirm') ? 204 : 202 })
  })

  render(
    <MemoryRouter>
      <SessionProvider>
        <QueryClientProvider
          client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
        >
          <PasswordResetPage />
        </QueryClientProvider>
      </SessionProvider>
    </MemoryRouter>,
  )

  return calls
}

describe('recuperare la password', () => {
  it('non promette un’email: dice «se quell’indirizzo ha un account»', async () => {
    const user = userEvent.setup()
    const calls = renderPage()

    await user.type(await screen.findByLabelText(/^email/i), 'chiunque@example.test')
    await user.click(screen.getByRole('button', { name: /mandami il codice/i }))

    await waitFor(() => expect(calls).toHaveLength(1))
    expect(calls[0]?.url).toContain('/auth/password-reset')
    expect(calls[0]?.body).toEqual({ email: 'chiunque@example.test' })

    // La frase esatta è il punto: una che prometta l'invio sarebbe falsa per un
    // indirizzo senza account, e la sua assenza direbbe chi è registrato.
    expect(await screen.findByText(/se quell’indirizzo ha un account/i)).toBeInTheDocument()
  })

  it('manda il codice nel corpo e non nell’URL, e avverte della revoca', async () => {
    const user = userEvent.setup()
    const calls = renderPage()

    await user.type(await screen.findByLabelText(/^email/i), 'chi.entra@example.test')
    await user.click(screen.getByRole('button', { name: /mandami il codice/i }))

    const code = await screen.findByLabelText(/codice ricevuto/i)
    await user.type(code, 'codice-incollato')
    await user.type(screen.getByLabelText(/password nuova/i), 'cavallo batteria graffetta')

    // La conseguenza si legge **prima** di premere: chi cambia la password si
    // ritrova disconnesso anche sul telefono, e scoprirlo dopo sembra un guasto.
    expect(screen.getByText(/chiude le sessioni aperte/i)).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /cambia la password/i }))

    await waitFor(() => expect(calls).toHaveLength(2))
    const confirm = calls[1]
    expect(confirm?.url).toContain('/auth/password-reset/confirm')
    // **Nell'URL non c'è**: né come segmento, né come parametro.
    expect(confirm?.url).not.toContain('codice-incollato')
    expect(confirm?.body).toEqual({
      code: 'codice-incollato',
      new_password: 'cavallo batteria graffetta',
    })
  })
})
