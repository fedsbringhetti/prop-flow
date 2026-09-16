import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
  useSyncExternalStore,
  type ReactNode,
} from 'react'

import { ApiClient } from '@/api/client'
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
  /** Vero finché il tentativo di ripristino dal cookie è in corso. */
  restoring: boolean
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

export function SessionProvider({ children }: { children: ReactNode }) {
  const [restoring, setRestoring] = useState(true)

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
   */
  useEffect(() => {
    let cancelled = false
    void store
      .refresh()
      .catch(() => undefined)
      .finally(() => {
        if (!cancelled) setRestoring(false)
      })
    return () => {
      cancelled = true
    }
  }, [store])

  const userId = session?.userId ?? null
  const sessionOrg = session?.organizationId ?? null

  /**
   * Ripristina l'organizzazione scelta prima del ricaricamento.
   *
   * Questo effetto **non tocca lo stato di React**: parla con due sistemi esterni,
   * lo store della sessione e il `localStorage`. È il caso per cui un effetto
   * esiste.
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
      store.selectOrganization(stored.active, stored.role)
    }
  }, [userId, sessionOrg, store])

  const value = useMemo<SessionContextValue>(
    () => ({
      session,
      store,
      client,
      restoring,
      selectOrganization: (organizationId, role) => {
        if (userId) writeStored(userId, { active: organizationId, role })
        store.selectOrganization(organizationId, role)
      },
      clearOrganization: () => {
        if (userId) clearStored(userId)
        store.clearOrganization()
      },
    }),
    [session, store, client, restoring, userId],
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
