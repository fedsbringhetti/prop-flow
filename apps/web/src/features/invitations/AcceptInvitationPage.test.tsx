import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { SessionProvider } from '@/app/session'
import { AcceptInvitationPage, AcceptInvitationSignedOutPage } from './AcceptInvitationPage'

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

/**
 * `/invito`, fuori sessione (`F7`, ADR-053): il codice **prima**, la
 * registrazione **dopo**, con il codice già dentro e senza un campo email.
 *
 * I due punti del criterio di fine che questa suite prova:
 *
 * 1. **si manda il token, non un'email** — la rotta nuova non ha un campo
 *    `email` e l'unico modo di romperla sarebbe rimandarne una nel corpo;
 * 2. **il campo dell'organizzazione compare solo dopo il `409
 *    invitation.owner_token`**, mai prima — mostrarlo da subito
 *    chiederebbe un nome anche a chi sta accettando un invito da inquilino.
 */
function renderEntry(handler: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>) {
  vi.stubGlobal('fetch', handler)

  // **Senza `QueryClientProvider`, di proposito.** È così che `App` monta le
  // rotte da fuori sessione: il provider avvolge solo il ramo autenticato. La
  // prima stesura di questo harness ne metteva uno, e la pagina — che chiamava
  // `useQueryClient()` — era verde qui e bianca nel browser, con un errore non
  // catturato. Il test deve rendere la pagina come la rende il prodotto.
  render(
    <SessionProvider>
      <MemoryRouter initialEntries={['/invito']}>
        <Routes>
          <Route path="/invito" element={<AcceptInvitationSignedOutPage />} />
          <Route path="/addebiti" element={<p>scadenzario</p>} />
          <Route path="/" element={<p>riepilogo</p>} />
        </Routes>
      </MemoryRouter>
    </SessionProvider>,
  )
}

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

/** Nessuna sessione da ripristinare: è la condizione reale di chi arriva senza account. */
function refuseRefresh(url: string): Response | null {
  if (!url.endsWith('/auth/refresh')) return null
  return jsonResponse(401, { code: 'auth.token_missing', status: 401 })
}

/**
 * Il campo, e non il bottone che lo scopre: `getByLabelText(/password/i)` da
 * solo ne trova due, l'etichetta del campo e l'`aria-label` del bottone che
 * mostra la password.
 */
function passwordField(): HTMLElement {
  return screen.getByLabelText(/password/i, { selector: 'input' })
}

async function enterCode(code: string) {
  const user = userEvent.setup()
  const field = await screen.findByLabelText(/codice d’invito/i)
  await user.type(field, code)
  await user.click(screen.getByRole('button', { name: /continua/i }))
  return user
}

describe('la registrazione dall’invito, fuori sessione', () => {
  it('manda il token dell’invito, non un’email', async () => {
    const sentBodies: Record<string, unknown>[] = []

    renderEntry(async (input, init) => {
      const url = String(input)
      const refused = refuseRefresh(url)
      if (refused) return refused
      if (url.endsWith('/auth/register-from-invitation')) {
        sentBodies.push(JSON.parse(String(init?.body)) as Record<string, unknown>)
        return jsonResponse(201, {
          access_token: 'token',
          token_type: 'Bearer',
          expires_in: 900,
          user_id: 'utente-1',
          organization_id: OWN_ORG,
          role: 'tenant',
        })
      }
      return jsonResponse(200, { items: [] })
    })

    const user = await enterCode('codice-inquilino')
    await user.type(screen.getByLabelText(/nome e cognome/i), 'Nuovo Inquilino')
    await user.type(passwordField(), 'cavallo batteria graffetta')
    await user.click(screen.getByRole('button', { name: /crea l’account ed entra/i }))

    await waitFor(() => expect(sentBodies.length).toBe(1))
    const sent = sentBodies[0]
    expect(sent).toBeDefined()
    // La riga per cui il test esiste: il token c'è, l'email no. `email` non è
    // nemmeno un campo del form — se qualcuno lo aggiungesse per «uniformare»
    // alla vecchia registrazione, questa riga cade.
    expect(sent?.['token']).toBe('codice-inquilino')
    expect(sent).not.toHaveProperty('email')
    expect(sent?.['full_name']).toBe('Nuovo Inquilino')

    // La sessione è installata come il login: si atterra sulla pagina del
    // ruolo senza un secondo giro.
    expect(await screen.findByText('scadenzario')).toBeInTheDocument()
  })

  it('mostra il campo dell’organizzazione solo dopo il 409 invitation.owner_token', async () => {
    let registerAttempts = 0
    const ownerBodies: Record<string, unknown>[] = []

    renderEntry(async (input, init) => {
      const url = String(input)
      const refused = refuseRefresh(url)
      if (refused) return refused
      if (url.endsWith('/auth/register-from-invitation')) {
        registerAttempts += 1
        return jsonResponse(409, {
          type: 'conflict',
          title: 'owner token',
          status: 409,
          code: 'invitation.owner_token',
        })
      }
      if (url.endsWith('/auth/accept-owner-invitation')) {
        ownerBodies.push(JSON.parse(String(init?.body)) as Record<string, unknown>)
        return jsonResponse(201, {
          access_token: 'token',
          token_type: 'Bearer',
          expires_in: 900,
          user_id: 'utente-2',
          organization_id: OWN_ORG,
          role: 'owner',
        })
      }
      return jsonResponse(200, { items: [] })
    })

    const user = await enterCode('codice-proprietario')
    await user.type(screen.getByLabelText(/nome e cognome/i), 'Nuovo Proprietario')
    await user.type(passwordField(), 'cavallo batteria graffetta')

    // **La riga per cui questo test esiste, prima metà.** Non si sa ancora se
    // il codice è da proprietario: il campo non deve comparire per indovinare.
    expect(screen.queryByLabelText(/nome dell’organizzazione/i)).not.toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /crea l’account ed entra/i }))

    // Il `409` non è un errore mostrato: è il segnale che manca un campo, e
    // nome e password restano compilati — non si ridigita nulla.
    const orgField = await screen.findByLabelText(/nome dell’organizzazione/i)
    expect(screen.getByLabelText(/nome e cognome/i)).toHaveValue('Nuovo Proprietario')
    expect(passwordField()).toHaveValue('cavallo batteria graffetta')
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()

    await user.type(orgField, 'La mia agenzia')
    await user.click(screen.getByRole('button', { name: /crea l’account ed entra/i }))

    await waitFor(() => expect(ownerBodies.length).toBe(1))
    expect(registerAttempts).toBe(1)
    const sent = ownerBodies[0]
    expect(sent?.['token']).toBe('codice-proprietario')
    expect(sent?.['organization_name']).toBe('La mia agenzia')

    expect(await screen.findByText('riepilogo')).toBeInTheDocument()
  })
})
