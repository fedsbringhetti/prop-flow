import { ApiError, apiErrorFrom, offlineError } from './problem'
import type { FetchLike, SessionStore } from './session'

export interface ApiRequestInit {
  method?: 'GET' | 'POST' | 'PATCH' | 'DELETE'
  /** Corpo JSON. Escluso con `formData`. */
  body?: unknown
  /** Corpo `multipart/form-data`: il browser mette da sé il `Content-Type`. */
  formData?: FormData
  /** Header aggiuntivi, per esempio `Idempotency-Key`. */
  headers?: Record<string, string>
  /**
   * Non mandare `X-Org-Id`.
   *
   * Serve all'accettazione di un invito, che è l'operazione che **crea**
   * l'appartenenza: dichiarare un'organizzazione di cui non si è ancora membri
   * farebbe rispondere `403` a una richiesta legittima.
   */
  withoutOrganization?: boolean
  signal?: AbortSignal
}

export interface ApiClientOptions {
  baseUrl?: string
  fetch?: FetchLike
  session: SessionStore
}

/**
 * L'unico posto da cui si parla con l'API.
 *
 * **Nessun `fetch` fuori da qui**, ed è l'equivalente della regola «nessun SQL
 * negli handler» del backend: un componente che chiamasse l'API da sé aggirerebbe
 * l'allegato del token, l'organizzazione attiva, la traduzione degli errori e —
 * soprattutto — il rinnovo della sessione, che è la parte in cui sbagliare costa
 * la sessione dell'utente.
 */
export class ApiClient {
  private readonly baseUrl: string
  private readonly fetchImpl: FetchLike
  private readonly session: SessionStore

  constructor(options: ApiClientOptions) {
    this.baseUrl = options.baseUrl ?? '/api/v1'
    this.fetchImpl = options.fetch ?? ((input, init) => fetch(input, init))
    this.session = options.session
  }

  /**
   * Esegue la richiesta e, se il token è scaduto, la **riprova una volta sola**
   * dopo aver rinnovato.
   *
   * Una volta sola e non in ciclo: se anche dopo un rinnovo riuscito la risposta è
   * `401`, il problema non è il token, e riprovare ancora produrrebbe soltanto
   * altre chiamate di refresh — cioè, con l'API che ruota, il modo più rapido di
   * far revocare la catena.
   */
  async request<T>(path: string, init: ApiRequestInit = {}): Promise<T> {
    const response = await this.send(path, init)

    if (response.status === 401) {
      const error = await apiErrorFrom(response)
      // Sulle rotte di autenticazione un `401` è la risposta, non un sintomo:
      // rinnovare dopo un login sbagliato non ha senso.
      if (!error.isSessionProblem || path.startsWith('/auth/')) throw error

      await this.session.refresh()
      const retried = await this.send(path, init)
      return this.decode<T>(retried)
    }

    return this.decode<T>(response)
  }

  private async send(path: string, init: ApiRequestInit): Promise<Response> {
    const headers: Record<string, string> = { accept: 'application/json', ...init.headers }

    const session = this.session.current
    if (session) headers['authorization'] = `Bearer ${session.accessToken}`

    // **Sempre**, anche quando coincide con l'organizzazione del token. È ciò che
    // rende l'organizzazione attiva uno stato dichiarato invece di un default che
    // si scopre sbagliato guardando un elenco vuoto.
    if (session?.organizationId && !init.withoutOrganization) {
      headers['x-org-id'] = session.organizationId
    }

    let body: BodyInit | undefined
    if (init.formData) {
      body = init.formData
    } else if (init.body !== undefined) {
      headers['content-type'] = 'application/json'
      body = JSON.stringify(init.body)
    }

    try {
      return await this.fetchImpl(`${this.baseUrl}${path}`, {
        method: init.method ?? 'GET',
        credentials: 'same-origin',
        headers,
        ...(body === undefined ? {} : { body }),
        ...(init.signal ? { signal: init.signal } : {}),
      })
    } catch (error) {
      // Un `AbortError` è una richiesta che *noi* abbiamo annullato — un filtro
      // cambiato, una schermata smontata — e non un problema di rete da mostrare.
      if (error instanceof DOMException && error.name === 'AbortError') throw error
      throw offlineError()
    }
  }

  private async decode<T>(response: Response): Promise<T> {
    if (!response.ok) throw await apiErrorFrom(response)
    // `204` è la risposta delle archiviazioni: nessun corpo da leggere, e
    // chiamare `json()` su di esso solleverebbe.
    if (response.status === 204) return undefined as T
    return (await response.json()) as T
  }

  get<T>(path: string, init: Omit<ApiRequestInit, 'method' | 'body'> = {}): Promise<T> {
    return this.request<T>(path, { ...init, method: 'GET' })
  }

  post<T>(path: string, body?: unknown, init: Omit<ApiRequestInit, 'method'> = {}): Promise<T> {
    return this.request<T>(path, { ...init, method: 'POST', ...(body === undefined ? {} : { body }) })
  }

  patch<T>(path: string, body: unknown, init: Omit<ApiRequestInit, 'method'> = {}): Promise<T> {
    return this.request<T>(path, { ...init, method: 'PATCH', body })
  }

  delete(path: string, init: Omit<ApiRequestInit, 'method' | 'body'> = {}): Promise<void> {
    return this.request<void>(path, { ...init, method: 'DELETE' })
  }
}

export { ApiError }
