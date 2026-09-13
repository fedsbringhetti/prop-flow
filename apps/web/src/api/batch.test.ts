import { describe, expect, it } from 'vitest'

import { classifyFailure, runInSequence } from './batch'
import { ApiClient } from './client'
import { ApiError } from './problem'
import { SessionStore, type FetchLike } from './session'

/**
 * Il giro su N righe, verificato **al contrario** dove conta.
 *
 * Il test che importa è quello dell'access token che scade a metà: è scritto con
 * il singolo volo **spento** e contro un'API finta che ruota il refresh token e
 * revoca la catena al riuso, come quella vera. In sequenza, una richiesta riceve
 * `401`, un rinnovo la ripara e le altre passano; in parallelo tre `401` insieme
 * fanno tre rinnovi con lo stesso cookie, e la catena cade. Lo stesso file
 * esegue anche la versione parallela e prova che fallisce: senza quella metà, il
 * test passerebbe anche con un giro parallelo protetto dal singolo volo — cioè
 * non proverebbe niente sulla sequenza.
 */

/**
 * Un'API finta che fa due cose vere: ruota il refresh token e conta quante
 * richieste di business ha **in volo** nello stesso istante.
 */
class RotatingApi {
  readonly calls: string[] = []
  /** Il massimo di richieste di business in volo insieme, osservato. */
  maxInFlight = 0
  private inFlight = 0
  private currentRefreshToken = 'refresh-1'
  private issued = 1
  private chainRevoked = false
  private accessToken = 'access-1'
  /** Dopo quante richieste di business l'access token scade. */
  private expireAfter: number | null = null
  private served = 0

  expireAccessTokenAfter(calls: number): void {
    this.expireAfter = calls
  }

  readonly fetch: FetchLike = async (input, init) => {
    const url = new URL(input, 'http://propflow.test')
    if (url.pathname === '/api/v1/auth/refresh') return this.refresh()

    this.inFlight += 1
    this.maxInFlight = Math.max(this.maxInFlight, this.inFlight)
    try {
      await delay(2)
      this.calls.push(url.pathname)
      const authorization = headerOf(init, 'authorization')
      if (authorization !== `Bearer ${this.accessToken}`) {
        return problem(401, 'auth.token_expired')
      }
      this.served += 1
      if (this.expireAfter !== null && this.served === this.expireAfter) {
        // Da qui il token in circolazione non vale più: la prossima richiesta
        // che lo presenta riceve `401`.
        this.accessToken = `access-scaduto-${this.issued}`
      }
      return json(200, { ok: true })
    } finally {
      this.inFlight -= 1
    }
  }

  private async refresh(): Promise<Response> {
    // Il cookie lo manderebbe il browser; qui il token presentato è quello
    // corrente **al momento della chiamata**. Due rinnovi che partono insieme
    // presentano lo stesso token: il primo ruota, il secondo è un riuso, e la
    // catena cade — esattamente `RefreshSession::ensure_usable`.
    const presented = this.currentRefreshToken
    await delay(5)
    if (this.chainRevoked) return problem(401, 'auth.invalid_credentials')
    if (presented !== this.currentRefreshToken) {
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
  return (headers as Record<string, string>)[name] ?? null
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

/** Un client con il singolo volo **spento**, così il parallelismo si vede. */
async function clientWithoutSingleFlight(api: RotatingApi): Promise<ApiClient> {
  const session = new SessionStore({ fetch: api.fetch, singleFlight: false, crossTabLock: false })
  await session.refresh()
  return new ApiClient({ fetch: api.fetch, session })
}

const IDS = ['a', 'b', 'c', 'd', 'e']

describe('il giro su N righe', () => {
  it('è in sequenza: mai più di una richiesta in volo', async () => {
    const api = new RotatingApi()
    const client = await clientWithoutSingleFlight(api)

    await runInSequence(IDS, (id) => client.post(`/things/${id}/archive`))

    expect(api.maxInFlight).toBe(1)
    expect(api.calls).toEqual(IDS.map((id) => `/api/v1/things/${id}/archive`))
  })

  it('con il token che scade a metà, in sequenza tutte le righe passano', async () => {
    const api = new RotatingApi()
    const client = await clientWithoutSingleFlight(api)
    api.expireAccessTokenAfter(2)

    const summary = await runInSequence(IDS, (id) => client.post(`/things/${id}/archive`))

    expect(summary.failed).toEqual([])
    expect(summary.settled).toEqual(IDS)
  })

  it('la stessa scadenza, con il giro parallelo, fa cadere la catena: è la prova che il test discrimina', async () => {
    const api = new RotatingApi()
    const client = await clientWithoutSingleFlight(api)
    api.expireAccessTokenAfter(2)

    // Il difetto, riprodotto apposta: se qualcuno riscrive `runInSequence` con
    // `Promise.all`, il test sopra si comporta come questo.
    const outcomes = await Promise.allSettled(
      IDS.map((id) => client.post(`/things/${id}/archive`)),
    )

    expect(api.maxInFlight).toBeGreaterThan(1)
    expect(outcomes.some((outcome) => outcome.status === 'rejected')).toBe(true)
  })

  it('non si ferma al primo errore, e non ritenta', async () => {
    const attempts = new Map<string, number>()
    const summary = await runInSequence(IDS, async (id) => {
      attempts.set(id, (attempts.get(id) ?? 0) + 1)
      if (id === 'b') throw new ApiError({ status: 403, code: 'org.staff_role_required' })
      if (id === 'd') throw new ApiError({ status: 0, code: 'network.unreachable' })
    })

    // Tutte e cinque percorse, una volta ciascuna.
    expect([...attempts.keys()]).toEqual(IDS)
    expect([...attempts.values()]).toEqual([1, 1, 1, 1, 1])
    expect(summary.failed.map((outcome) => outcome.id)).toEqual(['b', 'd'])
    expect(summary.settled).toEqual(['a', 'c', 'e'])
  })

  it('riporta l’avanzamento a ogni riga', async () => {
    const seen: number[] = []
    await runInSequence(IDS, async () => undefined, (done) => seen.push(done))
    expect(seen).toEqual([1, 2, 3, 4, 5])
  })
})

describe('«era già così» non è un errore', () => {
  it('un 409 di transizione o di archiviazione conta come fatto', () => {
    for (const code of [
      'charge.invalid_transition',
      'utility_bill.invalid_transition',
      'ticket.invalid_transition',
      'ticket.cancelled',
      'property.archived',
      'room.archived',
      'document.archived',
    ]) {
      expect(classifyFailure(new ApiError({ status: 409, code }))).toBe('already')
    }
  })

  it('un 404 conta come fatto: la riga non ci sarà alla rilettura', () => {
    // È la premessa che il piano aveva sbagliata: archiviare un immobile già
    // archiviato non dà `409 property.archived` ma `404 property.not_found`,
    // perché il repository legge con `deleted_at IS NULL`.
    expect(classifyFailure(new ApiError({ status: 404, code: 'property.not_found' }))).toBe(
      'already',
    )
  })

  it('un 409 che non è «già così» resta un errore', () => {
    expect(classifyFailure(new ApiError({ status: 409, code: 'expense.payer_not_in_group' }))).toBe(
      'failed',
    )
  })

  it('un 403, un 422, un 5xx e la rete restano errori, con il motivo', () => {
    for (const [status, code] of [
      [403, 'org.staff_role_required'],
      [422, 'validation.failed'],
      [500, 'http.500'],
      [0, 'network.unreachable'],
    ] as const) {
      expect(classifyFailure(new ApiError({ status, code }))).toBe('failed')
    }
    expect(classifyFailure(new Error('altro'))).toBe('failed')
  })
})
