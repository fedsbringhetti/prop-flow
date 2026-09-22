import { describe, expect, it, vi } from 'vitest'

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
  // Il lock fra schede segue il singolo volo: questo scenario mette a confronto
  // il difetto e il rimedio, e con il lock acceso il ramo «difetto» sarebbe
  // protetto da qualcos'altro. Vedi il primo test di questo file.
  const session = new SessionStore({
    fetch: api.fetch,
    singleFlight: options.singleFlight,
    crossTabLock: options.singleFlight,
  })
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
    // **Anche il lock fra schede va spento**, e la ragione è la stessa per cui
    // questo test esiste: il lock serializza pure dentro una scheda, quindi
    // lasciandolo acceso le tre chiamate passerebbero una alla volta e
    // riuscirebbero tutte. Il difetto resterebbe invisibile, e questo test —
    // che serve a dimostrare che il singolo volo protegge da qualcosa —
    // passerebbe senza provare niente.
    const session = new SessionStore({
      fetch: api.fetch,
      singleFlight: false,
      crossTabLock: false,
    })
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

  /**
   * **Un `5xx` al rinnovo non è un rifiuto** (`R52`). Un `401` che arriva a una
   * richiesta mentre l'API si riavvia, seguito dal rinnovo che riceve un `502`
   * dal proxy, buttava via la sessione in memoria con il cookie di refresh
   * intatto. Il `401` è l'unico messaggio del server sul valore della sessione;
   * un `502` dice che il server non c'era, che è un'altra cosa.
   */
  it('un 5xx al rinnovo lascia la sessione com’è, e il rinnovo dopo riesce', async () => {
    let ended = 0
    let outage = false
    const api = new RotatingApi()
    const session = new SessionStore({
      fetch: async (input, init) => (outage ? problem(502, 'proxy.bad_gateway') : api.fetch(input, init)),
      onSessionEnded: () => {
        ended += 1
      },
    })
    await session.refresh()
    expect(session.current).not.toBeNull()

    outage = true
    await expect(session.refresh()).rejects.toMatchObject({ status: 502 })
    expect(session.current).not.toBeNull()
    expect(ended).toBe(0)

    // L'API è tornata: il cookie è ancora buono, e nessuno deve rifare il login.
    outage = false
    await expect(session.refresh()).resolves.toMatchObject({ userId: expect.any(String) })
    expect(session.current).not.toBeNull()
  })
})

/**
 * **Due schede**, che è il difetto che il singolo volo non poteva coprire.
 *
 * Due `SessionStore` sono due contesti JavaScript quanto basta per questo test:
 * ognuno ha la propria promessa in memoria, e nessuna delle due sa dell'altra.
 * Il cookie invece è **uno solo** — lo tiene `RotatingApi`, che qui fa da browser
 * oltre che da server — ed è esattamente la condizione reale: ciò che le schede
 * condividono è il cookie, non la sessione.
 *
 * Lo scenario non è di laboratorio: è accaduto durante il collaudo della fetta
 * B-bis, con due schede aperte sulla stessa origine, e nei log dell'API si è
 * letto `riuso di un refresh token già ruotato: revocate tutte le sessioni`.
 */
describe('il refresh fra schede', () => {
  /**
   * **Il difetto, riprodotto al contrario**, come per il singolo volo.
   *
   * Senza il lock le due schede presentano lo stesso refresh token: la prima
   * ruota, la seconda presenta quello vecchio, e l'API — che tratta il riuso come
   * prova di una copia rubata — revoca la catena. Chi guardava la seconda scheda
   * si ritrova fuori senza aver fatto niente.
   *
   * Serve a quello a cui serviva il suo gemello: senza questo test, quello qui
   * sotto passerebbe anche con il lock rotto.
   */
  it('senza il lock, due schede che rinnovano insieme fanno revocare la catena', async () => {
    const api = new RotatingApi()
    const primaScheda = new SessionStore({ fetch: api.fetch, crossTabLock: false })
    const secondaScheda = new SessionStore({ fetch: api.fetch, crossTabLock: false })

    await primaScheda.refresh()
    await secondaScheda.refresh()
    api.resetCounters()

    const esiti = await Promise.allSettled([primaScheda.refresh(), secondaScheda.refresh()])

    expect(api.refreshCalls.length).toBe(2)
    // Lo stesso token presentato due volte: è **questo** che l'API punisce.
    expect(api.refreshCalls[0]).toBe(api.refreshCalls[1])
    const rifiutati = esiti.filter((esito) => esito.status === 'rejected')
    expect(rifiutati.length).toBeGreaterThan(0)
    const caduto = rifiutati[0]
    if (caduto && caduto.status === 'rejected') {
      expect(caduto.reason).toBeInstanceOf(ApiError)
      expect((caduto.reason as ApiError).status).toBe(401)
    }
  })

  /**
   * Lo stesso scenario con il lock: **due** rinnovi, e nessuno dei due rifiutato.
   *
   * Due e non uno, ed è la decisione e non una svista: le schede non si passano
   * l'access token, che resta in memoria e non deve uscirne. La seconda aspetta,
   * poi rinnova per sé presentando il cookie **ruotato** — cioè una rotazione
   * legittima, che è tutto ciò che serviva.
   */
  it('con il lock, due schede rinnovano una alla volta e restano dentro', async () => {
    const api = new RotatingApi()
    const primaScheda = new SessionStore({ fetch: api.fetch })
    const secondaScheda = new SessionStore({ fetch: api.fetch })

    await primaScheda.refresh()
    await secondaScheda.refresh()
    api.resetCounters()

    const sessioni = await Promise.all([primaScheda.refresh(), secondaScheda.refresh()])

    expect(api.refreshCalls.length).toBe(2)
    // Token **diversi**: la seconda ha atteso e ha presentato quello nuovo.
    expect(api.refreshCalls[0]).not.toBe(api.refreshCalls[1])
    expect(sessioni.every((sessione) => sessione.accessToken !== '')).toBe(true)
    expect(primaScheda.current).not.toBeNull()
    expect(secondaScheda.current).not.toBeNull()
  })

  /**
   * Che cosa il ripiego scrive, che è la parte facile da sbagliare.
   *
   * Il lock su `localStorage` esiste dove i Web Locks non ci sono — ed è il ramo
   * che questi test percorrono, perché jsdom non ha `navigator.locks`. Deve
   * scrivere **un marcatore con una scadenza e mai un token**: metterci la
   * sessione lo renderebbe leggibile a uno script qualunque, che è esattamente la
   * proprietà per cui l'access token non lascia la memoria.
   */
  it('il ripiego scrive un marcatore con una scadenza, e nessun segreto', async () => {
    const api = new RotatingApi()
    const session = new SessionStore({ fetch: api.fetch })

    let scrittoDurante: string | null = null
    const originale = localStorage.setItem.bind(localStorage)
    const spia = vi
      .spyOn(Storage.prototype, 'setItem')
      .mockImplementation((chiave: string, valore: string) => {
        if (chiave.includes('refresh')) scrittoDurante = valore
        originale(chiave, valore)
      })

    try {
      const sessione = await session.refresh()

      expect(scrittoDurante).not.toBeNull()
      const marcatore = JSON.parse(scrittoDurante ?? '{}') as Record<string, unknown>
      // Due campi, e nessuno dei due è un segreto.
      expect(Object.keys(marcatore).sort()).toEqual(['holder', 'until'])
      expect(typeof marcatore['until']).toBe('number')
      expect(marcatore['until']).toBeGreaterThan(Date.now())
      // La prova esplicita: il token consegnato non compare da nessuna parte.
      expect(scrittoDurante).not.toContain(sessione.accessToken)
    } finally {
      spia.mockRestore()
    }
  })

  /** Il marcatore si **rilascia**: la scheda successiva non deve aspettare. */
  it('il marcatore sparisce a rinnovo concluso', async () => {
    const api = new RotatingApi()
    const session = new SessionStore({ fetch: api.fetch })

    await session.refresh()

    const rimasti = Object.keys(localStorage).filter((chiave) => chiave.includes('refresh'))
    expect(rimasti).toEqual([])
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
