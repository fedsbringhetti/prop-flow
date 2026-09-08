import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router'
import { describe, expect, it, vi } from 'vitest'

import { SessionProvider } from '@/app/session'
import { LoginPage } from './AuthPage'

/**
 * Il percorso da **sola tastiera**, verificato con eventi veri.
 *
 * Il criterio «prodotto reale e non da dimostrazione» rende questa verifica
 * obbligatoria, e non è una che si possa fare guardando: un form si usa a occhio
 * anche quando la tabulazione salta un campo o l'invio non risponde a `Invio`. Qui
 * `user-event` produce eventi di tastiera con `key`, `code` e i comportamenti di
 * default del browser — che è ciò che distingue questa verifica da un click
 * sintetico che «funziona» a prescindere.
 *
 * Copre le tre cose che si rompono per prime:
 *
 * 1. il **giro di tabulazione** arriva a ogni campo e al bottone, in ordine;
 * 2. **`Invio` in un campo di testo invia il form**, che è il gesto che chiunque
 *    usa e che si perde nel momento in cui qualcuno intercetta `keydown`;
 * 3. il form **non perde ciò che è stato digitato** quando la richiesta fallisce.
 *
 * A queste si aggiunge il bottone che **mostra la password**, che è nel giro di
 * tabulazione apposta: un'icona con un `onClick` sarebbe invisibile a chi non usa
 * il mouse, e dentro un `<form>` un `<button>` senza `type` invierebbe il form
 * invece di scoprire il campo.
 */

/**
 * Il campo, e non il bottone che lo scopre.
 *
 * `getByLabelText(/password/i)` da solo ne trova **due**: l'etichetta del campo e
 * l'`aria-label` del bottone, che dice «Mostra la password» perché è ciò che fa.
 * Sono due controlli distinti e devono restare distinguibili, quindi qui si dice
 * quale dei due si vuole invece di rendere il bottone più muto.
 */
function passwordField(): HTMLElement {
  return screen.getByLabelText(/password/i, { selector: 'input' })
}

function renderLogin(fetchImpl: typeof fetch) {
  vi.stubGlobal('fetch', fetchImpl)
  return render(
    <MemoryRouter>
      <SessionProvider>
        <LoginPage />
      </SessionProvider>
    </MemoryRouter>,
  )
}

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

describe('l’accesso da sola tastiera', () => {
  it('tabula fra i campi e invia con Invio, senza toccare il mouse', async () => {
    const calls: string[] = []
    const user = userEvent.setup()

    renderLogin(async (input, init) => {
      const url = String(input)
      calls.push(`${init?.method ?? 'GET'} ${url}`)
      if (url.endsWith('/auth/refresh')) {
        return jsonResponse(401, { code: 'auth.invalid_credentials', status: 401 })
      }
      return jsonResponse(200, {
        access_token: 'token',
        token_type: 'Bearer',
        expires_in: 899,
        organization_id: 'org-1',
        role: 'owner',
        user_id: 'utente-1',
      })
    })

    const email = await screen.findByLabelText(/email/i)
    // Il primo campo prende il fuoco da sé: chi entra con la tastiera comincia a
    // digitare, non a cercare dove digitare.
    await waitFor(() => expect(email).toHaveFocus())

    await user.keyboard('federica.bianchi@example.it')
    await user.tab()
    expect(passwordField()).toHaveFocus()

    await user.keyboard('cavallo batteria graffetta')
    // `Invio` dentro un campo di testo: è l'invio implicito del form, e funziona
    // solo se nessuno ha intercettato il tasto per farne altro.
    await user.keyboard('{Enter}')

    await waitFor(() => {
      expect(calls.some((call) => call.includes('/auth/login'))).toBe(true)
    })
  })

  it('il bottone si attiva con la barra spaziatrice', async () => {
    const calls: string[] = []
    const user = userEvent.setup()

    renderLogin(async (input, init) => {
      const url = String(input)
      calls.push(`${init?.method ?? 'GET'} ${url}`)
      if (url.endsWith('/auth/refresh')) {
        return jsonResponse(401, { code: 'auth.invalid_credentials', status: 401 })
      }
      return jsonResponse(200, {
        access_token: 'token',
        token_type: 'Bearer',
        expires_in: 899,
        user_id: 'utente-1',
      })
    })

    await user.type(await screen.findByLabelText(/email/i), 'federica.bianchi@example.it')
    await user.type(passwordField(), 'cavallo batteria graffetta')
    // Fra il campo e l'invio c'è il bottone che mostra la password, ed è lì
    // apposta: è raggiungibile da tastiera come tutto il resto.
    await user.tab()
    expect(screen.getByRole('button', { name: /mostra la password/i })).toHaveFocus()
    await user.tab()
    expect(screen.getByRole('button', { name: /entra/i })).toHaveFocus()
    await user.keyboard(' ')

    await waitFor(() => {
      expect(calls.some((call) => call.includes('/auth/login'))).toBe(true)
    })
  })

  /**
   * **Il form non si svuota su un errore.**
   *
   * È la differenza fra un'interfaccia che si prova e una che si usa: chi ha
   * sbagliato la password non deve ridigitare anche l'email, e chi ha compilato
   * dodici campi non deve rifarli perché il server ha risposto `422`.
   */
  it('conserva ciò che è stato digitato quando le credenziali sono rifiutate', async () => {
    const user = userEvent.setup()

    renderLogin(async (input) => {
      const url = String(input)
      if (url.endsWith('/auth/refresh')) {
        return jsonResponse(401, { code: 'auth.invalid_credentials', status: 401 })
      }
      return jsonResponse(401, {
        type: 'unauthorized',
        title: 'Authentication required',
        status: 401,
        code: 'auth.invalid_credentials',
      })
    })

    await user.type(await screen.findByLabelText(/email/i), 'federica.bianchi@example.it')
    await user.type(passwordField(), 'password sbagliata')
    await user.click(screen.getByRole('button', { name: /entra/i }))

    // L'errore è annunciato — `role="alert"` — e non solo disegnato.
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent(/email o password non corrette/i)
    expect(screen.getByLabelText(/email/i)).toHaveValue('federica.bianchi@example.it')
    expect(passwordField()).toHaveValue('password sbagliata')
  })

  /**
   * Ogni campo ha un'etichetta **legata al controllo**.
   *
   * Senza, un lettore di schermo annuncia «campo di testo» e nient'altro, e
   * l'unico modo di accorgersene è provarlo — cioè mai. Il test lo chiede per
   * nome, quindi fallisce nel momento in cui il legame si rompe.
   */
  it('ogni campo è raggiungibile per etichetta', async () => {
    renderLogin(async () => jsonResponse(401, { code: 'auth.invalid_credentials', status: 401 }))

    expect(await screen.findByLabelText(/email/i)).toBeInTheDocument()
    expect(passwordField()).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /entra/i })).toBeInTheDocument()
  })

  /**
   * **Mostrare la password non invia il form.**
   *
   * Dentro un `<form>` il default di `<button>` è `submit`, ed è il difetto più
   * comune e più invisibile dei form HTML: si scopre premendo, quando la pagina è
   * già partita. Qui si verifica che il carattere si veda, che l'etichetta dica in
   * che stato si è — l'icona da sola è ambigua — e che **nessuna richiesta parta**.
   */
  it('mostra e nasconde la password senza inviare il form', async () => {
    const calls: string[] = []
    const user = userEvent.setup()

    renderLogin(async (input, init) => {
      const url = String(input)
      calls.push(`${init?.method ?? 'GET'} ${url}`)
      return jsonResponse(401, { code: 'auth.invalid_credentials', status: 401 })
    })

    await user.type(await screen.findByLabelText(/email/i), 'federica.bianchi@example.it')
    await user.type(passwordField(), 'cavallo batteria graffetta')
    expect(passwordField()).toHaveAttribute('type', 'password')

    const toggle = screen.getByRole('button', { name: /mostra la password/i })
    expect(toggle).toHaveAttribute('aria-pressed', 'false')
    await user.click(toggle)

    expect(passwordField()).toHaveAttribute('type', 'text')
    // Ciò che era stato digitato resta, e il campo non si è svuotato scoprendolo.
    expect(passwordField()).toHaveValue('cavallo batteria graffetta')

    const pressed = screen.getByRole('button', { name: /nascondi la password/i })
    expect(pressed).toHaveAttribute('aria-pressed', 'true')
    await user.click(pressed)
    expect(passwordField()).toHaveAttribute('type', 'password')

    expect(calls.some((call) => call.includes('/auth/login'))).toBe(false)
  })
})
