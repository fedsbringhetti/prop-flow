import { t } from '@/i18n'

/** Errore per campo, come lo manda l'API sulle validazioni. */
export interface FieldError {
  field: string
  message: string
}

/** Il corpo `application/problem+json` di RFC 9457, come lo manda questa API. */
export interface ProblemDetails {
  type: string
  title: string
  status: number
  code: string
  detail?: string
  request_id?: string
  errors?: FieldError[]
}

/**
 * Un errore dell'API, con il `code` in evidenza.
 *
 * **Si instrada sul `code` e mai sul messaggio.** `title` e `detail` sono in
 * inglese e sono per chi legge i log: possono essere riscritti in qualunque
 * momento quando si trova una formulazione migliore, e un client che li
 * confrontasse si romperebbe in silenzio. `code` è il contratto.
 *
 * `status === 0` è il caso in cui la richiesta non è nemmeno partita — rete
 * assente, server irraggiungibile — e non un errore dell'API. Va distinto perché
 * il rimedio è diverso: riprovare, non correggere.
 */
export class ApiError extends Error {
  readonly status: number
  readonly code: string
  readonly detail: string | undefined
  readonly requestId: string | undefined
  readonly fieldErrors: readonly FieldError[]

  constructor(problem: {
    status: number
    code: string
    title?: string
    detail?: string | undefined
    requestId?: string | undefined
    fieldErrors?: readonly FieldError[]
  }) {
    super(problem.title ?? problem.code)
    this.name = 'ApiError'
    this.status = problem.status
    this.code = problem.code
    this.detail = problem.detail
    this.requestId = problem.requestId
    this.fieldErrors = problem.fieldErrors ?? []
  }

  /** Se la sessione è il problema, e quindi un refresh può risolverlo. */
  get isSessionProblem(): boolean {
    return (
      this.status === 401 &&
      (this.code === 'auth.token_expired' ||
        this.code === 'auth.token_invalid' ||
        this.code === 'auth.token_missing')
    )
  }

  /** Se la rete non ha nemmeno raggiunto il server. */
  get isOffline(): boolean {
    return this.status === 0
  }
}

const messages: Record<string, string> = t.errors

/**
 * Il messaggio da mostrare, scelto sul `code`.
 *
 * Il ripiego non è il `title` inglese dell'API: sarebbe un testo di sistema in
 * mezzo a un'interfaccia italiana, e mostrarlo insegnerebbe a fidarsene. Un codice
 * senza traduzione produce il messaggio generico, e la mancanza si vede in
 * sviluppo perché il codice esce nel dettaglio tecnico.
 */
export function messageFor(error: unknown): string {
  if (!(error instanceof ApiError)) return t.errors.generic
  if (error.isOffline) return t.errors.network
  return messages[error.code] ?? t.errors.generic
}

/** Il messaggio per un campo, se l'API lo ha segnalato. */
export function fieldErrorFor(error: unknown, field: string): string | undefined {
  if (!(error instanceof ApiError)) return undefined
  return error.fieldErrors.find((entry) => entry.field === field)?.message
}

/**
 * Costruisce l'errore da una risposta non riuscita.
 *
 * Un corpo che non è JSON non è un caso da nascondere: un `502` di un proxy
 * risponde HTML, e senza questo ramo l'eccezione di parsing sostituirebbe lo stato
 * vero con un errore che non dice niente.
 */
export async function apiErrorFrom(response: Response): Promise<ApiError> {
  let problem: Partial<ProblemDetails> = {}
  try {
    problem = (await response.json()) as Partial<ProblemDetails>
  } catch {
    problem = {}
  }
  return new ApiError({
    status: response.status,
    code: problem.code ?? `http.${response.status}`,
    ...(problem.title === undefined ? {} : { title: problem.title }),
    ...(problem.detail === undefined ? {} : { detail: problem.detail }),
    ...(problem.request_id === undefined ? {} : { requestId: problem.request_id }),
    ...(problem.errors === undefined ? {} : { fieldErrors: problem.errors }),
  })
}

/** L'errore di una richiesta che non è partita. */
export function offlineError(): ApiError {
  return new ApiError({ status: 0, code: 'network.unreachable' })
}
