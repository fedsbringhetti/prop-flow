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
import { SessionStore, type Membership, type OrgRole, type Session } from '@/api/session'

interface SessionContextValue {
  session: Session | null
  store: SessionStore
  client: ApiClient
  /** Le organizzazioni che questo client ha conosciuto per questo utente. */
  memberships: readonly Membership[]
  rememberMembership: (membership: Membership) => void
  /** Vero finché il tentativo di ripristino dal cookie è in corso. */
  restoring: boolean
}

const SessionContext = createContext<SessionContextValue | null>(null)

/**
 * Che cosa il client ricorda fra un caricamento e l'altro, e che cosa no.
 *
 * **Non il token**: l'access token vive in memoria e il refresh in un cookie che
 * JavaScript non legge. Qui restano due cose che token non sono — quale
 * organizzazione era attiva, e quali il client ha conosciuto — perché
 * l'organizzazione attiva è **stato esplicito** e ricominciare ogni volta da
 * quella del token la renderebbe un default implicito: esattamente la condizione
 * in cui un elenco risponde `200` con zero elementi e nessun errore.
 *
 * Sono dati non sensibili — due UUID e un ruolo — e chi legge il `localStorage`
 * di quel browser è già dentro la sessione.
 */
const STORAGE_KEY = 'propflow.memberships.v1'

interface StoredState {
  active: string | null
  known: Membership[]
}

function readStored(userId: string): StoredState {
  try {
    const raw = localStorage.getItem(`${STORAGE_KEY}:${userId}`)
    if (!raw) return { active: null, known: [] }
    const parsed = JSON.parse(raw) as Partial<StoredState>
    return { active: parsed.active ?? null, known: parsed.known ?? [] }
  } catch {
    // Un `localStorage` non disponibile — navigazione privata, impostazioni del
    // browser — non deve impedire di usare l'applicazione: si perde la memoria
    // dell'organizzazione, non la sessione.
    return { active: null, known: [] }
  }
}

function writeStored(userId: string, state: StoredState): void {
  try {
    localStorage.setItem(`${STORAGE_KEY}:${userId}`, JSON.stringify(state))
  } catch {
    // Vedi sopra.
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
  const sessionOrg = session?.organizationId
  const sessionRole = session?.role

  /**
   * Le appartenenze conosciute, ricaricate **quando cambia l'utente**.
   *
   * L'aggiustamento avviene durante il render e non dentro un effetto: un
   * `setState` in un effetto produce un secondo render con lo stato vecchio già
   * disegnato, che qui significherebbe un menù dell'organizzazione che compare
   * vuoto per un fotogramma. È il caso in cui React prescrive di correggere lo
   * stato durante il render, confrontando la chiave da cui dipende.
   */
  const [cache, setCache] = useState<{ userId: string | null; known: Membership[] }>({
    userId: null,
    known: [],
  })
  if (cache.userId !== userId) {
    setCache({ userId, known: userId ? readStored(userId).known : [] })
  }

  const memberships = useMemo<readonly Membership[]>(() => {
    const known = [...cache.known]
    if (sessionOrg && sessionRole && !known.some((m) => m.organizationId === sessionOrg)) {
      known.push({ organizationId: sessionOrg, role: sessionRole })
    }
    return known
  }, [cache, sessionOrg, sessionRole])

  /**
   * Ripristina l'organizzazione scelta prima del ricaricamento.
   *
   * Questo effetto **non tocca lo stato di React**: parla con due sistemi esterni,
   * lo store della sessione e il `localStorage`. È il caso per cui un effetto
   * esiste.
   */
  useEffect(() => {
    if (!userId) return
    const stored = readStored(userId)
    const restored = stored.active
      ? memberships.find((m) => m.organizationId === stored.active)
      : undefined
    if (restored && restored.organizationId !== sessionOrg) {
      store.selectOrganization(restored.organizationId, restored.role)
    }
    writeStored(userId, {
      active: restored?.organizationId ?? sessionOrg ?? null,
      known: [...memberships],
    })
  }, [userId, sessionOrg, memberships, store])

  const value = useMemo<SessionContextValue>(
    () => ({
      session,
      store,
      client,
      memberships,
      restoring,
      rememberMembership: (membership) => {
        setCache((current) => ({
          userId: current.userId,
          known: [
            ...current.known.filter((m) => m.organizationId !== membership.organizationId),
            membership,
          ],
        }))
        if (userId) {
          const known = readStored(userId).known.filter(
            (m) => m.organizationId !== membership.organizationId,
          )
          writeStored(userId, { active: membership.organizationId, known: [...known, membership] })
        }
        store.selectOrganization(membership.organizationId, membership.role)
      },
    }),
    [session, store, client, memberships, restoring, userId],
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

export type { OrgRole }
