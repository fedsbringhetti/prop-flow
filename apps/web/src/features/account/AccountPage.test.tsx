import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { SessionProvider } from '@/app/session'
import { AccountPage } from './AccountPage'

/**
 * Il proprio account, dai due punti che il piano segna come trappole.
 *
 * 1. **L'email si mostra e non si modifica**, con la riga che dice perché: un campo
 *    disabilitato senza spiegazione si legge come un difetto.
 * 2. **L'uscita chiede conferma dicendo che cosa resta**, e la propria
 *    organizzazione — di cui si è l'unico proprietario — non ha nemmeno il bottone,
 *    perché porterebbe a un `membership.last_owner`.
 *
 * Non si prova qui la revoca del cambio password: disconnette, e un test che la
 * seguisse fino in fondo verificherebbe il logout, non l'account. Si prova che la
 * conseguenza sia **scritta prima del bottone**, che è la cosa che si toglie
 * «migliorando».
 */

const ME = '00000000-0000-4000-8000-00000000000a'
const OWN_ORG = '00000000-0000-4000-8000-0000000000f1'
const RENTED_ORG = '00000000-0000-4000-8000-0000000000f2'

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

interface Call {
  method: string
  url: string
  /** L'organizzazione **dichiarata** dalla richiesta, se ce n'è una. */
  org: string | undefined
}

/**
 * @param activeOrg quale organizzazione il token dichiara attiva.
 */
function renderPage(activeOrg = OWN_ORG): Call[] {
  const calls: Call[] = []
  // Vero appena l'uscita è avvenuta: da lì in poi il finto server si comporta
  // come `resolve_role`, che rifiuta un `X-Org-Id` su un'appartenenza non attiva.
  let left = false

  vi.stubGlobal('fetch', async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)
    const method = init?.method ?? 'GET'
    const org = (init?.headers as Record<string, string> | undefined)?.['x-org-id']
    calls.push({ method, url, org })

    // **Il rifiuto vero, riprodotto.** `Authenticated` risolve il ruolo per
    // qualunque richiesta che porti `X-Org-Id` e risponde `403` prima
    // dell'handler: senza questo ramo il test passerebbe anche con il difetto.
    if (left && org === RENTED_ORG) {
      return json({ code: 'org.not_a_member', title: 'Forbidden' }, 403)
    }

    if (url.endsWith('/auth/refresh')) {
      return json({
        access_token: 'token',
        token_type: 'Bearer',
        expires_in: 900,
        user_id: ME,
        organization_id: activeOrg,
        role: activeOrg === OWN_ORG ? 'owner' : 'tenant',
      })
    }
    if (url.endsWith('/me')) {
      return json({
        id: ME,
        email: 'mario.rossi@example.test',
        full_name: 'Mario Rossi',
        locale: 'it-IT',
      })
    }
    if (url.endsWith('/organizations')) {
      // Dopo l'uscita l'elenco vero non la contiene più: è ciò che il
      // riallineamento del guscio aspetta per correggere l'organizzazione attiva.
      const own = { id: OWN_ORG, name: 'Immobili di Mario', slug: 'immobili-di-mario', role: 'owner' }
      const rented = { id: RENTED_ORG, name: 'Casa di Anna', slug: 'casa-di-anna', role: 'tenant' }
      return json({ items: left ? [own] : [own, rented] })
    }
    if (url.includes('/leave')) {
      left = true
      return json({ organization_id: RENTED_ORG, role: 'tenant' })
    }
    return json({}, 200)
  })

  render(
    <MemoryRouter>
      <SessionProvider>
        <QueryClientProvider
          client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
        >
          <AccountPage />
        </QueryClientProvider>
      </SessionProvider>
    </MemoryRouter>,
  )

  return calls
}

afterEach(() => {
  vi.unstubAllGlobals()
  // L'organizzazione ricordata è per browser e sopravvive a un test: senza
  // ripulirla, il secondo eredita la scelta del primo.
  localStorage.clear()
})

describe('il proprio account', () => {
  it('mostra l’email disabilitata, con la riga che dice perché', async () => {
    renderPage()

    const email = (await screen.findByLabelText(/^email/i)) as HTMLInputElement
    expect(email.value).toBe('mario.rossi@example.test')
    expect(email).toBeDisabled()
    // La spiegazione, senza cui un campo spento sembra un guasto.
    expect(screen.getByText(/credenziale d’accesso/i)).toBeInTheDocument()
  })

  it('avverte della revoca prima del bottone del cambio password', async () => {
    renderPage()
    await screen.findByLabelText(/^email/i)

    expect(screen.getByText(/chiudi le sessioni aperte/i)).toBeInTheDocument()
  })

  it('esce da un’organizzazione altrui, e non offre l’uscita dalla propria', async () => {
    const user = userEvent.setup()
    const calls = renderPage()
    await screen.findByLabelText(/^email/i)

    // La propria — dove si è owner — compare senza il bottone «esci».
    expect(await screen.findByText('La tua organizzazione')).toBeInTheDocument()

    // Quella in cui si è inquilini ha il bottone: si preme, si conferma.
    await user.click(screen.getByRole('button', { name: /^esci$/i }))
    expect(await screen.findByText(/non vedrai più i contratti/i)).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: /sì, esci/i }))

    await waitFor(() =>
      expect(calls.some((c) => c.method === 'POST' && c.url.includes(`/organizations/${RENTED_ORG}/leave`))).toBe(
        true,
      ),
    )
  })

  /**
   * **Uscire dall'organizzazione attiva non deve incastrare l'applicazione.**
   *
   * È il difetto che la sezione 6 ha reso raggiungibile, e la catena vale la pena
   * scriverla perché ogni anello sembrava innocuo da solo:
   *
   * 1. si esce, e il server dice sì;
   * 2. il client tiene l'organizzazione lasciata come attiva — sta in
   *    `localStorage`, e nessuno gliel'ha ancora tolta;
   * 3. `GET /organizations`, che è **la richiesta che ripara quel ricordo**,
   *    mandava `X-Org-Id`, quindi tornava `403 org.not_a_member`;
   * 4. React Query tiene i dati dell'ultimo successo quando un refetch fallisce,
   *    quindi l'elenco conteneva ancora l'organizzazione lasciata e il
   *    riallineamento non partiva mai. Nemmeno ricaricando, perché il ricordo
   *    viene riselezionato **prima** che l'elenco sia chiesto.
   *
   * Il finto server riproduce il rifiuto vero: senza quel ramo il test passerebbe
   * anche con il difetto, ed è la parte che si dimentica scrivendolo dopo.
   */
  it('esce dall’organizzazione attiva senza restare incastrata su di essa', async () => {
    const user = userEvent.setup()
    const calls = renderPage(RENTED_ORG)
    await screen.findByLabelText(/^email/i)
    expect(await screen.findByText('Casa di Anna')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: /^esci$/i }))
    await user.click(screen.getByRole('button', { name: /sì, esci/i }))

    // L'elenco si riallinea: la riga sparisce invece di restare con un errore
    // accanto, che è ciò che succedeva quando la rilettura veniva rifiutata.
    await waitFor(() => expect(screen.queryByText('Casa di Anna')).not.toBeInTheDocument())
    expect(screen.getByText('Immobili di Mario')).toBeInTheDocument()

    // La radice: l'elenco delle organizzazioni **non dichiara** un'organizzazione.
    // È l'unica richiesta che deve poter rispondere quando il ricordo è sbagliato.
    const listings = calls.filter((c) => c.url.endsWith('/organizations'))
    expect(listings.length).toBeGreaterThan(0)
    expect(listings.every((c) => c.org === undefined)).toBe(true)

    // E nessuna richiesta, dopo l'uscita, porta più l'organizzazione lasciata.
    const afterLeave = calls.slice(calls.findIndex((c) => c.url.includes('/leave')) + 1)
    expect(afterLeave.every((c) => c.org !== RENTED_ORG)).toBe(true)
  })
})
