import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Organization } from '@/api/types'
import { SessionProvider } from './session'
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
