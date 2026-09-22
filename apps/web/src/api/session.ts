import { ApiError, apiErrorFrom, offlineError } from './problem'
import type { OrgRole } from './types'

export type { OrgRole }

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
  /**
   * Il lock **fra schede**. Vero sempre, tranne in un test.
   *
   * Disattivarlo esiste per la stessa ragione di `singleFlight`: poter
   * riprodurre il difetto che il lock evita. Vedi {@link withRefreshLock}.
   */
  crossTabLock?: boolean
  /** Chiamato quando la sessione cade: il guscio rimanda al login. */
  onSessionEnded?: () => void
}

/**
 * Il nome del lock, per origine.
 *
 * Le schede della stessa origine condividono **il cookie**, quindi condividono
 * anche ciò che va serializzato. Un nome per utente non servirebbe: due sessioni
 * di due utenti nello stesso browser sono due cookie diversi, ma il primo dei due
 * rinnovi a partire non è mai in attesa del secondo per più di un istante.
 */
const REFRESH_LOCK = 'propflow.refresh.v1'

/** Vita del marcatore di ripiego: una scheda chiusa a metà rinnovo lo libera. */
const FALLBACK_LOCK_TTL_MS = 15_000
/** Ogni quanto si guarda se il lock si è liberato. */
const FALLBACK_POLL_MS = 20
/**
 * Dopo quanto si procede **senza** il lock.
 *
 * Un rinnovo bloccato per sempre è peggio del difetto che il lock evita: là si
 * perde la sessione e si rientra, qui l'applicazione resterebbe ferma senza dire
 * perché. Venti secondi sono molto più di un rinnovo e molto meno di un'attesa
 * che qualcuno noti.
 */
const FALLBACK_GIVE_UP_MS = 20_000

interface LockMarker {
  /** Chi lo tiene. **Un identificatore, mai un token.** */
  holder: string
  /** Quando scade, in millisecondi epoch. */
  until: number
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function readMarker(): LockMarker | null {
  try {
    const raw = localStorage.getItem(REFRESH_LOCK)
    if (!raw) return null
    const parsed = JSON.parse(raw) as Partial<LockMarker>
    if (typeof parsed.holder !== 'string' || typeof parsed.until !== 'number') return null
    return { holder: parsed.holder, until: parsed.until }
  } catch {
    return null
  }
}

/**
 * Il ripiego, dove i Web Locks non ci sono.
 *
 * Scrive **un marcatore con una scadenza** e mai il token: ciò che va in
 * `localStorage` è un identificatore casuale e un istante, cioè niente che serva a
 * qualcuno che lo legga.
 *
 * `localStorage` non ha un confronta-e-scambia, quindi due schede che guardino nel
 * medesimo istante possono scrivere entrambe. Il rimedio è rileggere dopo aver
 * scritto: **l'ultima scrittura vince**, e chi si rilegge diverso riprova. Resta
 * una finestra teorica — entrambe rileggono prima che l'altra scriva — e in quel
 * caso si torna esattamente al comportamento di oggi, che è il difetto e non una
 * regressione. È il prezzo di non avere `navigator.locks`, che nei browser di
 * questo decennio c'è.
 */
async function withStorageLock<T>(run: () => Promise<T>): Promise<T> {
  const holder =
    typeof crypto !== 'undefined' && 'randomUUID' in crypto
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random()}`
  const giveUpAt = Date.now() + FALLBACK_GIVE_UP_MS
  let held = false

  for (;;) {
    const marker = readMarker()
    const free = marker === null || marker.until <= Date.now()

    if (free) {
      try {
        localStorage.setItem(
          REFRESH_LOCK,
          JSON.stringify({ holder, until: Date.now() + FALLBACK_LOCK_TTL_MS }),
        )
      } catch {
        // `localStorage` non disponibile — navigazione privata, impostazioni del
        // browser: si rinuncia al coordinamento e si rinnova comunque. Perdere il
        // lock è tornare al comportamento di prima; rifiutarsi di rinnovare
        // sarebbe una sessione che muore per una preferenza del browser.
        return run()
      }
      await delay(FALLBACK_POLL_MS)
      if (readMarker()?.holder === holder) {
        held = true
        break
      }
    }

    if (Date.now() >= giveUpAt) break
    await delay(FALLBACK_POLL_MS)
  }

  try {
    return await run()
  } finally {
    if (held) {
      try {
        // Solo se è ancora il proprio: uno scaduto e ripreso da un'altra scheda
        // non va cancellato da chi lo aveva prima.
        if (readMarker()?.holder === holder) localStorage.removeItem(REFRESH_LOCK)
      } catch {
        // Vedi sopra: il marcatore scade da sé.
      }
    }
  }
}

interface WebLockManager {
  request: <T>(name: string, callback: () => Promise<T>) => Promise<T>
}

/**
 * Serializza il rinnovo **fra le schede**, senza trasmettere niente.
 *
 * Il singolo volo è una promessa in memoria, e due schede sono due contesti
 * JavaScript: se l'access token di entrambe scade insieme, la seconda presenta un
 * refresh token già ruotato, l'API lo legge come prova di una copia rubata e
 * revoca l'intera catena. Non è un'ipotesi — è accaduto, con due schede aperte
 * sulla stessa origine e nessuno che avesse fatto niente.
 *
 * La via che viene in mente per prima è **trasmettere la sessione**: chi vince
 * rinnova e manda l'access token alle altre con `BroadcastChannel`. Funziona, e
 * rompe una proprietà scelta apposta — l'access token vive in memoria e non è
 * leggibile da uno script che arrivi dopo il caricamento. Metterlo su un canale, o
 * peggio in `localStorage`, lo espone esattamente a ciò da cui era tenuto fuori.
 *
 * **Non serve condividerlo.** Ciò che le schede condividono è il cookie, e l'API
 * ruota a ogni uso: il riuso lo rileva soltanto su *due usi dello stesso token*.
 * Se la seconda scheda aspetta che la prima abbia finito e poi rinnova a sua
 * volta, presenta il cookie **nuovo**, che è una rotazione legittima. Serve una
 * coda, non una trasmissione.
 *
 * Costa `N` rinnovi per `N` schede invece di uno: una volta ogni quindici minuti
 * per scheda, e in cambio il token non lascia la memoria.
 *
 * *Alternativa scartata: un `SharedWorker` che tenga la sessione.* È il disegno
 * giusto per un'applicazione che ne avesse bisogno, e qui sarebbe un terzo
 * contesto da far vivere, morire e ricollegare per un problema che un lock chiude
 * in venti righe.
 */
function withRefreshLock<T>(run: () => Promise<T>): Promise<T> {
  const locks = (navigator as Navigator & { locks?: WebLockManager }).locks
  if (locks) return locks.request(REFRESH_LOCK, run)
  return withStorageLock(run)
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
 * # Fra schede, che è il secondo strato
 *
 * Una promessa in memoria non attraversa i contesti JavaScript, quindi due schede
 * la aggirano per costruzione. Sopra il singolo volo c'è quindi un lock per
 * origine — {@link withRefreshLock} — che **serializza** i rinnovi invece di
 * trasmettere la sessione: chi arriva secondo aspetta, e poi rinnova a sua volta
 * presentando il cookie già ruotato, che è una rotazione legittima.
 *
 * I due strati non si sovrappongono e non sono ridondanti: il singolo volo
 * risparmia i rinnovi dentro una scheda, il lock impedisce che due schede li
 * facciano insieme.
 */
export class SessionStore {
  private session: Session | null = null
  private inFlight: Promise<Session> | null = null
  private readonly listeners = new Set<() => void>()
  private readonly baseUrl: string
  private readonly fetchImpl: FetchLike
  private readonly singleFlight: boolean
  private readonly crossTabLock: boolean
  private readonly onSessionEnded: (() => void) | undefined

  constructor(options: SessionStoreOptions = {}) {
    this.baseUrl = options.baseUrl ?? '/api/v1'
    this.fetchImpl = options.fetch ?? ((input, init) => fetch(input, init))
    this.singleFlight = options.singleFlight ?? true
    this.crossTabLock = options.crossTabLock ?? true
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

  /**
   * Nessuna organizzazione attiva: lo stato in cui si arriva uscendo
   * dall'unica che si aveva (ADR-053).
   *
   * Senza, la sessione resterebbe con l'`X-Org-Id` dell'organizzazione appena
   * lasciata, e ogni richiesta risponderebbe `403 org.not_a_member` — la stessa
   * forma del difetto del poscritto della sezione 6, qui per «zero
   * organizzazioni» invece che «organizzazione sbagliata».
   */
  clearOrganization(): void {
    if (!this.session) return
    if (this.session.organizationId === null && this.session.role === null) return
    this.session = { ...this.session, organizationId: null, role: null }
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

  /**
   * Il rinnovo, **in coda con le altre schede**.
   *
   * Il lock avvolge la chiamata e non la promessa: due schede devono poter
   * arrivare qui insieme e passare una alla volta, non condividere un risultato —
   * che è precisamente la differenza fra serializzare e trasmettere.
   */
  private performRefresh(): Promise<Session> {
    if (!this.crossTabLock) return this.sendRefresh()
    return withRefreshLock(() => this.sendRefresh())
  }

  private async sendRefresh(): Promise<Session> {
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

    // **Solo il `401` chiude la sessione**: è la risposta «questa sessione non
    // vale più», ed è l'unica. Un `502` dal proxy con l'API che si riavvia, un
    // `503` per timeout, un `500`: la sessione è nel cookie, non in memoria, e
    // cancellare la memoria non revocherebbe niente — manderebbe al login chi
    // ha un cookie buono (`R52`). L'errore si propaga alla richiesta che aveva
    // chiesto il rinnovo, ed è la schermata a mostrare «Riprova», non il guscio
    // a uscire. Simmetrico a ciò che il ripristino fa dalla sezione 10.
    //
    // Un `429` è «riprova fra poco», non un rifiuto; un `403` sul rinnovo non ha
    // oggi nessuna via che lo produca. Nessuno dei due è un giudizio sul valore
    // della sessione, quindi nessuno dei due la cancella.
    if (response.status === 401) {
      this.clear()
      throw await apiErrorFrom(response)
    }
    if (!response.ok) throw await apiErrorFrom(response)
    return this.adopt((await response.json()) as SessionResponseBody)
  }

  async login(email: string, password: string): Promise<Session> {
    const response = await this.post('/auth/login', { email, password })
    return this.adopt((await response.json()) as SessionResponseBody)
  }

  /**
   * Accetta un invito da proprietario: crea account e organizzazione dietro un
   * token di piattaforma (ADR-053), e — a differenza della vecchia
   * `POST /auth/register`, che rispondeva solo gli identificatori — **installa
   * già la sessione**, esattamente come {@link login}: chi accetta non deve
   * rifare un secondo giro.
   *
   * Non manda mai `X-Org-Id`, ma non per un flag da passare: questa classe non
   * ne manda **mai** uno, a differenza di `ApiClient` — non esiste ancora una
   * sessione da cui leggerlo. È `ApiClient.withoutOrganization` a servire dove
   * un'organizzazione attiva **potrebbe** esserci ed è quella che romperebbe la
   * richiesta; qui semplicemente non c'è nulla da dichiarare.
   */
  async acceptOwnerInvitation(input: {
    token: string
    fullName: string
    password: string
    organizationName?: string | undefined
  }): Promise<Session> {
    const response = await this.post('/auth/accept-owner-invitation', {
      token: input.token,
      full_name: input.fullName,
      password: input.password,
      ...(input.organizationName ? { organization_name: input.organizationName } : {}),
    })
    return this.adopt((await response.json()) as SessionResponseBody)
  }

  /**
   * Registra un inquilino a partire da un invito, **dentro l'organizzazione di
   * chi lo ha invitato** (ADR-053): un solo passo, non più
   * «registrati, poi accetta». Installa la sessione come {@link login}.
   *
   * Se il token è in realtà da **proprietario**, l'API risponde
   * `409 invitation.owner_token` invece di aprire una sessione: tocca a chi
   * chiama rimandare lo stesso corpo, più il nome dell'organizzazione, a
   * {@link acceptOwnerInvitation}.
   */
  async registerFromInvitation(input: {
    token: string
    fullName: string
    password: string
  }): Promise<Session> {
    const response = await this.post('/auth/register-from-invitation', {
      token: input.token,
      full_name: input.fullName,
      password: input.password,
    })
    return this.adopt((await response.json()) as SessionResponseBody)
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
