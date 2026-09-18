import { ApiError } from './problem'

/**
 * Un'azione su N righe, fatta con **N chiamate in sequenza** e un esito per riga.
 *
 * Nessuna rotta accetta più identificatori, ed è una decisione e non una lacuna
 * (sezione 7, decisione 2): le rotte singole autorizzano riga per riga sotto Row
 * Level Security, rispondono con un codice che dice perché, e ripeterle su una
 * riga già fatta risponde con un codice riconoscibile. Una rotta in blocco
 * avrebbe chiesto una semantica transazionale che non ha una risposta buona.
 *
 * # In sequenza, non in parallelo
 *
 * Una pagina sono al più venticinque righe, l'esito si legge nell'ordine della
 * tabella, e soprattutto: se l'access token scade a metà del giro, **una**
 * richiesta riceve `401`, **un** rinnovo la ripara, e le successive partono con
 * il token nuovo. In parallelo sarebbero venticinque `401` insieme, che il singolo
 * volo di `SessionStore` reggerebbe ma che non c'è ragione di provocare. Il test
 * di questo modulo fa scadere il token a metà con il singolo volo **spento**: se
 * qualcuno rende il giro parallelo, il test lo dice.
 *
 * # Tre esiti, e il terzo è quello progettato
 *
 * | Esito     | Come si riconosce                                   |
 * |-----------|-----------------------------------------------------|
 * | `done`    | `2xx`                                               |
 * | `already` | `409` «era già così», oppure `404` — vedi sotto     |
 * | `failed`  | `403`, altri `409`, `422`, `5xx`, rete              |
 *
 * **«Era già così» non è un errore.** Il `409 *.invalid_transition` o
 * `*.archived` su una riga già nello stato chiesto è la conferma che lo stato c'è:
 * mostrarlo in rosso lascerebbe righe selezionate che non si possono «sistemare».
 *
 * **E il `404` è della stessa famiglia**, ed è la parte che il piano non aveva
 * previsto: un immobile archiviato non risponde `409 property.archived` come il
 * piano diceva — il repository legge con `deleted_at IS NULL`, quindi una seconda
 * archiviazione trova il nulla e risponde `404 property.not_found`. Vale per
 * tutta la famiglia del soft delete: immobili, stanze, documenti, spese. Una riga
 * che era in tabella e adesso risponde `404` è una riga che la prossima rilettura
 * non mostrerà — le liste passano dalle stesse policy e dallo stesso filtro — e
 * un motivo scritto accanto a una riga che sta per sparire non dice niente a
 * nessuno.
 *
 * # Il giro non si ferma, e non ritenta
 *
 * Fermarsi al primo errore lascerebbe le righe dopo in uno stato ignoto — «le ho
 * annullate o no?» — che è peggio di un errore per riga. E ritentare da soli
 * nasconderebbe il motivo: un `403` non cambia riprovando, un `404` nemmeno, e su
 * un errore di rete la persona ha già il bottone davanti. Le righe non fatte
 * **restano selezionate**, così riprovarle è un clic.
 */
export type BatchResult = 'done' | 'already' | 'failed'

export interface BatchOutcome {
  id: string
  result: BatchResult
  /** Presente solo su `failed`: è ciò che si mostra accanto alla riga, tradotto
   *  da `messageFor`, che sa leggere anche ciò che non è un `ApiError`. */
  error?: unknown
}

export interface BatchSummary {
  outcomes: BatchOutcome[]
  /** Le righe che sono uscite dallo stato di partenza: fatte o già così. */
  settled: string[]
  /** Le righe da riprovare, con il loro errore. */
  failed: BatchOutcome[]
}

/**
 * I codici `409` che dicono «era già così».
 *
 * L'elenco è scritto a mano perché la forma dei codici non basta a riconoscerli:
 * `expense.payer_not_in_group` è un `409` e **non** è «già così». Un codice nuovo
 * di questa famiglia va aggiunto qui, e il test lo ricorda.
 */
const ALREADY_SO = new Set([
  'property.archived',
  'room.archived',
  'document.archived',
  'expense_group.archived',
  'ticket.cancelled',
  'ticket.invalid_transition',
  'charge.invalid_transition',
  'utility_bill.invalid_transition',
  'lease.invalid_transition',
  'payment.invalid_transition',
  'settlement.invalid_transition',
  'expense_group.membership_unchanged',
])

/** Classifica l'errore di una riga. Esportata per il test, non per i componenti. */
export function classifyFailure(error: unknown): Exclude<BatchResult, 'done'> {
  if (!(error instanceof ApiError)) return 'failed'
  if (error.status === 404) return 'already'
  if (error.status === 409 && ALREADY_SO.has(error.code)) return 'already'
  return 'failed'
}

/**
 * Esegue `action` su ogni identificatore, **uno alla volta**, e restituisce un
 * esito per riga nell'ordine ricevuto.
 *
 * `onProgress` riceve quante righe sono state percorse: è ciò che la barra mostra
 * mentre il giro è in corso.
 */
export async function runInSequence(
  ids: readonly string[],
  action: (id: string) => Promise<unknown>,
  onProgress?: (done: number, total: number) => void,
): Promise<BatchSummary> {
  const outcomes: BatchOutcome[] = []

  for (const id of ids) {
    try {
      await action(id)
      outcomes.push({ id, result: 'done' })
    } catch (error) {
      const result = classifyFailure(error)
      outcomes.push(result === 'failed' ? { id, result, error } : { id, result })
    }
    onProgress?.(outcomes.length, ids.length)
  }

  return {
    outcomes,
    settled: outcomes.filter((outcome) => outcome.result !== 'failed').map((outcome) => outcome.id),
    failed: outcomes.filter((outcome) => outcome.result === 'failed'),
  }
}
