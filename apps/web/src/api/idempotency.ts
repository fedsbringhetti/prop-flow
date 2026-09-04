import { useRef, useState } from 'react'

/**
 * La chiave di idempotenza, generata **una volta per intento** e riusata a ogni
 * tentativo.
 *
 * # Perché la genera il client, e non il server
 *
 * È deliberato ed è il cuore della difesa. Se a generarla fosse il server, ogni
 * ritentativo produrrebbe una chiave nuova, quindi un **secondo pagamento**,
 * quindi un saldo che scende due volte — e `audit_log` mostrerebbe due
 * inserimenti perfettamente legittimi, cioè un errore contabile senza un solo
 * indizio di essere tale.
 *
 * # La condizione precisa, e il modo di sbagliarla
 *
 * L'intento è **l'invio del form**, non la chiamata HTTP. La chiave si genera lì
 * e si riusa per ogni tentativo successivo dello stesso intento: il secondo clic
 * su un bottone, il rinvio dopo un errore di rete, il ritentativo dopo un `5xx`.
 *
 * Generarla dentro `mutationFn` è precisamente il difetto che l'header esiste per
 * impedire: quella funzione gira **una volta per tentativo**, quindi un `Math.random`
 * o un `randomUUID()` scritto lì dentro darebbe una chiave nuova a ogni ritentativo.
 * Sembra corretto perché il codice è nello stesso posto in cui si compone il corpo,
 * e non lo è.
 *
 * Qui la chiave vive in un `ref` — non in uno stato, perché non deve provocare un
 * render — e [`IntentKey.current`] la crea solo se non c'è. Un intento nuovo si
 * dichiara con [`IntentKey.renew`], e va dichiarato: dopo un invio **riuscito**, e
 * quando il form si riapre per un pagamento diverso. Senza quel `renew`, due
 * incassi distinti condividerebbero la chiave e il secondo tornerebbe `200` con il
 * primo — l'errore opposto, e altrettanto silenzioso.
 *
 * # Il `200` non è un errore
 *
 * Una chiave ripetuta risponde `200` invece di `201`, con il pagamento già
 * registrato. Per il client i due sono **lo stesso esito riuscito**: il `409`
 * esiste per due casi che la rilettura non assorbe — la stessa chiave verso un
 * altro addebito, e una chiave usata da qualcuno di cui il chiamante non vede i
 * pagamenti — e nessuno dei due si risolve riprovando.
 */
export interface IntentKey {
  /** La chiave dell'intento in corso. La crea il primo tentativo; i successivi la riusano. */
  current: () => string
  /** Dichiara un intento nuovo: la prossima `current()` genera una chiave diversa. */
  renew: () => void
}

/**
 * Un identificatore irripetibile.
 *
 * `crypto.randomUUID` dove c'è — è quello che l'API si aspetta, e che rende la
 * collisione teorica. Il ripiego serve ai contesti non sicuri (`http://` su un
 * host che non sia `localhost`), dove `crypto.randomUUID` **non esiste**: senza,
 * un'eccezione impedirebbe di registrare un incasso invece di degradare la
 * qualità della chiave.
 */
function randomKey(): string {
  const source = globalThis.crypto
  if (source && typeof source.randomUUID === 'function') return source.randomUUID()
  // Non è un UUID e non deve sembrarlo: l'API accetta una stringa opaca.
  return `k-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 12)}`
}

export function useIntentKey(): IntentKey {
  // La chiave vive in un `ref` e non in uno stato: cambiarla non deve provocare un
  // render, perché non è qualcosa che si mostra. Nessuno la legge **durante** il
  // render — solo dentro `mutationFn` e nei gestori — che è la condizione perché
  // un `ref` sia lecito.
  const key = useRef<string | null>(null)

  // `useState` con inizializzatore pigro e non un secondo `ref`: costruisce
  // l'oggetto una volta sola e lo restituisce identico a ogni render, senza
  // leggere un `ref` in fase di render. Serve che sia stabile perché una
  // `useMutation` che ne dipendesse si ricostruirebbe a ogni render, perdendo lo
  // stato dell'invio in corso.
  const [api] = useState<IntentKey>(() => ({
    current: () => {
      key.current ??= randomKey()
      return key.current
    },
    renew: () => {
      key.current = null
    },
  }))
  return api
}
