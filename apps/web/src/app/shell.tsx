import * as DropdownMenu from '@radix-ui/react-dropdown-menu'
import { useQuery } from '@tanstack/react-query'
import {
  Building2,
  ChevronDown,
  FileText,
  LayoutDashboard,
  LifeBuoy,
  LogOut,
  Menu,
  TicketCheck,
  Banknote,
  Receipt,
  Users,
  Wallet,
  X,
} from 'lucide-react'
import { Suspense, useEffect, useState } from 'react'
import { Link, NavLink, Outlet, useLocation } from 'react-router'

import { getOrganizations } from '@/api/endpoints'
import type { Organization } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Loading, SkeletonRows } from '@/components/state'
import { t } from '@/i18n'
import { cn } from '@/lib/utils'
import { keys } from './query'
import { useApi, useIsStaff, useSession } from './session'

/**
 * La navigazione di chi **amministra**.
 *
 * Il riepilogo per primo perché è la sua pagina iniziale, e i moduli poi
 * nell'ordine in cui il lavoro li attraversa: il patrimonio, i contratti, i
 * guasti, le bollette, gli addebiti che ne derivano, il denaro che entra.
 */
const staffNavigation = [
  { to: '/', label: t.nav.dashboard, icon: LayoutDashboard, end: true },
  { to: '/immobili', label: t.nav.properties, icon: Building2, end: false },
  { to: '/contratti', label: t.nav.leases, icon: FileText, end: false },
  { to: '/segnalazioni', label: t.nav.tickets, icon: LifeBuoy, end: false },
  { to: '/bollette', label: t.nav.bills, icon: Receipt, end: false },
  { to: '/addebiti', label: t.nav.charges, icon: Wallet, end: false },
  { to: '/pagamenti', label: t.nav.payments, icon: Banknote, end: false },
] as const

/**
 * La navigazione di chi **abita**, e ognuna delle cinque voci è una scelta.
 *
 * **Il riepilogo esce** perché gli risponderebbe `403 org.staff_role_required`:
 * `GetOwnerDashboard` usa `ensure_staff`, che *stringe* la Row Level Security
 * invece di ripeterla — la vista passa da `organizations_select`, che è
 * `is_org_member`, quindi senza quel controllo un inquilino leggerebbe i numeri
 * dell'intera organizzazione. Una voce che porta a un errore è peggio di una voce
 * che manca.
 *
 * **Gli immobili escono, e non per sicurezza**: `ListProperties` è `ensure_member`
 * e gliene mostrerebbe uno. Una voce di navigazione che porta a un elenco di un
 * elemento è rumore, e la scheda dell'immobile resta raggiungibile dal contratto —
 * che è anche il modo in cui ci si arriva pensando.
 *
 * **Il contratto è al singolare** perché per chi abita è uno.
 *
 * **Le bollette restano**, ed è la voce su cui si esita: sono lette e non agite.
 * Ma sono la ragione di metà dei suoi addebiti, e un riaddebito che non si può
 * risalire a un documento è una cifra da accettare sulla fiducia.
 *
 * **Le spese condivise sono sue e di nessun altro**: sono il denaro che i
 * coinquilini si devono **fra loro**, e il proprietario ne è estraneo per
 * costruzione. L'API gliela concederebbe — quasi tutto quel modulo è
 * `ensure_member`, e lui è staff dell'organizzazione — ma una voce che per lui è
 * sempre vuota è il rumore che ADR-042 ha appena tolto dalla sua navigazione con
 * *Immobili*. Le schermate restano
 * progettate e raggiungibili per URL: un proprietario che abiti in una delle
 * proprie case è in un gruppo come chiunque altro.
 *
 * Gli addebiti stanno per primi perché sono la sua pagina iniziale: la domanda che
 * si fa davvero è *quanto devo*. Le spese subito dopo, perché è la stessa domanda
 * rivolta a chi abita con lui.
 */
const tenantNavigation = [
  { to: '/addebiti', label: t.nav.charges, icon: Wallet, end: false },
  { to: '/spese', label: t.nav.expenses, icon: Users, end: false },
  { to: '/segnalazioni', label: t.nav.tickets, icon: LifeBuoy, end: false },
  { to: '/contratti', label: t.nav.lease, icon: FileText, end: false },
  { to: '/bollette', label: t.nav.bills, icon: Receipt, end: false },
] as const

/**
 * Dove atterra chi non amministra.
 *
 * Vive qui accanto alla navigazione e non dentro il router perché è la stessa
 * decisione — la prima voce del suo elenco è la sua pagina iniziale — e tenerle in
 * due file diversi è il modo in cui finiscono per non coincidere più.
 *
 * **Non è una rotta nuova**: è lo scadenzario, che `ensure_member` gli concede già
 * ristretto dalle policy ai suoi. Una dashboard dell'inquilino sarebbe una
 * proiezione nuova, quindi un endpoint nuovo, quindi backend dentro una fetta di
 * client — ed è rinviata con la sua ragione.
 */
export const TENANT_HOME = '/addebiti'

/**
 * Il guscio: navigazione, organizzazione attiva, uscita.
 *
 * # La navigazione è un `<nav>` con `aria-label`
 *
 * Una pagina con più regioni di navigazione ne ha una sola che conta, e chi usa un
 * lettore di schermo salta fra le regioni per nome. Senza l'etichetta si chiamano
 * tutte «navigazione».
 *
 * # Il salto al contenuto è la prima cosa nel DOM
 *
 * Senza, ogni schermata comincia con cinque tabulazioni identiche prima di
 * arrivare a ciò che si è venuti a fare.
 */
export function AppShell() {
  const { session, restoring } = useSession()
  const isStaff = useIsStaff()
  const [menuOpen, setMenuOpen] = useState(false)
  const location = useLocation()

  if (restoring) return <Loading className="min-h-screen justify-center" />
  if (!session) return null

  // Il ramo sul ruolo, e questo è **uno** dei tre posti in cui compare: qui, la
  // rotta iniziale in `App`, e i bottoni di scrittura delle schermate. Non un
  // `if` per componente — quello finirebbe scritto dodici volte, e la
  // tredicesima verrebbe dimenticata.
  const navigation = isStaff ? staffNavigation : tenantNavigation

  return (
    <div className="min-h-screen">
      <a href="#contenuto" className="skip-link">
        {t.app.skipToContent}
      </a>

      <div className="mx-auto flex w-full max-w-[100rem] flex-col lg:flex-row">
        {/* Barra superiore su schermo stretto: il menù è a scomparsa, e il
            titolo resta sempre visibile perché è anche il ritorno al riepilogo. */}
        <div className="flex items-center justify-between gap-2 border-b border-border-subtle bg-surface px-4 py-3 lg:hidden">
          <span className="text-sm font-semibold">{t.app.name}</span>
          <div className="flex items-center gap-2">
            <OrganizationMenu />
            <Button
              size="icon"
              variant="ghost"
              aria-expanded={menuOpen}
              aria-controls="navigazione-principale"
              aria-label={t.nav.main}
              onClick={() => setMenuOpen((open) => !open)}
            >
              {menuOpen ? <X /> : <Menu />}
            </Button>
          </div>
        </div>

        <nav
          id="navigazione-principale"
          aria-label={t.nav.main}
          className={cn(
            'shrink-0 border-b border-border-subtle bg-surface lg:w-60 lg:border-r lg:border-b-0',
            menuOpen ? 'block' : 'hidden lg:block',
          )}
        >
          <div className="hidden px-4 py-4 lg:block">
            <span className="text-sm font-semibold">{t.app.name}</span>
          </div>
          <ul className="flex flex-col gap-0.5 p-2">
            {navigation.map((item) => (
              <li key={item.to}>
                <NavLink
                  to={item.to}
                  end={item.end}
                  onClick={() => setMenuOpen(false)}
                  className={({ isActive }) =>
                    cn(
                      'flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm transition-colors',
                      isActive
                        ? 'bg-accent-soft font-medium text-accent'
                        : 'text-ink-muted hover:bg-surface-muted hover:text-ink',
                    )
                  }
                >
                  {({ isActive }) => (
                    <>
                      <item.icon className="size-4 shrink-0" aria-hidden />
                      <span>{item.label}</span>
                      {/* Lo stato attivo non è **solo** un colore: chi non
                          distingue i colori deve poterlo sapere lo stesso. */}
                      {isActive ? <span className="sr-only">(pagina corrente)</span> : null}
                    </>
                  )}
                </NavLink>
              </li>
            ))}
          </ul>
          <div className="hidden border-t border-border-subtle p-2 lg:block">
            <OrganizationMenu />
          </div>
        </nav>

        <main
          id="contenuto"
          // `tabIndex={-1}` rende il contenuto raggiungibile dal salto: senza,
          // il link sposta la vista ma non il focus, e la tabulazione successiva
          // riparte dalla navigazione.
          tabIndex={-1}
          key={location.pathname}
          className="min-w-0 flex-1 px-4 py-6 sm:px-6 lg:px-8"
        >
          {/* Il confine di `Suspense` di ogni rotta, e **con il suo stato**.
              Sta qui dentro `<main>` e non attorno al guscio: il cambio di
              schermata non deve smontare la navigazione né il selettore
              dell'organizzazione, che sono ciò che resta fermo mentre il resto
              cambia. Il ripiego è lo scheletro che le schermate mostrano già
              mentre leggono i propri dati — senza, dove c'era uno scheletro
              comparirebbe una pagina bianca. */}
          <Suspense fallback={<SkeletonRows rows={5} />}>
            <Outlet />
          </Suspense>
        </main>
      </div>
    </div>
  )
}

/**
 * Le organizzazioni di chi guarda, **lette dall'API**.
 *
 * Prima di questo passo l'elenco era ciò che questo browser aveva conosciuto, e
 * il menù mostrava un ruolo e un UUID: per un proprietario, che ne ha una, era
 * indistinguibile da un elenco vero; per un inquilino sarebbero stati due UUID fra
 * cui indovinare. `GET /organizations` è la sola rotta che dia il **nome** a chi
 * non è staff, ed è la ragione per cui esiste.
 */
function useOrganizations() {
  const client = useApi()
  const { session, selectOrganization } = useSession()

  const query = useQuery({
    queryKey: keys.organizations,
    queryFn: () => getOrganizations(client),
    // Le proprie organizzazioni non cambiano mentre si lavora: cambiano quando si
    // accetta un invito, e quello invalida la chiave da sé.
    staleTime: 5 * 60_000,
  })

  const items = query.data?.items
  const activeId = session?.organizationId ?? null
  const activeRole = session?.role ?? null

  /**
   * L'elenco vero **corregge il ricordo**, e non il contrario.
   *
   * Due casi reali, e nessuno dei due è teorico. Il primo: l'organizzazione
   * ricordata non è più fra le proprie — un invito revocato, un'appartenenza
   * chiusa — e da quel momento **ogni** richiesta porta un `X-Org-Id` che l'API
   * rifiuta, cioè un'applicazione intera che risponde `403`. Il secondo: non ce
   * n'è nessuna attiva, e allora gli elenchi rispondono `200` con zero elementi,
   * che è il modo peggiore di sbagliare perché non somiglia a un errore.
   *
   * Il ruolo si riallinea per la stessa ragione: quello ricordato serve a non far
   * lampeggiare la navigazione al ricaricamento, ma è l'API a dire qual è.
   */
  useEffect(() => {
    if (!items || items.length === 0) return
    const active = items.find((organization) => organization.id === activeId)
    if (!active) {
      const fallback = items[0]
      if (fallback) selectOrganization(fallback.id, fallback.role)
      return
    }
    if (active.role !== activeRole) selectOrganization(active.id, active.role)
  }, [items, activeId, activeRole, selectOrganization])

  const active: Organization | undefined = items?.find(
    (organization) => organization.id === activeId,
  )

  return { query, items, active }
}

/**
 * L'organizzazione attiva, e da qui si cambia.
 *
 * **Si mostra sempre, anche quando ce n'è una sola.** Non è ridondanza: senza
 * `X-Org-Id` corretto gli elenchi rispondono `200` con zero elementi, cioè una
 * schermata vuota senza errore. Avere l'organizzazione scritta in alto è ciò che
 * rende quel caso diagnosticabile in due secondi invece che in mezz'ora.
 *
 * # I tre stati valgono anche per un menù
 *
 * Un selettore che sta caricando **non deve mostrare un elenco vuoto**: un elenco
 * vuoto è indistinguibile da «non appartieni a niente», e chi lo legge va a
 * cercare un problema che non esiste. Caricamento, vuoto ed errore sono quindi tre
 * risposte diverse anche qui, dove costerebbe poco confonderle.
 */
function OrganizationMenu() {
  const { session, store } = useSession()
  const { query, items, active } = useOrganizations()

  if (!session) return null

  // Il nome quando c'è; altrimenti si dice che si sta cercando, e mai l'UUID —
  // che è precisamente ciò da cui questo passo libera.
  const label = active?.name ?? (query.isPending ? t.state.loadingTitle : t.org.unknownName)
  const role = active?.role ?? session.role

  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <button
          type="button"
          className="flex w-full items-center justify-between gap-2 rounded-lg px-3 py-2 text-left text-sm hover:bg-surface-muted"
        >
          <span className="min-w-0">
            <span className="block text-xs text-ink-muted">{t.org.label}</span>
            <span className="block truncate font-medium">{label}</span>
            <span className="block truncate text-xs text-ink-muted">
              {role ? t.org.role[role] : t.org.missing}
            </span>
          </span>
          <ChevronDown className="size-4 shrink-0 text-ink-muted" aria-hidden />
        </button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="start"
          sideOffset={4}
          className="z-50 max-w-[calc(100vw-2rem)] min-w-56 rounded-lg border border-border-subtle bg-surface p-1 shadow-lg"
        >
          <DropdownMenu.Label className="px-2 py-1.5 text-xs text-ink-muted">
            {t.org.switch}
          </DropdownMenu.Label>

          {query.isPending ? (
            <p className="px-2 py-1.5 text-sm text-ink-muted">{t.org.loading}</p>
          ) : null}
          {query.error ? (
            <p role="alert" className="px-2 py-1.5 text-sm text-danger">
              {t.org.loadFailed}
            </p>
          ) : null}
          {items && items.length === 0 ? (
            <p className="px-2 py-1.5 text-sm text-ink-muted">{t.org.empty}</p>
          ) : null}

          {(items ?? []).map((organization) => (
            <OrganizationItem
              key={organization.id}
              organization={organization}
              current={organization.id === session.organizationId}
            />
          ))}

          <DropdownMenu.Separator className="my-1 h-px bg-border-subtle" />
          {/* **Dove si usa un codice d'invito**, e sta qui per una ragione
              precisa: chi ha appena ricevuto un invito arriva in questo menù da
              sé, perché è il posto in cui si va quando ci si accorge di essere
              nell'organizzazione sbagliata. Una voce nella navigazione
              principale sarebbe un permanente per un gesto che si fa una volta. */}
          <DropdownMenu.Item asChild>
            <Link
              to="/invito"
              className="flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-sm outline-none select-none data-highlighted:bg-surface-muted"
            >
              <TicketCheck className="size-4" aria-hidden />
              {t.invitation.acceptEntry}
            </Link>
          </DropdownMenu.Item>
          <DropdownMenu.Item
            onSelect={() => {
              void store.logout()
            }}
            className="flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-sm outline-none select-none data-highlighted:bg-surface-muted"
          >
            <LogOut className="size-4" aria-hidden />
            {t.auth.logout}
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}

function OrganizationItem({
  organization,
  current,
}: {
  organization: Organization
  current: boolean
}) {
  const { selectOrganization } = useSession()

  return (
    <DropdownMenu.Item
      onSelect={() => selectOrganization(organization.id, organization.role)}
      className={cn(
        'cursor-pointer rounded-md px-2 py-1.5 text-sm outline-none select-none',
        'data-highlighted:bg-surface-muted',
        current && 'font-medium text-accent',
      )}
    >
      <span className="block truncate">{organization.name}</span>
      <span className="block text-xs text-ink-muted">
        {t.org.role[organization.role]}
        {/* Quale sia attiva non è **solo** un colore: chi non distingue i colori
            deve poterlo sapere lo stesso. */}
        {current ? <span className="sr-only"> (attiva)</span> : null}
      </span>
    </DropdownMenu.Item>
  )
}
