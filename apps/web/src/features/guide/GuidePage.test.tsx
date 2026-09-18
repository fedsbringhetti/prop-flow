import { render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { SessionProvider } from '@/app/session'
import { t } from '@/i18n'
import { GuidePage } from './GuidePage'

/**
 * Una pagina, due versioni: il ruolo sceglie. La prova è che una sezione che
 * esiste solo per chi amministra — gli immobili, che chi abita non registra — non
 * compaia a un inquilino, e che una che esiste solo per chi abita — le spese fra
 * coinquilini, che chi amministra non vede — non compaia allo staff.
 *
 * Resa **senza** `QueryClientProvider`, com'è nel prodotto per ciò che le serve:
 * la pagina non legge niente dall'API, e un harness più generoso del prodotto è la
 * forma del difetto che il giro a mano della 8 ha trovato.
 */
function renderGuide(role: 'owner' | 'tenant' | null) {
  let refreshes = 0
  vi.stubGlobal('fetch', async (input: RequestInfo | URL) => {
    const url = String(input)
    if (url.endsWith('/auth/refresh')) {
      refreshes += 1
      return new Response(
        JSON.stringify({
          access_token: 'token',
          token_type: 'Bearer',
          expires_in: 900,
          user_id: 'utente-1',
          organization_id: role ? 'org-1' : null,
          role,
        }),
        { status: 200, headers: { 'content-type': 'application/json' } },
      )
    }
    return new Response('{}', { status: 200, headers: { 'content-type': 'application/json' } })
  })

  render(
    <MemoryRouter initialEntries={['/guida']}>
      <SessionProvider>
        <GuidePage />
      </SessionProvider>
    </MemoryRouter>,
  )

  // Il ruolo arriva con il rinnovo, e la pagina rende prima: si aspetta che sia
  // arrivato, altrimenti un'asserzione negativa passerebbe sulla versione
  // provvisoria.
  return { settled: () => waitFor(() => expect(refreshes).toBe(1)) }
}

beforeEach(() => {
  localStorage.clear()
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('la guida sceglie la versione dal ruolo', () => {
  it('a chi amministra racconta il giro dagli immobili, e non le spese fra coinquilini', async () => {
    renderGuide('owner')

    expect(
      await screen.findByRole('heading', { level: 2, name: t.nav.properties }),
    ).toBeInTheDocument()
    expect(screen.getByRole('heading', { level: 2, name: t.nav.dashboard })).toBeInTheDocument()
    expect(screen.getByRole('heading', { level: 2, name: t.guide.entry })).toBeInTheDocument()
    expect(screen.queryByRole('heading', { level: 2, name: t.nav.expenses })).not.toBeInTheDocument()
    expect(screen.queryByRole('heading', { level: 2, name: t.nav.lease })).not.toBeInTheDocument()
  })

  it('a chi abita racconta l’altro lato, senza gli immobili né il riepilogo', async () => {
    const page = renderGuide('tenant')
    await page.settled()

    expect(
      await screen.findByRole('heading', { level: 2, name: t.nav.expenses }),
    ).toBeInTheDocument()
    expect(screen.getByRole('heading', { level: 2, name: t.nav.lease })).toBeInTheDocument()
    expect(screen.getByRole('heading', { level: 2, name: t.guide.entry })).toBeInTheDocument()
    expect(screen.queryByRole('heading', { level: 2, name: t.nav.properties })).not.toBeInTheDocument()
    expect(screen.queryByRole('heading', { level: 2, name: t.nav.dashboard })).not.toBeInTheDocument()
  })

  it('senza un ruolo — chi aspetta un invito — legge la versione di chi abita', async () => {
    const page = renderGuide(null)
    await page.settled()

    expect(
      await screen.findByRole('heading', { level: 2, name: t.nav.expenses }),
    ).toBeInTheDocument()
    expect(screen.queryByRole('heading', { level: 2, name: t.nav.properties })).not.toBeInTheDocument()
  })

  it('ogni sezione si chiama come una voce della navigazione, o come l’ingresso', () => {
    const names = new Set<string>([...Object.values(t.nav), t.guide.entry])
    for (const version of [t.guide.staff, t.guide.tenant]) {
      for (const section of version.sections) expect(names).toContain(section.title)
    }
  })
})
