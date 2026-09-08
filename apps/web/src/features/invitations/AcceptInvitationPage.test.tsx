import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { SessionProvider } from '@/app/session'
import { AcceptInvitationPage } from './AcceptInvitationPage'

/**
 * L'accettazione di un invito, dai due lati che questa fetta esiste per chiudere.
 *
 * 1. **senza questa schermata il giro del prodotto non si chiude**: il
 *    proprietario genera il codice, lo consegna, e chi lo riceve non ha un posto
 *    in cui usarlo. Era l'unica operazione dell'API raggiungibile soltanto con
 *    `curl`;
 * 2. **accettare sceglie l'organizzazione appena accettata**, che è l'unico
 *    momento in cui il client sa quale si vuole. Senza, chi entra atterra nella
 *    propria organizzazione personale — `pick_default_membership` preferisce il
 *    ruolo più alto e la registrazione ne crea sempre una di cui si è owner — e la
 *    prima schermata di un inquilino appena registrato è «comincia registrando il
 *    tuo primo immobile».
 *
 * La chiamata va **senza `X-Org-Id`**, ed è la trappola più cara di questo passo:
 * è l'operazione che *crea* l'appartenenza, e dichiarare un'organizzazione di cui
 * non si è ancora membri fa rispondere `403` a una richiesta legittima. Il primo
 * test lo verifica sull'header vero, non sul codice che lo scrive.
 */

const INVITED_ORG = '00000000-0000-4000-8000-0000000000aa'
const OWN_ORG = '00000000-0000-4000-8000-0000000000bb'

interface Reply {
  status: number
  body: unknown
}

function renderPage(reply: Reply) {
  /** Gli header di `POST /auth/accept-invitation`, per verificarli dopo. */
  const acceptHeaders: Headers[] = []

  vi.stubGlobal('fetch', async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)
    if (url.endsWith('/auth/refresh')) {
      // La sessione esiste già, ed è quella **personale**: è la condizione reale
      // di chi si è appena registrato per accettare un invito.
      return new Response(
        JSON.stringify({
          access_token: 'token',
          token_type: 'Bearer',
          expires_in: 900,
          user_id: 'utente-1',
          organization_id: OWN_ORG,
          role: 'owner',
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      )
    }
    if (url.endsWith('/auth/accept-invitation')) {
      acceptHeaders.push(new Headers(init?.headers))
      return new Response(JSON.stringify(reply.body), {
        status: reply.status,
        headers: { 'content-type': 'application/problem+json' },
      })
    }
    return new Response('{}', { status: 200, headers: { 'content-type': 'application/json' } })
  })

  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })

  render(
    <SessionProvider>
      <QueryClientProvider client={queryClient}>
        <MemoryRouter initialEntries={['/invito']}>
          <Routes>
            <Route path="/invito" element={<AcceptInvitationPage />} />
            {/* Le destinazioni possibili, riconoscibili una dall'altra: è così
                che si verifica che il ruolo decida dove si atterra. */}
            <Route path="/addebiti" element={<p>scadenzario</p>} />
            <Route path="/" element={<p>riepilogo</p>} />
          </Routes>
        </MemoryRouter>
      </QueryClientProvider>
    </SessionProvider>,
  )

  return { acceptHeaders }
}

async function paste(code: string) {
  const user = userEvent.setup()
  const field = await screen.findByLabelText(/codice d’invito/i)
  await user.type(field, code)
  await user.click(screen.getByRole('button', { name: /^entra$/i }))
}

afterEach(() => {
  vi.unstubAllGlobals()
  localStorage.clear()
})

describe('accettare un invito', () => {
  it('non dichiara un’organizzazione di cui non si è ancora membri', async () => {
    const { acceptHeaders } = renderPage({
      status: 200,
      body: { organization_id: INVITED_ORG, role: 'tenant' },
    })

    await paste('codice-buono')

    await waitFor(() => expect(acceptHeaders.length).toBe(1))
    const sent = acceptHeaders[0]
    expect(sent).toBeDefined()
    // **La riga per cui questo test esiste.** Con `X-Org-Id` addosso la richiesta
    // riceverebbe `403` pur essendo legittima, e chi la debugga cercherebbe il
    // problema nel token invece che in un header di troppo.
    expect(sent?.get('x-org-id')).toBeNull()
    // L'autenticazione invece serve: la rotta è autenticata.
    expect(sent?.get('authorization')).toBe('Bearer token')
  })

  it('sceglie l’organizzazione appena accettata e porta alla pagina del ruolo', async () => {
    renderPage({ status: 200, body: { organization_id: INVITED_ORG, role: 'tenant' } })

    await paste('codice-buono')

    // Per chi abita la pagina iniziale è lo scadenzario, non la radice: la
    // radice ce lo rimanderebbe comunque, e passarci in mezzo farebbe
    // lampeggiare una schermata che non è la sua.
    expect(await screen.findByText('scadenzario')).toBeInTheDocument()

    // L'organizzazione ricordata per questo browser è **quella dell'invito** e
    // non quella personale con cui la sessione era partita. È la metà del debito
    // che questa fetta chiude senza migration.
    await waitFor(() => {
      const stored = localStorage.getItem('propflow.active-org.v1:utente-1')
      expect(stored).not.toBeNull()
      expect(JSON.parse(stored ?? '{}')).toMatchObject({ active: INVITED_ORG, role: 'tenant' })
    })
  })

  it('un codice sbagliato non parla di password', async () => {
    renderPage({
      status: 401,
      body: {
        type: 'auth.invalid_credentials',
        title: 'invalid credentials',
        status: 401,
        code: 'auth.invalid_credentials',
      },
    })

    await paste('codice-inventato')

    // Nel catalogo `auth.invalid_credentials` significa «email o password non
    // corrette», che qui sarebbe una frase su una password che nessuno ha
    // digitato: la schermata riscrive il messaggio, e questo test è ciò che
    // impedisce di «semplificare» tornando a quello globale.
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent(/codice non è valido/i)
    expect(alert).not.toHaveTextContent(/password/i)

    // E si resta qui, con il campo ancora compilato: un codice rifiutato si
    // ridigita solo se chi lo ha mandato ne manda un altro.
    expect(screen.getByLabelText(/codice d’invito/i)).toHaveValue('codice-inventato')
  })

  it('un invito già accettato non è un errore di chi legge', async () => {
    renderPage({
      status: 409,
      body: {
        type: 'invitation.already_accepted',
        title: 'already accepted',
        status: 409,
        code: 'invitation.already_accepted',
      },
    })

    await paste('codice-gia-usato')

    expect(await screen.findByRole('alert')).toHaveTextContent(/già stato accettato/i)
  })
})
