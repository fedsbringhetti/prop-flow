import { ApiError, apiErrorFrom, offlineError } from './problem'

export type OrgRole = 'owner' | 'manager' | 'tenant'

/** Un'organizzazione di cui l'utente è membro, come il client l'ha conosciuta. */
export interface Membership {
  organizationId: string
  role: OrgRole
}

/**
 * La sessione viva, **in memoria e solo in memoria**.
 *
 * L'access token non sta in `localStorage` né in `sessionStorage`: non è leggibile
 * da uno script che arrivi dopo il caricamento della pagina, e muore al reload.
 * Che muoia non è un problema — si riottiene dal cookie del refresh, che il
 * browser manda da sé e che nessuno script può leggere.
 */
export interface Session {
  accessToken: string
  /** Secondi di vita dichiarati dall'API alla consegna. */
  expiresIn: number
  userId: string
  organizationId: string | null
  role: OrgRole | null
}

export type FetchLike = (input: string, init?: RequestInit) => Promise<Response>

interface SessionResponseBody {
  access_token: string
  token_type: string
  expires_in: number
  organization_id?: string
  role?: OrgRole
  user_id: string
}

export interface SessionStoreOptions {
  baseUrl?: string
  fetch?: FetchLike
  /**
   * Il refresh a **singolo volo**. Vero sempre, tranne in un test.
   *
   * Disattivarlo esiste per una ragione sola: poter riprodurre il difetto che il
   * singolo volo evita. Vedi il commento su {@link SessionStore.refresh}.
   */
  singleFlight?: boolean
  /** Chiamato quando la sessione cade: il guscio rimanda al login. */
  onSessionEnded?: () => void
}

/**
 * Custodisce la sessione e sa rinnovarla.
 *
 * # Il refresh a singolo volo, che è una condizione di **correttezza**
 *
 * L'API ruota il refresh token a ogni uso e tratta il riuso come prova che
 * qualcuno ne ha una copia: revoca l'intera catena delle sessioni dell'utente.
 * È corretto e severo, e in un browser diventa un difetto se il client è ingenuo.
 *
 * Lo scenario **accade**, non è ipotetico: una schermata fa tre richieste in
 * parallelo, l'access token è scaduto, tutte e tre ricevono `401`, tutte e tre
 * chiamano `/auth/refresh` **con lo stesso cookie**. La prima ruota il token; la
 * seconda e la terza presentano un token già ruotato; l'API revoca tutto;
 * l'utente si ritrova fuori senza aver fatto niente.
 *
 * Il rimedio è una promessa condivisa: chi arriva mentre una chiamata è in volo
 * **aspetta quella** invece di farne una propria. Non è un'ottimizzazione — non
 * risparmia una richiesta, evita una revoca.
 *
 * # Quello che resta fuori, e va saputo
 *
 * Fra **schede diverse** il problema resta, e una promessa in memoria non lo
 * risolve: due schede sono due contesti JavaScript. Servirebbe un coordinamento
 * (`BroadcastChannel`, o un lock su `localStorage`). In questa fetta si accetta:
 * il sintomo è raro — due schede il cui access token scade nello stesso istante —
 * e il rimedio è ricaricare. È registrato fra i debiti noti.
 */
export class SessionStore {
  private session: Session | null = null
  private inFlight: Promise<Session> | null = null
  private readonly listeners = new Set<() => void>()
  private readonly baseUrl: string
  private readonly fetchImpl: FetchLike
  private readonly singleFlight: boolean
  private readonly onSessionEnded: (() => void) | undefined

  constructor(options: SessionStoreOptions = {}) {
    this.baseUrl = options.baseUrl ?? '/api/v1'
    this.fetchImpl = options.fetch ?? ((input, init) => fetch(input, init))
    this.singleFlight = options.singleFlight ?? true
    this.onSessionEnded = options.onSessionEnded
  }

  get current(): Session | null {
    return this.session
  }

  /** Per `useSyncExternalStore`: identità stabile finché la sessione non cambia. */
  readonly snapshot = (): Session | null => this.session

  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  private emit(): void {
    for (const listener of this.listeners) listener()
  }

  private adopt(body: SessionResponseBody): Session {
    const session: Session = {
      accessToken: body.access_token,
      expiresIn: body.expires_in,
      userId: body.user_id,
      organizationId: body.organization_id ?? null,
      role: body.role ?? null,
    }
    this.session = session
    this.emit()
    return session
  }

  /**
   * L'organizzazione attiva è **stato esplicito**, non un default implicito.
   *
   * Senza `X-Org-Id` gli elenchi rispondono `200` con zero elementi — una
   * schermata vuota, nessun errore, nessun indizio. Tenerla qui e mandarla sempre
   * è ciò che impedisce quel sintomo, che è il modo peggiore di sbagliare.
   */
  selectOrganization(organizationId: string, role: OrgRole): void {
    if (!this.session) return
    this.session = { ...this.session, organizationId, role }
    this.emit()
  }

  clear(): void {
    if (this.session === null) return
    this.session = null
    this.emit()
    this.onSessionEnded?.()
  }

  /**
   * Rinnova la sessione, **una chiamata alla volta**.
   *
   * Chi arriva mentre una è in volo riceve la stessa promessa e riparte con il
   * token nuovo. Con `singleFlight: false` ognuno fa la propria: è il difetto,
   * riprodotto apposta in `session.test.ts`.
   */
  refresh(): Promise<Session> {
    if (this.singleFlight && this.inFlight) return this.inFlight

    const attempt = this.performRefresh().finally(() => {
      // Solo se è ancora la propria: una chiamata più recente non va azzerata da
      // una vecchia che si conclude dopo.
      if (this.inFlight === attempt) this.inFlight = null
    })
    if (this.singleFlight) this.inFlight = attempt
    return attempt
  }

  private async performRefresh(): Promise<Session> {
    let response: Response
    try {
      response = await this.fetchImpl(`${this.baseUrl}/auth/refresh`, {
        method: 'POST',
        // Il cookie è di prima parte — client e API stanno sulla stessa origine
        // dietro un reverse proxy — quindi `same-origin` basta e `include` non
        // servirebbe a nulla se non ad ammettere una configurazione che non c'è.
        credentials: 'same-origin',
        headers: { accept: 'application/json' },
      })
    } catch {
      // La rete non ha raggiunto il server: la sessione **non** si butta via. Un
      // tunnel in galleria non è un logout, e cancellarla qui manderebbe al login
      // chi ha solo perso il segnale.
      throw offlineError()
    }

    if (!response.ok) {
      this.clear()
      throw await apiErrorFrom(response)
    }
    return this.adopt((await response.json()) as SessionResponseBody)
  }

  async login(email: string, password: string): Promise<Session> {
    const response = await this.post('/auth/login', { email, password })
    return this.adopt((await response.json()) as SessionResponseBody)
  }

  async register(input: {
    email: string
    fullName: string
    password: string
    organizationName?: string | undefined
  }): Promise<void> {
    await this.post('/auth/register', {
      email: input.email,
      full_name: input.fullName,
      password: input.password,
      ...(input.organizationName ? { organization_name: input.organizationName } : {}),
    })
  }

  /**
   * Esce, e **cancella la sessione locale comunque**.
   *
   * Se la chiamata fallisce — rete assente, token già scaduto — l'utente ha
   * chiesto di uscire e deve uscire: lasciarlo dentro perché il server non ha
   * risposto sarebbe il contrario di ciò che ha chiesto. Il cookie lo cancella la
   * risposta quando arriva; la sessione in memoria muore subito.
   */
  async logout(): Promise<void> {
    const token = this.session?.accessToken
    try {
      if (token) {
        await this.fetchImpl(`${this.baseUrl}/auth/logout`, {
          method: 'POST',
          credentials: 'same-origin',
          headers: { authorization: `Bearer ${token}`, accept: 'application/json' },
        })
      }
    } catch {
      // Ignorato di proposito: vedi sopra.
    } finally {
      this.clear()
    }
  }

  private async post(path: string, body: unknown): Promise<Response> {
    let response: Response
    try {
      response = await this.fetchImpl(`${this.baseUrl}${path}`, {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'content-type': 'application/json', accept: 'application/json' },
        body: JSON.stringify(body),
      })
    } catch {
      throw offlineError()
    }
    if (!response.ok) throw await apiErrorFrom(response)
    return response
  }
}

export { ApiError }
