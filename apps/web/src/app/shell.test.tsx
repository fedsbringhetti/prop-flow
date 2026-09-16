import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useEffect } from 'react'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Organization } from '@/api/types'
import { SessionProvider, useApi } from './session'
import { AppShell } from './shell'

/**
 * Il guscio, dai due lati che questa fetta ha cambiato.
 *
 * 1. **il selettore mostra i nomi**, non gli UUID. Prima l'elenco era ciò che
 *    questo browser aveva conosciuto, e per un inquilino sarebbero stati due
 *    identificatori fra cui indovinare;
 * 2. **la navigazione è una funzione del ruolo**. Non è un confine di sicurezza —
 *    a negare sono l'API e la policy — ma è la differenza fra un prodotto e sei
 *    schermate che sembrano funzionare finché non si preme qualcosa: un inquilino
 *    non incontra una pagina mancante, incontra un `403`;
 * 3. **l'elenco vero corregge il ricordo**. Se l'organizzazione ricordata non è
 *    più fra le proprie, ogni richiesta successiva porterebbe un `X-Org-Id` che
 *    l'API rifiuta — cioè un'applicazione intera che risponde `403`.
 */

const OWNER_ORG = '00000000-0000-4000-8000-000000000001'
const TENANT_ORG = '00000000-0000-4000-8000-000000000002'

const organizations: Organization[] = [
  { id: TENANT_ORG, name: 'Casa di Giulia', slug: 'casa-di-giulia-11aa22', role: 'owner' },
  { id: OWNER_ORG, name: 'Immobili di Mario Rossi', slug: 'immobili-4f21ab', role: 'tenant' },
]

interface Options {
  /** L'organizzazione e il ruolo che il **token** porta. */
  role: 'owner' | 'tenant'
  organizationId: string
  /** L'elenco che risponde `GET /organizations`. `null` lo lascia in volo. */
  items?: Organization[] | null
}

function renderShell({ role, organizationId, items = organizations }: Options) {
  const orgHeaders: (string | null)[] = []
  let pending: (() => void) | null = null

  vi.stubGlobal('fetch', async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)
    if (url.endsWith('/auth/refresh')) {
      return new Response(
        JSON.stringify({
          access_token: 'token',
          token_type: 'Bearer',
          expires_in: 900,
          user_id: 'utente-1',
          organization_id: organizationId,
          role,
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      )
    }
    if (url.endsWith('/organizations')) {
      orgHeaders.push(new Headers(init?.headers).get('x-org-id'))
      if (items === null) {
        // Mai risolta: è il caricamento, che non deve somigliare a un elenco vuoto.
        await new Promise<void>((resolve) => {
          pending = resolve
        })
      }
      return new Response(JSON.stringify({ items: items ?? [] }), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      })
    }
    return new Response('{}', { status: 200, headers: { 'content-type': 'application/json' } })
  })

  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })

  render(
    <MemoryRouter initialEntries={['/addebiti']}>
      <SessionProvider>
        <QueryClientProvider client={queryClient}>
          <Routes>
            <Route element={<AppShell />}>
              <Route path="/addebiti" element={<p>contenuto</p>} />
            </Route>
          </Routes>
        </QueryClientProvider>
      </SessionProvider>
    </MemoryRouter>,
  )

  return { orgHeaders, release: () => pending?.() }
}

/**
 * La navigazione, letta **dalla regione con il suo nome accessibile**.
 *
 * `getAllByRole('link')` sull'intera pagina prenderebbe anche il salto al
 * contenuto, che è il primo elemento del DOM apposta. Cercare dentro il `<nav>`
 * per nome è anche il modo in cui ci si muove con un lettore di schermo, ed è la
 * ragione per cui quel nome esiste.
 *
 * Il marcatore «(pagina corrente)» è testo per soli lettori di schermo: lo stato
 * attivo non è **solo** un colore. Qui si toglie perché il test guarda le voci,
 * non quale sia selezionata.
 */
function navigationLinks(): string[] {
  const nav = screen.getByRole('navigation', { name: 'Navigazione principale' })
  return within(nav)
    .getAllByRole('link')
    .map((link) => (link.textContent ?? '').replace('(pagina corrente)', '').trim())
    .filter((label) => label !== '')
}

beforeEach(() => {
  localStorage.clear()
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('il guscio si adatta al ruolo', () => {
  it('mostra all’inquilino cinque voci, e non il riepilogo', async () => {
    renderShell({ role: 'tenant', organizationId: OWNER_ORG })

    await screen.findByText('contenuto')
    const labels = navigationLinks()

    expect(labels).toEqual(
      expect.arrayContaining(['Addebiti', 'Spese', 'Segnalazioni', 'Contratto', 'Bollette']),
    )
    // Il riepilogo gli risponderebbe `403 org.staff_role_required`: `ensure_staff`
    // *stringe* la policy invece di ripeterla, e una voce che porta a un errore è
    // peggio di una voce che manca.
    expect(labels).not.toContain('Riepilogo')
    // Gli immobili escono per un'altra ragione, che non è la sicurezza: gliene
    // mostrerebbe uno, e una voce che porta a un elenco di un elemento è rumore.
    expect(labels).not.toContain('Immobili')
    expect(labels).not.toContain('Pagamenti')
  })

  it('mostra al proprietario tutte le voci, riepilogo compreso', async () => {
    renderShell({ role: 'owner', organizationId: TENANT_ORG })

    await screen.findByText('contenuto')
    const labels = navigationLinks()

    expect(labels).toEqual(
      expect.arrayContaining([
        'Riepilogo',
        'Immobili',
        'Contratti',
        'Segnalazioni',
        'Bollette',
        'Addebiti',
        'Pagamenti',
      ]),
    )
    // Le spese co-living sono il denaro che i coinquilini si devono **fra loro**,
    // e il proprietario ne è estraneo per costruzione. L'API gliele concederebbe —
    // quasi tutto quel modulo è `ensure_member` — ma una voce che per lui è sempre
    // vuota è rumore. `/spese` resta raggiungibile per URL: un proprietario che
    // abiti in una delle proprie case è in un gruppo come chiunque altro.
    expect(labels).not.toContain('Spese')
  })
})

describe('il selettore delle organizzazioni', () => {
  it('mostra i nomi e non gli identificatori', async () => {
    const user = userEvent.setup()
    renderShell({ role: 'tenant', organizationId: OWNER_ORG })

    await screen.findByText('contenuto')
    // Il nome compare già nel bottone, prima ancora di aprire il menù: è lì che
    // serve, perché è la scritta che dice in quale organizzazione si sta.
    await screen.findAllByText('Immobili di Mario Rossi')

    await user.click(screen.getAllByRole('button', { name: /organizzazione attiva/i })[0]!)
    await screen.findByText('Casa di Giulia')

    // L'UUID non compare da nessuna parte: è precisamente ciò da cui questo passo
    // libera.
    expect(screen.queryByText(OWNER_ORG)).not.toBeInTheDocument()
    expect(screen.queryByText(TENANT_ORG)).not.toBeInTheDocument()
  })

  it('non mostra un elenco vuoto mentre sta caricando', async () => {
    const user = userEvent.setup()
    const { release } = renderShell({ role: 'tenant', organizationId: OWNER_ORG, items: null })

    await screen.findByText('contenuto')
    await user.click(screen.getAllByRole('button', { name: /organizzazione attiva/i })[0]!)

    // «Non ce ne sono» e «non le ho ancora lette» sono due cose diverse, e
    // mostrarle uguali fa cercare un problema che non esiste.
    expect(await screen.findByText(/cerco le tue organizzazioni/i)).toBeInTheDocument()
    expect(screen.queryByText(/non appartieni a nessuna organizzazione/i)).not.toBeInTheDocument()

    release()
  })

  it('corregge l’organizzazione ricordata quando non è più fra le proprie', async () => {
    const orphan = '00000000-0000-4000-8000-0000000000ff'
    localStorage.setItem(
      'propflow.active-org.v1:utente-1',
      JSON.stringify({ active: orphan, role: 'tenant' }),
    )

    const { orgHeaders } = renderShell({ role: 'tenant', organizationId: OWNER_ORG })

    await screen.findByText('contenuto')

    // Senza la correzione l'applicazione resterebbe a mandare l'`X-Org-Id`
    // orfano, e **ogni** richiesta risponderebbe `403 org.not_a_member`.
    await waitFor(() => {
      expect(screen.getAllByText('Casa di Giulia').length).toBeGreaterThan(0)
    })

    // **L'identificatore orfano non esce mai**, e questa riga asseriva il
    // contrario: pretendeva che la *prima* richiesta lo portasse, cioè descriveva
    // il difetto come se fosse il comportamento. L'elenco delle organizzazioni è
    // la richiesta che ripara il ricordo, e dichiarare un'organizzazione è
    // precisamente ciò che la faceva rifiutare — con l'elenco irraggiungibile, la
    // correzione non partiva mai. Vedi il poscritto dell'esito della sezione 6.
    expect(orgHeaders.every((header) => header !== orphan)).toBe(true)
  })
})

/**
 * Il terzo stato del guscio (`F7`, ADR-053): nessuna organizzazione, accanto a
 * «fuori sessione» e «dentro».
 *
 * Il punto del criterio di fine che questa suite prova: la schermata dedicata
 * compare, `<Outlet/>` non rende — cioè `/addebiti` non compare mai — e
 * **nessuna richiesta partita dopo l'elenco** porta `X-Org-Id`: è l'elenco a
 * dire che l'organizzazione del token non esiste più, e solo da lì in poi il
 * guscio lo sa. È la stessa lezione del poscritto della sezione 6, qui per
 * «zero organizzazioni» invece che «organizzazione sbagliata».
 */
describe('il guscio senza nessuna organizzazione', () => {
  /**
   * Il caso vero, non quello che si dimostra da sé. `/auth/refresh` porta
   * un'organizzazione — è il token, come per chi non è mai uscito — e solo
   * `/organizations` dice che non esiste più. Un refresh **senza**
   * `organization_id` renderebbe l'assert sugli header vero a prescindere dal
   * guscio: `session.organizationId` sarebbe già nullo dal primo istante, e
   * `ApiClient.send` non manderebbe mai `X-Org-Id` per costruzione — il test
   * proverebbe se stesso, non il guscio.
   */
  const DEAD_ORG = '00000000-0000-4000-8000-0000000000dd'

  /**
   * La rotta figlia vera, non un `<p>` inerte. Deve chiamare l'API **con**
   * organizzazione — `useApi()` è lo stesso client che userebbe una schermata
   * di dominio — perché l'unico modo di dimostrare che `<Outlet/>` non rende
   * è avere qualcosa che, se rendesse, si vedrebbe: un testo comparso e una
   * richiesta partita. Un elemento che non chiama nulla renderebbe l'assert
   * sugli header vero anche se il gate del guscio sparisse.
   */
  function RouteProbe() {
    const client = useApi()
    useEffect(() => {
      void client.get('/properties')
    }, [client])
    return <p>contenuto</p>
  }

  function renderEmptyShell() {
    /**
     * Ogni richiesta partita dopo il login, con la propria destinazione e
     * l'header `X-Org-Id` che portava — per distinguere «nessuna richiesta ha
     * portato un'organizzazione» da «`/properties`, quella della rotta
     * figlia, non è mai partita».
     */
    const requests: { url: string; orgHeader: string | null }[] = []

    vi.stubGlobal('fetch', async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input)
      if (url.endsWith('/auth/refresh')) {
        // Un'organizzazione che il token porta e che non esiste più: è lo
        // stato di chi è uscito dall'unica che aveva, scoperto solo quando
        // l'elenco risponde — non prima, dal token stesso.
        return new Response(
          JSON.stringify({
            access_token: 'token',
            token_type: 'Bearer',
            expires_in: 900,
            user_id: 'utente-1',
            organization_id: DEAD_ORG,
            role: 'tenant',
          }),
          { status: 200, headers: { 'content-type': 'application/json' } },
        )
      }
      if (url.endsWith('/organizations')) {
        requests.push({ url, orgHeader: new Headers(init?.headers).get('x-org-id') })
        return new Response(JSON.stringify({ items: [] }), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        })
      }
      if (url.endsWith('/me')) {
        requests.push({ url, orgHeader: new Headers(init?.headers).get('x-org-id') })
        return new Response(
          JSON.stringify({
            id: 'utente-1',
            email: 'sara.neri@example.it',
            full_name: 'Sara Neri',
            locale: 'it-IT',
            created_at: '2026-01-01T00:00:00Z',
          }),
          { status: 200, headers: { 'content-type': 'application/json' } },
        )
      }
      // Qualunque altra richiesta — `/properties` della rotta figlia, se
      // `<Outlet/>` l'avesse resa per sbaglio — si registra come le altre: se
      // ne comparisse una, l'ultima riga del test la vedrebbe.
      requests.push({ url, orgHeader: new Headers(init?.headers).get('x-org-id') })
      return new Response(JSON.stringify({ items: [] }), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      })
    })

    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    })

    render(
      <MemoryRouter initialEntries={['/addebiti']}>
        <SessionProvider>
          <QueryClientProvider client={queryClient}>
            <Routes>
              <Route element={<AppShell />}>
                <Route path="/addebiti" element={<RouteProbe />} />
              </Route>
            </Routes>
          </QueryClientProvider>
        </SessionProvider>
      </MemoryRouter>,
    )

    return { requests }
  }

  it('mostra la schermata dedicata e non manda mai X-Org-Id dopo l’elenco', async () => {
    const { requests } = renderEmptyShell()

    // **La riga per cui questo test esiste.** Il token porta `DEAD_ORG`, non
    // nulla: se il guscio non riallineasse la sessione quando l'elenco
    // risponde vuoto, questa attesa scadrebbe — `session.organizationId`
    // resterebbe `DEAD_ORG` per sempre, il terzo stato non scatterebbe mai, e
    // «Sara Neri» non comparirebbe.
    expect(await screen.findByText(/Sara Neri/)).toBeInTheDocument()

    // Nessuna navigazione, e la rotta figlia non è mai montata: se
    // `<Outlet/>` l'avesse resa, il suo `useEffect` avrebbe chiamato
    // `/properties` con `useApi()` e il suo testo sarebbe comparso — nessuno
    // dei due accade.
    expect(screen.queryByRole('navigation')).not.toBeInTheDocument()
    expect(screen.queryByText('contenuto')).not.toBeInTheDocument()
    expect(requests.some((request) => request.url.endsWith('/properties'))).toBe(false)

    // Ogni richiesta che questo stato ha fatto partire dopo che l'elenco ha
    // risposto — `/me`, e l'elenco stesso — non porta `DEAD_ORG` né alcun
    // altro identificatore: la sessione non ne ha più uno da dare.
    await waitFor(() => expect(requests.length).toBeGreaterThan(0))
    expect(requests.every((request) => request.orgHeader === null)).toBe(true)
    expect(requests.map((request) => request.orgHeader)).not.toContain(DEAD_ORG)
  })

  /**
   * Il terzo caso di `session.organizationId === null` (B2 della revisione):
   * non solo «sta caricando» e «è vuoto per davvero», ma anche «non è arrivato
   * per niente». Senza un ramo suo, `items` resta `undefined` per sempre e il
   * guscio è un caricamento senza via d'uscita — niente riprova, niente
   * logout — che è precisamente la regola 23 violata.
   */
  it('mostra la riprova e l’uscita quando l’elenco fallisce, non un caricamento senza fine', async () => {
    let attempts = 0

    vi.stubGlobal('fetch', async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url.endsWith('/auth/refresh')) {
        return new Response(
          JSON.stringify({
            access_token: 'token',
            token_type: 'Bearer',
            expires_in: 900,
            user_id: 'utente-1',
          }),
          { status: 200, headers: { 'content-type': 'application/json' } },
        )
      }
      if (url.endsWith('/organizations')) {
        attempts += 1
        return new Response(
          JSON.stringify({
            type: 'about:blank',
            title: 'Internal error',
            status: 500,
            code: 'internal.error',
          }),
          { status: 500, headers: { 'content-type': 'application/problem+json' } },
        )
      }
      return new Response('{}', { status: 200, headers: { 'content-type': 'application/json' } })
    })

    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    })

    render(
      <MemoryRouter initialEntries={['/addebiti']}>
        <SessionProvider>
          <QueryClientProvider client={queryClient}>
            <Routes>
              <Route element={<AppShell />}>
                <Route path="/addebiti" element={<p>contenuto</p>} />
              </Route>
            </Routes>
          </QueryClientProvider>
        </SessionProvider>
      </MemoryRouter>,
    )

    // **La riga per cui questo test esiste.** Non un caricamento senza fine:
    // lo stato d'errore, annunciato, con la riprova e l'uscita.
    expect(await screen.findByRole('alert')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /riprova/i })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /esci/i })).toBeInTheDocument()
    expect(screen.queryByText('contenuto')).not.toBeInTheDocument()
    expect(attempts).toBeGreaterThan(0)
  })

  /**
   * M1 della revisione: `NoOrganizationScreen` scriveva `selectOrganization`
   * mentre la cache dell'elenco era ancora `[]` — quella letta da questa
   * stessa schermata poco prima — e l'effetto di riallineamento vedeva
   * l'elenco vuoto, chiamava `clearOrganization()` (che cancella il ricordo
   * di questo browser) e il ramo `fallback` lo riscriveva da capo non appena
   * la rilettura arrivava: un rimbalzo cancella-e-riscrive per
   * un'accettazione sola.
   */
  it('accettando un codice, scrive un solo ricordo e non lo cancella mai', async () => {
    const user = userEvent.setup()
    const ACCEPTED_ORG = '00000000-0000-4000-8000-0000000000ee'
    const ORG_PREFERENCE_KEY = 'propflow.active-org.v1'
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem')
    const removeItemSpy = vi.spyOn(Storage.prototype, 'removeItem')
    // Prima dell'accettazione l'elenco è vuoto per davvero; dopo, la
    // rilettura che `invalidateQueries` scatena vede l'organizzazione vera —
    // com'è nella realtà, e non un elenco che resta vuoto per comodità del
    // controllo. È la rilettura vera a dover confermare il segnaposto, non a
    // contraddirlo.
    let accepted = false

    vi.stubGlobal('fetch', async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url.endsWith('/auth/refresh')) {
        return new Response(
          JSON.stringify({
            access_token: 'token',
            token_type: 'Bearer',
            expires_in: 900,
            user_id: 'utente-1',
          }),
          { status: 200, headers: { 'content-type': 'application/json' } },
        )
      }
      if (url.endsWith('/auth/accept-invitation')) {
        accepted = true
        return new Response(
          JSON.stringify({ organization_id: ACCEPTED_ORG, role: 'tenant' }),
          { status: 200, headers: { 'content-type': 'application/json' } },
        )
      }
      if (url.endsWith('/organizations')) {
        const items = accepted
          ? [{ id: ACCEPTED_ORG, name: 'Organizzazione Vera', slug: 'organizzazione-vera', role: 'tenant' }]
          : []
        return new Response(JSON.stringify({ items }), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        })
      }
      if (url.endsWith('/me')) {
        return new Response(
          JSON.stringify({
            id: 'utente-1',
            email: 'utente@example.it',
            full_name: 'Utente Prova',
            locale: 'it-IT',
            created_at: '2026-01-01T00:00:00Z',
          }),
          { status: 200, headers: { 'content-type': 'application/json' } },
        )
      }
      return new Response('{}', { status: 200, headers: { 'content-type': 'application/json' } })
    })

    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    })

    render(
      <MemoryRouter initialEntries={['/addebiti']}>
        <SessionProvider>
          <QueryClientProvider client={queryClient}>
            <Routes>
              <Route element={<AppShell />}>
                <Route path="/addebiti" element={<p>scadenzario</p>} />
              </Route>
            </Routes>
          </QueryClientProvider>
        </SessionProvider>
      </MemoryRouter>,
    )

    const field = await screen.findByLabelText(/codice d’invito/i)
    await user.type(field, 'codice-buono')
    await user.click(screen.getByRole('button', { name: /^entra$/i }))

    await screen.findByText('scadenzario')

    const preferenceWrites = setItemSpy.mock.calls.filter(([key]) =>
      String(key).startsWith(ORG_PREFERENCE_KEY),
    )
    const preferenceClears = removeItemSpy.mock.calls.filter(([key]) =>
      String(key).startsWith(ORG_PREFERENCE_KEY),
    )

    // **La riga per cui questo test esiste.** Un solo ricordo scritto, quello
    // buono, e nessuna cancellazione in mezzo.
    expect(preferenceWrites.length).toBe(1)
    expect(preferenceClears.length).toBe(0)

    setItemSpy.mockRestore()
    removeItemSpy.mockRestore()
  })
})
