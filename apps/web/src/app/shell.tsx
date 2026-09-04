import * as DropdownMenu from '@radix-ui/react-dropdown-menu'
import {
  Building2,
  ChevronDown,
  FileText,
  LayoutDashboard,
  LogOut,
  Menu,
  Banknote,
  Receipt,
  Wallet,
  X,
} from 'lucide-react'
import { useState } from 'react'
import { NavLink, Outlet, useLocation } from 'react-router'

import { Button } from '@/components/ui/button'
import { Loading } from '@/components/state'
import { t } from '@/i18n'
import { cn } from '@/lib/utils'
import { useSession } from './session'

const navigation = [
  { to: '/', label: t.nav.dashboard, icon: LayoutDashboard, end: true },
  { to: '/immobili', label: t.nav.properties, icon: Building2, end: false },
  { to: '/contratti', label: t.nav.leases, icon: FileText, end: false },
  { to: '/bollette', label: t.nav.bills, icon: Receipt, end: false },
  { to: '/addebiti', label: t.nav.charges, icon: Wallet, end: false },
  { to: '/pagamenti', label: t.nav.payments, icon: Banknote, end: false },
] as const

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
  const [menuOpen, setMenuOpen] = useState(false)
  const location = useLocation()

  if (restoring) return <Loading className="min-h-screen justify-center" />
  if (!session) return null

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
          <Outlet />
        </main>
      </div>
    </div>
  )
}

/**
 * L'organizzazione attiva, e da qui si cambia.
 *
 * **Si mostra sempre, anche quando ce n'è una sola.** Non è ridondanza: senza
 * `X-Org-Id` corretto gli elenchi rispondono `200` con zero elementi, cioè una
 * schermata vuota senza errore. Avere l'organizzazione scritta in alto è ciò che
 * rende quel caso diagnosticabile in due secondi invece che in mezz'ora.
 *
 * **L'elenco è quello che questo client ha conosciuto**, non uno letto dall'API:
 * un endpoint che elenchi le organizzazioni di un utente non esiste. Per il
 * proprietario, che ne ha una, è indistinguibile; per l'inquilino della fetta 2
 * non lo sarà, ed è registrato fra i debiti.
 */
function OrganizationMenu() {
  const { session, memberships, rememberMembership, store } = useSession()
  if (!session) return null

  const active = session.organizationId
  const role = session.role
  const label = role ? t.org.role[role] : t.org.missing

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
          </span>
          <ChevronDown className="size-4 shrink-0 text-ink-muted" aria-hidden />
        </button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="start"
          sideOffset={4}
          className="z-50 min-w-56 rounded-lg border border-border-subtle bg-surface p-1 shadow-lg"
        >
          <DropdownMenu.Label className="px-2 py-1.5 text-xs text-ink-muted">
            {t.org.switch}
          </DropdownMenu.Label>
          {memberships.map((membership) => (
            <DropdownMenu.Item
              key={membership.organizationId}
              onSelect={() => rememberMembership(membership)}
              className={cn(
                'cursor-pointer rounded-md px-2 py-1.5 text-sm outline-none select-none',
                'data-highlighted:bg-surface-muted',
                membership.organizationId === active && 'font-medium text-accent',
              )}
            >
              <span className="block">{t.org.role[membership.role]}</span>
              <span className="block font-numeric text-xs text-ink-muted">
                {membership.organizationId}
              </span>
            </DropdownMenu.Item>
          ))}
          <DropdownMenu.Separator className="my-1 h-px bg-border-subtle" />
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
