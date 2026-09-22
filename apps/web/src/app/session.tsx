import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
  type ReactNode,
} from 'react'

import { ApiClient } from '@/api/client'
import { ApiError } from '@/api/problem'
import { SessionStore, type OrgRole, type Session } from '@/api/session'

interface SessionContextValue {
  session: Session | null
  store: SessionStore
  client: ApiClient
  /**
   * Rende attiva un'organizzazione e la **ricorda per questo browser**.
   *
   * Il ruolo va passato insieme all'identificatore perché è ciò che decide che
   * cosa il guscio offre: la stessa persona è proprietario della propria
   * organizzazione e inquilino in quella di chi le affitta casa.
   */
  selectOrganization: (organizationId: string, role: OrgRole) => void
  /**
   * Nessuna organizzazione attiva, e **dimentica anche il ricordo di questo
   * browser**: senza, un ricaricamento riproverebbe a selezionare
   * un'organizzazione che non c'è più (ADR-053, guscio a zero organizzazioni).
   */
  clearOrganization: () => void
  /**
   * L'organizzazione attiva viene **dal ricordo di questo browser**, e l'elenco
   * non l'ha ancora confermata né corretta.
   *
   * Vero da quando l'effetto applica il ricordo a quando il guscio, con l'elenco
   * davanti, passa da `selectOrganization` o `clearOrganization`; falso quando
   * l'organizzazione è quella del token o è stata scelta a mano. È stato del
   * provider e non dello store, perché è il provider a sapere che cosa sia il
   * `localStorage` (`R49`): senza, un secondo browser con in memoria
   * un'organizzazione lasciata altrove partiva con quell'`X-Org-Id`, e la prima
   * schermata riceveva un `403` prima che l'elenco correggesse.
   */
  organizationRemembered: boolean
  /** Vero finché il tentativo di ripristino dal cookie è in corso. */
  restoring: boolean
  /**
   * Il ripristino è fallito **per rete**, non per un rifiuto: il cookie può
   * essere ancora buono e la sessione non si è buttata via. È uno stato a sé,
   * distinto da «nessuna sessione», perché la via d'uscita è diversa: riprovare,
   * non entrare di nuovo.
   */
  restoreError: ApiError | null
  /** Rilancia il ripristino, con il suo caricamento davanti. */
  retryRestore: () => void
}

const SessionContext = createContext<SessionContextValue | null>(null)

/**
 * Che cosa il client ricorda fra un caricamento e l'altro, e che cosa no.
 *
 * **Non il token**: l'access token vive in memoria e il refresh in un cookie che
 * JavaScript non legge.
 *
 * Resta **una cosa sola**, e la distinzione è il punto di questo passo: *quale*
 * organizzazione era attiva. È una preferenza di questo browser — non un elenco,
 * che ora lo dà `GET /organizations` — e ricominciare ogni volta da quella del
 * token la renderebbe un default implicito: esattamente la condizione in cui un
 * elenco risponde `200` con zero elementi e nessun errore.
 *
 * Il ruolo si memorizza insieme perché serve **prima** che la rete risponda: il
 * guscio decide su di esso che cosa mostrare, e aspettare una richiesta per
 * saperlo farebbe lampeggiare la navigazione di un ruolo al ricaricamento. Non è
 * autorevole e non deve esserlo — a negare sono l'API e la policy dietro — e
 * l'elenco vero lo corregge appena arriva.
 *
 * Sono dati non sensibili — un UUID e un ruolo — e chi legge il `localStorage` di
 * quel browser è già dentro la sessione.
 */
const STORAGE_KEY = 'propflow.active-org.v1'

interface StoredPreference {
  active: string | null
  role: OrgRole | null
}

function readStored(userId: string): StoredPreference {
  try {
    const raw = localStorage.getItem(`${STORAGE_KEY}:${userId}`)
    if (!raw) return { active: null, role: null }
    const parsed = JSON.parse(raw) as Partial<StoredPreference>
    return { active: parsed.active ?? null, role: parsed.role ?? null }
  } catch {
    // Un `localStorage` non disponibile — navigazione privata, impostazioni del
    // browser — non deve impedire di usare l'applicazione: si perde la memoria
    // dell'organizzazione, non la sessione.
    return { active: null, role: null }
  }
}

function writeStored(userId: string, preference: StoredPreference): void {
  try {
    localStorage.setItem(`${STORAGE_KEY}:${userId}`, JSON.stringify(preference))
  } catch {
    // Vedi sopra.
  }
}

function clearStored(userId: string): void {
  try {
    localStorage.removeItem(`${STORAGE_KEY}:${userId}`)
  } catch {
    // Vedi sopra: un `localStorage` non disponibile non è un errore qui.
  }
}

/**
 * L'organizzazione applicata dal ricordo, finché nessuno la sceglie o la
 * corregge: **un sistema esterno accanto allo store**, e non uno stato di React.
 *
 * L'effetto che applica il ricordo scrive lo store e questo valore nello stesso
 * giro, e React li rende insieme: è ciò che `R49` richiede, perché un render in
 * cui la sessione dice già l'organizzazione ricordata e il guscio non lo sa
 * ancora è esattamente il giro in cui una schermata parte con l'`X-Org-Id`
 * sbagliato. Le due forme più ovvie non vanno: un `setState` dentro quell'effetto
 * è ciò che il lint vieta (`set-state-in-effect`), un ref letto nel render anche
 * (`refs`). Un valore letto con `useSyncExternalStore`, come la sessione stessa,
 * è la forma che dice il vero: l'effetto sincronizza React con due sistemi
 * esterni, non con il proprio stato.
 */
class RememberedOrganization {
  private value: string | null = null
  private readonly listeners = new Set<() => void>()

  readonly snapshot = (): string | null => this.value

  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  set(value: string | null): void {
    if (value === this.value) return
    this.value = value
    for (const listener of this.listeners) listener()
  }
}

export function SessionProvider({ children }: { children: ReactNode }) {
  const [restoring, setRestoring] = useState(true)
  const [restoreError, setRestoreError] = useState<ApiError | null>(null)

  const store = useMemo(() => new SessionStore(), [])
  const client = useMemo(() => new ApiClient({ session: store }), [store])

  const session = useSyncExternalStore(store.subscribe, store.snapshot, store.snapshot)

  /**
   * Al caricamento si prova a **rinnovare in silenzio**.
   *
   * È il modo in cui una sessione sopravvive a un ricaricamento: l'access token è
   * morto con la pagina, il cookie no. Un rifiuto qui non è un errore da mostrare
   * — vuol dire soltanto che non c'era una sessione — e va distinto dal
   * caricamento, altrimenti chi arriva sulla pagina di login vede lampeggiare un
   * errore che non lo riguarda.
   *
   * **La rete caduta non è un rifiuto, e nemmeno un server che non c'è.** Un
   * `TypeError` di `fetch` in galleria — il telefono non raggiunge nemmeno il
   * proxy — diventa `restoreError`, non «nessuna sessione», che manderebbe al
   * form di accesso chi ha un cookie buono e nessun modo di capire perché. E lo
   * stesso vale per un **`5xx`**: con il proxy acceso e l'API spenta — un deploy,
   * un riavvio — `fetch` non lancia, il proxy risponde `502`, e senza questo
   * ramo il sintomo sarebbe identico. È il caso che `R51` aveva visto davvero,
   * «con l'API spenta», e che «offline» da solo non copre. Un `4xx` resta ciò
   * che era: il `401` è la risposta «nessuna sessione», e un form è la via.
   *
   * `restoring` torna a `false` **anche sul ramo dell'errore**, e la riprova lo
   * rimette a `true` prima di rilanciare: senza il primo l'errore non comparirebbe
   * mai, senza il secondo la riprova non avrebbe un caricamento da mostrare e lo
   * schermo salterebbe dall'errore al contenuto.
   *
   * Il contatore serve a ignorare un tentativo superato: lo smontaggio dello
   * `StrictMode` in sviluppo, o una riprova lanciata mentre la precedente è
   * ancora in volo — che con il refresh a singolo volo è la stessa promessa, ma
   * qui non lo si presuppone.
   */
  const attempt = useRef(0)
  const launchRestore = useCallback(() => {
    const current = ++attempt.current
    void store
      .refresh()
      .then(
        () => undefined,
        (error: unknown) => {
          if (current !== attempt.current) return
          if (error instanceof ApiError && (error.isOffline || error.status >= 500)) {
            setRestoreError(error)
          }
        },
      )
      .finally(() => {
        if (current === attempt.current) setRestoring(false)
      })
  }, [store])

  // Al montaggio lo stato è già «in corso»: il lancio non lo tocca, e l'effetto
  // sincronizza solo con lo store, che è il sistema esterno.
  useEffect(() => {
    launchRestore()
    return () => {
      // Un tentativo superato non scrive più: vedi sopra.
      attempt.current += 1
    }
  }, [launchRestore])

  // La riprova invece **parte da un errore**, e deve rimettere davanti il
  // caricamento prima di rilanciare.
  const retryRestore = useCallback(() => {
    setRestoring(true)
    setRestoreError(null)
    launchRestore()
  }, [launchRestore])

  const userId = session?.userId ?? null
  const sessionOrg = session?.organizationId ?? null

  const remembered = useMemo(() => new RememberedOrganization(), [])
  const rememberedOrg = useSyncExternalStore(
    remembered.subscribe,
    remembered.snapshot,
    remembered.snapshot,
  )

  /**
   * Ripristina l'organizzazione scelta prima del ricaricamento.
   *
   * Questo effetto **non tocca lo stato di React**: parla con tre sistemi esterni
   * — lo store della sessione, il `localStorage` e `RememberedOrganization`. È il
   * caso per cui un effetto esiste.
   *
   * Il ripristino avviene **prima** e non dopo `GET /organizations`: fra il
   * caricamento e la risposta di quella query passano richieste — la prima
   * schermata le fa subito — e mandarle con l'organizzazione del token invece che
   * con quella scelta darebbe elenchi vuoti per un istante. Se poi l'elenco vero
   * smentisce il ricordo, a correggerlo è il guscio.
   */
  useEffect(() => {
    if (!userId) return
    const stored = readStored(userId)
    if (stored.active && stored.role && stored.active !== sessionOrg) {
      remembered.set(stored.active)
      store.selectOrganization(stored.active, stored.role)
    }
  }, [userId, sessionOrg, store, remembered])

  // Derivato e non memorizzato: se la sessione cade o cambia organizzazione per
  // un'altra via, il ricordo non conta più senza che nessuno debba azzerarlo.
  const organizationRemembered = sessionOrg !== null && rememberedOrg === sessionOrg

  const value = useMemo<SessionContextValue>(
    () => ({
      session,
      store,
      client,
      organizationRemembered,
      restoring,
      restoreError,
      retryRestore,
      selectOrganization: (organizationId, role) => {
        remembered.set(null)
        if (userId) writeStored(userId, { active: organizationId, role })
        store.selectOrganization(organizationId, role)
      },
      clearOrganization: () => {
        remembered.set(null)
        if (userId) clearStored(userId)
        store.clearOrganization()
      },
    }),
    [
      session,
      store,
      client,
      remembered,
      organizationRemembered,
      restoring,
      restoreError,
      retryRestore,
      userId,
    ],
  )

  return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>
}

export function useSession(): SessionContextValue {
  const value = useContext(SessionContext)
  if (!value) throw new Error('useSession fuori da SessionProvider')
  return value
}

/**
 * Il client dell'API.
 *
 * **È l'unico modo di raggiungere l'API da un componente**, ed è ciò che rende
 * vero il vincolo «nessun `fetch` fuori dal livello dei dati»: un componente che
 * volesse chiamare l'API da sé dovrebbe scrivere `fetch` a mano, cioè fare una
 * cosa visibile in revisione invece di una che si mimetizza.
 */
export function useApi(): ApiClient {
  return useSession().client
}

/**
 * Se chi guarda **amministra**, o se abita e basta.
 *
 * Il ramo sul ruolo sta qui e in nessun altro posto: scritto schermata per
 * schermata finirebbe scritto dodici volte, e la tredicesima verrebbe dimenticata
 * — che è precisamente come si costruisce un bottone che risponde sempre `403`.
 *
 * **Non è un confine di sicurezza, e non va scambiato per tale.** A negare sono
 * l'API e la policy di riga dietro; qui si evita soltanto di *offrire* ciò che
 * verrebbe rifiutato. La differenza si perde facilmente, e per questo è scritta.
 */
export function useIsStaff(): boolean {
  const role = useSession().session?.role
  return role === 'owner' || role === 'manager'
}

export type { OrgRole }
