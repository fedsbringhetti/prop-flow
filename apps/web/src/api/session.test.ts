import { describe, expect, it } from 'vitest'

import { ApiClient } from './client'
import { ApiError } from './problem'
import { SessionStore, type FetchLike } from './session'

/**
 * Un'API finta che si comporta come quella vera **sulla cosa che conta**: ruota il
 * refresh token a ogni uso e tratta il riuso come prova di una copia rubata,
 * revocando l'intera catena.
 *
 * Non è un doppio di comodo: è precisamente `RefreshSession::ensure_usable` del
 * dominio, e senza questa severità il test del singolo volo passerebbe anche con
 * il difetto dentro.
 */
class RotatingApi {
  /** Ogni chiamata a `/auth/refresh` arrivata, in ordine. */
  readonly refreshCalls: string[] = []
  /** Le richieste di business arrivate, come `METODO percorso`. */
  readonly businessCalls: string[] = []

  private currentRefreshToken = 'refresh-1'
  private issued = 1
  private chainRevoked = false
  private accessToken = 'access-scaduto'
  /** Quanto dura una risposta di refresh: serve a farne partire tre insieme. */
  private readonly latencyMs: number

  constructor(options: { latencyMs?: number } = {}) {
    this.latencyMs = options.latencyMs ?? 5
  }

  /** Come se l'access token in circolazione fosse scaduto. */
  expireAccessToken(): void {
    this.accessToken = 'access-scaduto'
  }

  /**
   * Azzera i contatori dopo la preparazione.
   *
   * Ogni scenario comincia con un rinnovo che serve solo ad avere una sessione:
   * contarlo insieme a quelli dello scenario renderebbe le asserzioni un
   * «tre più uno» che nessuno rilegge senza sbagliare.
   */
  resetCounters(): void {
    this.refreshCalls.length = 0
    this.businessCalls.length = 0
  }

  readonly fetch: FetchLike = async (input, init) => {
    const url = new URL(input, 'http://propflow.test')
    const method = init?.method ?? 'GET'

    if (url.pathname === '/api/v1/auth/refresh') {
      return this.refresh(init)
    }

    this.businessCalls.push(`${method} ${url.pathname}`)
    const authorization = headerOf(init, 'authorization')
    if (authorization !== `Bearer ${this.accessToken}`) {
      return problem(401, 'auth.token_expired')
    }
    return json(200, { ok: true, org: headerOf(init, 'x-org-id') ?? null })
  }

  private async refresh(init: RequestInit | undefined): Promise<Response> {
    // Il cookie lo manderebbe il browser; qui lo si legge dall'header, che è
    // l'unica cosa che un `fetch` finto può osservare.
    const presented = headerOf(init, 'cookie') ?? this.currentRefreshToken
    this.refreshCalls.push(presented)

    await delay(this.latencyMs)

    if (this.chainRevoked) return problem(401, 'auth.invalid_credentials')

    if (presented !== this.currentRefreshToken) {
      // Riuso: qualcuno ha una copia, e non si sa quale parte sia legittima.
      this.chainRevoked = true
      return problem(401, 'auth.invalid_credentials')
    }

    this.issued += 1
    this.currentRefreshToken = `refresh-${this.issued}`
    this.accessToken = `access-${this.issued}`
    return json(200, {
      access_token: this.accessToken,
      token_type: 'Bearer',
      expires_in: 899,
      organization_id: 'org-1',
      role: 'owner',
      user_id: 'utente-1',
    })
  }
}

function headerOf(init: RequestInit | undefined, name: string): string | null {
  const headers = init?.headers
  if (!headers) return null
  const entries = headers as Record<string, string>
  return entries[name] ?? null
}

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

function problem(status: number, code: string): Response {
  return new Response(JSON.stringify({ type: code, title: code, status, code }), {
    status,
    headers: { 'content-type': 'application/problem+json' },
  })
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function scenario(options: { singleFlight: boolean }): {
  api: RotatingApi
  client: ApiClient
  session: SessionStore
} {
  const api = new RotatingApi()
  const session = new SessionStore({ fetch: api.fetch, singleFlight: options.singleFlight })
  const client = new ApiClient({ fetch: api.fetch, session })
  // La sessione parte con un access token già scaduto: è lo stato in cui una
  // pagina si trova quando la si riapre dopo un quarto d'ora.
  session.selectOrganization('org-1', 'owner')
  return { api, client, session }
}

/** Dà alla sessione un token scaduto, come dopo un login di quindici minuti fa. */
function withStaleToken(session: SessionStore): void {
  session.selectOrganization('org-1', 'owner')
}

describe('il refresh a singolo volo', () => {
  /**
   * **Il difetto, riprodotto al contrario.**
   *
   * Senza la promessa condivisa tre richieste parallele che scadono insieme fanno
   * tre chiamate a `/auth/refresh`: la prima ruota, le altre due presentano un
   * token già ruotato, e l'API revoca la catena. Il test lo mostra accadere.
   *
   * Serve a due cose, e nessuna è la copertura: dimostra che la severità dell'API
   * finta è quella vera — altrimenti il test qui sotto passerebbe anche col
   * difetto dentro — e lascia scritto **che cosa** il singolo volo evita, invece
   * di lasciarlo a un commento che nessuno rilegge.
   */
  it('senza la promessa condivisa, tre richieste parallele buttano fuori l’utente', async () => {
    const api = new RotatingApi()
    const session = new SessionStore({ fetch: api.fetch, singleFlight: false })
    const client = new ApiClient({ fetch: api.fetch, session })
    await session.refresh()
    api.resetCounters()
    api.expireAccessToken()

    const outcomes = await Promise.allSettled([
      client.get('/properties'),
      client.get('/leases'),
      client.get('/charges'),
    ])

    expect(api.refreshCalls.length).toBe(3)
    const rejected = outcomes.filter((outcome) => outcome.status === 'rejected')
    expect(rejected.length).toBeGreaterThan(0)
    const failure = rejected[0]
    expect(failure).toBeDefined()
    if (failure && failure.status === 'rejected') {
      expect(failure.reason).toBeInstanceOf(ApiError)
      expect((failure.reason as ApiError).status).toBe(401)
    }
  })

  /** Lo stesso scenario, con il singolo volo: **una** chiamata, nessuna revoca. */
  it('con la promessa condivisa, tre richieste parallele fanno un solo refresh', async () => {
    const api = new RotatingApi()
    const session = new SessionStore({ fetch: api.fetch })
    const client = new ApiClient({ fetch: api.fetch, session })
    await session.refresh()
    api.resetCounters()
    api.expireAccessToken()

    const results = await Promise.all([
      client.get<{ ok: boolean }>('/properties'),
      client.get<{ ok: boolean }>('/leases'),
      client.get<{ ok: boolean }>('/charges'),
    ])

    expect(api.refreshCalls.length).toBe(1)
    expect(results.every((result) => result.ok)).toBe(true)
  })

  it('rilascia la promessa a rinnovo concluso: un secondo scadere ne fa un altro', async () => {
    const api = new RotatingApi()
    const session = new SessionStore({ fetch: api.fetch })
    const client = new ApiClient({ fetch: api.fetch, session })

    await session.refresh()
    api.resetCounters()

    api.expireAccessToken()
    await client.get('/properties')
    expect(api.refreshCalls.length).toBe(1)

    api.expireAccessToken()
    await client.get('/properties')
    expect(api.refreshCalls.length).toBe(2)
  })

  it('un refresh rifiutato chiude la sessione e lo dice al guscio', async () => {
    let ended = 0
    let refuse = false
    const api = new RotatingApi()
    const session = new SessionStore({
      fetch: async (input, init) =>
        refuse ? problem(401, 'auth.invalid_credentials') : api.fetch(input, init),
      onSessionEnded: () => {
        ended += 1
      },
    })

    // Prima una sessione vera: è lo stato da cui un rifiuto è un evento.
    await session.refresh()
    expect(session.current).not.toBeNull()

    refuse = true
    await expect(session.refresh()).rejects.toBeInstanceOf(ApiError)
    expect(session.current).toBeNull()
    expect(ended).toBe(1)
  })

  /**
   * La rete assente **non** è un logout.
   *
   * Cancellare la sessione qui manderebbe al login chi è entrato in galleria, e
   * al ritorno del segnale dovrebbe ridigitare la password per niente.
   */
  it('un errore di rete non cancella la sessione', async () => {
    const api = new RotatingApi()
    let failNext = false
    const session = new SessionStore({
      fetch: async (input, init) => {
        if (failNext) throw new TypeError('Failed to fetch')
        return api.fetch(input, init)
      },
    })
    await session.refresh()
    expect(session.current).not.toBeNull()

    failNext = true
    await expect(session.refresh()).rejects.toMatchObject({ status: 0 })
    expect(session.current).not.toBeNull()
  })
})

describe('il client dell’API', () => {
  it('riprova una volta sola dopo il rinnovo, e non entra in ciclo', async () => {
    let refreshes = 0
    const session = new SessionStore({
      fetch: async () => {
        refreshes += 1
        return json(200, {
          access_token: 'sempre-rifiutato',
          token_type: 'Bearer',
          expires_in: 899,
          user_id: 'utente-1',
        })
      },
    })
    await session.refresh()
    refreshes = 0

    let attempts = 0
    const client = new ApiClient({
      session,
      fetch: async () => {
        attempts += 1
        return problem(401, 'auth.token_expired')
      },
    })

    await expect(client.get('/properties')).rejects.toBeInstanceOf(ApiError)
    expect(attempts).toBe(2)
    expect(refreshes).toBe(1)
  })

  it('non tenta il rinnovo su un login sbagliato', async () => {
    let refreshes = 0
    const session = new SessionStore({
      fetch: async () => {
        refreshes += 1
        return problem(401, 'auth.invalid_credentials')
      },
    })
    const client = new ApiClient({
      session,
      fetch: async () => problem(401, 'auth.invalid_credentials'),
    })

    await expect(client.post('/auth/login', {})).rejects.toMatchObject({
      code: 'auth.invalid_credentials',
    })
    expect(refreshes).toBe(0)
  })

  it('manda sempre l’organizzazione attiva, anche quando è quella del token', async () => {
    const api = new RotatingApi()
    const session = new SessionStore({ fetch: api.fetch })
    const client = new ApiClient({ fetch: api.fetch, session })
    await session.refresh()
    api.resetCounters()
    withStaleToken(session)
    api.expireAccessToken()

    const result = await client.get<{ org: string | null }>('/properties')
    expect(result.org).toBe('org-1')
  })

  it('un 204 non prova a leggere un corpo che non c’è', async () => {
    const session = new SessionStore({ fetch: async () => problem(401, 'auth.token_invalid') })
    const client = new ApiClient({
      session,
      fetch: async () => new Response(null, { status: 204 }),
    })
    await expect(client.delete('/properties/qualcosa')).resolves.toBeUndefined()
  })
})

describe('il guscio della sessione', () => {
  it('l’organizzazione attiva è stato esplicito e sopravvive al rinnovo scelto', async () => {
    const api = new RotatingApi()
    const session = new SessionStore({ fetch: api.fetch })
    await session.refresh()
    expect(session.current?.organizationId).toBe('org-1')

    session.selectOrganization('org-2', 'tenant')
    expect(session.current?.organizationId).toBe('org-2')
    expect(session.current?.role).toBe('tenant')
  })

  it('notifica chi si è iscritto quando la sessione cambia', async () => {
    const api = new RotatingApi()
    const session = new SessionStore({ fetch: api.fetch })
    let changes = 0
    const unsubscribe = session.subscribe(() => {
      changes += 1
    })

    await session.refresh()
    expect(changes).toBe(1)

    unsubscribe()
    session.clear()
    expect(changes).toBe(1)
  })
})

describe('lo scenario che il singolo volo protegge, visto da vicino', () => {
  it('il difetto e il rimedio partono dallo stesso stato', async () => {
    const broken = scenario({ singleFlight: false })
    await broken.session.refresh()
    broken.api.resetCounters()
    broken.api.expireAccessToken()
    await Promise.allSettled([
      broken.client.get('/properties'),
      broken.client.get('/leases'),
      broken.client.get('/charges'),
    ])

    const fixed = scenario({ singleFlight: true })
    await fixed.session.refresh()
    fixed.api.resetCounters()
    fixed.api.expireAccessToken()
    await Promise.all([
      fixed.client.get('/properties'),
      fixed.client.get('/leases'),
      fixed.client.get('/charges'),
    ])

    // Le stesse tre richieste, lo stesso stato di partenza, tre refresh contro
    // uno. È l'intera differenza, e vale la sessione dell'utente.
    expect(broken.api.refreshCalls.length).toBe(3)
    expect(fixed.api.refreshCalls.length).toBe(1)
    expect(fixed.session.current).not.toBeNull()
  })
})
