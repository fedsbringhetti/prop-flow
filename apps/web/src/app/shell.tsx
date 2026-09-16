import * as DropdownMenu from '@radix-ui/react-dropdown-menu'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Building2,
  ChevronDown,
  FileText,
  LayoutDashboard,
  LifeBuoy,
  LogOut,
  Menu,
  TicketCheck,
  UserCog,
  Banknote,
  Receipt,
  Users,
  Wallet,
  X,
} from 'lucide-react'
import { Suspense, useEffect, useState } from 'react'
import { Link, NavLink, Outlet, useLocation, useNavigate } from 'react-router'

import { acceptInvitation, getAccount, getOrganizations } from '@/api/endpoints'
import type { Items, OrgRole, Organization } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Field, Input } from '@/components/ui/field'
import { ErrorState, Loading, SkeletonRows } from '@/components/state'
import { InvitationCodeError } from '@/features/invitations/errors'
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
  const { session, store, restoring } = useSession()
  const isStaff = useIsStaff()
  const [menuOpen, setMenuOpen] = useState(false)
  const location = useLocation()
  // Chiamato qui e non solo in `OrganizationMenu`: il terzo stato del guscio,
  // sotto, decide **prima** di `<Outlet/>`, quindi la regola dei hook lo vuole
  // ad ogni giro di questo componente e non solo quando serve davvero. Non è
  // una seconda chiamata di rete: `useQuery` condivide la cache sulla stessa
  // chiave con quella che `OrganizationMenu` fa più sotto.
  const { query: organizations, items } = useOrganizations()

  if (restoring) return <Loading className="min-h-screen justify-center" />
  if (!session) return null

  /**
   * Terzo stato del guscio, accanto a «fuori sessione» e «dentro» (`F7`,
   * ADR-053): nessuna organizzazione attiva. Non è un errore e non è il vuoto
   * di un elenco — è quello che si è e che cosa aspettarsi, senza navigazione.
   *
   * **L'ordine conta.** `session.organizationId === null` è vero sia mentre
   * l'elenco sta ancora arrivando, sia quando è arrivato vuoto per davvero, sia
   * quando **non è arrivato affatto** — un errore di rete, un `500` — e i tre
   * casi non si distinguono guardando solo la sessione. Finché non si sa quale
   * dei tre sia, **non si rende `<Outlet/>`**: la dashboard o un'altra
   * schermata di rotta partirebbe senza `X-Org-Id` — la sessione non ne ha uno
   * da dare — e riceverebbe `403 org.not_selected` per un istante prima che
   * l'elenco corregga tutto. È la stessa lezione del poscritto della sezione 6:
   * una richiesta non deve dipendere da uno stato che non è ancora arrivato.
   * Quando invece `session.organizationId` è già valorizzato — il caso comune,
   * dal token o dal `localStorage` — non c'è nulla da aspettare: il guscio
   * rende subito, come faceva prima di questo passo.
   *
   * **Il terzo caso — l'errore — non è un `Loading` che dura per sempre**
   * (regola 23): senza un ramo suo, un `getOrganizations` che fallisse
   * lascerebbe chi guarda davanti a uno spinner senza via d'uscita, perché il
   * guscio che offrirebbe la riprova o l'uscita è proprio quello che questo
   * stato sostituisce.
   */
  if (session.organizationId === null) {
    if (organizations.isError) {
      return (
        <main className="flex min-h-screen flex-col items-center justify-center gap-4 px-4 py-10">
          <ErrorState error={organizations.error} onRetry={organizations.refetch} />
          <Button
            variant="ghost"
            onClick={() => {
              void store.logout()
            }}
          >
            {t.auth.logout}
          </Button>
        </main>
      )
    }
    if (items && items.length === 0) return <NoOrganizationScreen />
    return <Loading className="min-h-screen justify-center" />
  }

  // Il ramo sul ruolo, e questo è **uno** dei tre posti in cui compare: qui, la
  // rotta iniziale in `App`, e i bottoni di scrittura delle schermate. Non un
  // `if` per componente — quello finirebbe scritto dodici volte, e la
  // tredicesima verrebbe dimenticata.
  const navigation = isStaff ? staffNavigation : tenantNavigation

  return (
    <div>
      <a href="#contenuto" className="skip-link">
        {t.app.skipToContent}
      </a>

      {/* **L'altezza sta qui, e non sul `<div>` di fuori.** Era il contrario, e
          il sintomo era una barra laterale che «si vede a metà»: un `min-height`
          su un genitore **non allunga** un figlio di blocco, quindi il
          contenitore flex era alto quanto il suo contenuto e la barra — che è
          `stretch` dentro di esso — finiva dove finiva il contenuto. Su una
          schermata di poche righe si fermava a metà schermo. */}
      <div className="mx-auto flex min-h-screen w-full max-w-[100rem] flex-col lg:flex-row">
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

        {/* Da `lg` in su è una colonna alta quanto la finestra e **ancorata**:
            `h-screen` e non `stretch`, perché su una schermata lunga la barra si
            allungherebbe quanto il documento e il selettore in fondo finirebbe a
            quattromila pixel di distanza. Sotto quella soglia è un menù a
            scomparsa e nulla di tutto questo si applica. */}
        <nav
          id="navigazione-principale"
          aria-label={t.nav.main}
          className={cn(
            'shrink-0 border-b border-border-subtle bg-surface',
            'lg:sticky lg:top-0 lg:flex lg:h-screen lg:w-60 lg:flex-col lg:border-r lg:border-b-0',
            menuOpen ? 'block' : 'hidden',
          )}
        >
          <div className="hidden px-4 py-4 lg:block">
            <span className="text-sm font-semibold">{t.app.name}</span>
          </div>
          {/* `min-h-0` non è ornamentale: senza, un figlio flex non scende sotto
              la propria altezza naturale e `overflow-y-auto` non scorre mai. */}
          <ul className="flex flex-col gap-0.5 p-2 lg:min-h-0 lg:flex-1 lg:overflow-y-auto">
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
          {/* **In fondo, e solo da `lg`.** `mt-auto` vuol dire «in fondo alla
              colonna flex», e sotto quella soglia la colonna non c'è: la
              navigazione è a scomparsa e il selettore sta già nella barra in
              alto, dove un `mt-auto` incondizionato spingerebbe in basso
              qualcosa che lì non deve stare. */}
          <div className="hidden border-t border-border-subtle p-2 lg:mt-auto lg:block">
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
 * Il terzo stato del guscio: **nessuna organizzazione**.
 *
 * Non un errore e non il vuoto di un elenco — è una schermata con un titolo
 * suo, a tutto schermo come `AuthFrame`, e **senza navigazione**: né la barra
 * laterale né `<Outlet/>`. Non c'è ancora niente da amministrare o da
 * abitare, e mostrarli offrirebbe voci che risponderebbero tutte `403`.
 *
 * # Come ci si arriva
 *
 * Nessun utente ci nasce (ADR-053, decisione 1): la registrazione — da
 * proprietario o da inquilino — lega sempre una membership nella stessa
 * transazione. Ci si arriva **uscendo** dall'unica organizzazione che si
 * aveva: da inquilino, da gestore, o da proprietario non ultimo — l'ultimo
 * proprietario non può, `membership.last_owner` lo impedisce finché resta lui
 * a doverla amministrare. Un invito **non** accettato non c'entra: si revoca
 * finché è pendente, e chi non l'ha ancora accettato non è mai entrato.
 *
 * # Il codice si incolla **qui**, non porta a `/invito`
 *
 * `/invito` vive dentro il guscio che questa schermata sostituisce — è una
 * rotta di `<Outlet/>`, che qui non rende — quindi un link lì sarebbe un
 * rimando a una pagina che non apparirebbe mai. Il modo di uscire da questo
 * stato è lo stesso gesto, incollato direttamente qui: la stessa
 * `acceptInvitation` che usa `AcceptInvitationPage`, con lo stesso esito —
 * l'organizzazione appena accettata diventa quella attiva e si atterra sulla
 * pagina del proprio ruolo.
 */
function NoOrganizationScreen() {
  const client = useApi()
  const { store, selectOrganization } = useSession()
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const [token, setToken] = useState('')

  // I propri dati, per dire **chi si è**: senza, la schermata direbbe solo che
  // non si appartiene a niente, e non a chi è successo. `getAccount` risponde
  // senza organizzazione attiva per la stessa ragione di `getOrganizations`.
  const account = useQuery({ queryKey: keys.account, queryFn: () => getAccount(client) })

  const mutation = useMutation({
    mutationFn: () => acceptInvitation(client, token.trim()),
    onSuccess: async (accepted) => {
      const role = accepted.role as OrgRole
      // **In cache prima del giro di rete**, come fa `AccountPage` all'uscita
      // (`OrganizationRow`, che toglie dalla cache prima di rileggerla): senza
      // questa scrittura, `selectOrganization` da sola farebbe scattare
      // l'effetto di riallineamento con l'elenco ancora `[]` — quello letto
      // l'ultima volta, da questa stessa schermata — nella finestra fra
      // l'`await` di `invalidateQueries` qui sotto e la risposta vera:
      // l'effetto vedrebbe `[]` e chiamerebbe `clearOrganization()`, e il ramo
      // `fallback` la riselezionerebbe da capo non appena la rilettura arriva
      // — due scritture di troppo (e una cancellazione) sul ricordo di questo
      // browser per un'accettazione sola. Scrivere qui, **prima** di
      // `selectOrganization` e nello stesso giro sincrono, fa sì che React
      // veda le due modifiche insieme in un solo giro: l'elenco non sembra mai
      // vuoto. Nome e slug veri arrivano dalla rilettura sotto.
      queryClient.setQueryData<Items<Organization>>(keys.organizations, (previous) => {
        const placeholder: Organization = {
          id: accepted.organization_id,
          name: t.org.unknownName,
          slug: '',
          role,
        }
        const items = previous?.items ?? []
        if (items.some((item) => item.id === accepted.organization_id)) return previous
        return { items: [...items, placeholder] }
      })
      selectOrganization(accepted.organization_id, role)
      await queryClient.invalidateQueries({ queryKey: keys.organizations })
      await navigate(role === 'tenant' ? TENANT_HOME : '/', { replace: true })
    },
  })

  return (
    <main className="flex min-h-screen items-center justify-center px-4 py-10">
      <div className="w-full max-w-sm">
        <p className="mb-6 text-sm font-semibold text-accent">{t.app.name}</p>
        <h1 className="text-xl font-semibold tracking-tight text-ink">{t.org.noneTitle}</h1>
        {/* Caricamento e errore restano distinti dal caso felice, come ovunque
            (regola 23): senza, «sto ancora leggendo chi sei» e «non fai parte
            di nessuna organizzazione» direbbero la stessa frase generica, e
            un secondo che passa sembrerebbe un salto di contenuto invece che
            un nome che arriva. L'errore non ha un ripiego suo: questa
            schermata non deve dipendere da `/me` per il suo scopo vero, che è
            lasciare incollare un codice. */}
        <p className="mt-1 text-sm text-ink-muted">
          {account.isPending
            ? t.state.loadingTitle
            : account.data
              ? t.org.noneBody(account.data.full_name)
              : t.org.noneBodyFallback}
        </p>
        <p className="mt-1 mb-6 text-sm text-ink-muted">{t.org.noneHint}</p>
        <form
          noValidate
          className="flex flex-col gap-4"
          onSubmit={(event) => {
            event.preventDefault()
            mutation.mutate()
          }}
        >
          <InvitationCodeError error={mutation.error} />
          <Field label={t.invitation.tokenLabel} hint={t.invitation.acceptCodeHint} required>
            {(props) => (
              <Input
                {...props}
                value={token}
                autoFocus
                autoCapitalize="off"
                autoCorrect="off"
                spellCheck={false}
                onChange={(event) => setToken(event.target.value)}
              />
            )}
          </Field>
          <Button
            type="submit"
            variant="primary"
            size="lg"
            disabled={mutation.isPending || token.trim() === ''}
          >
            {mutation.isPending ? t.common.saving : t.invitation.acceptSubmit}
          </Button>
        </form>
        <Button
          variant="ghost"
          size="lg"
          className="mt-3 w-full"
          onClick={() => {
            void store.logout()
          }}
        >
          {t.auth.logout}
        </Button>
      </div>
    </main>
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
export function useOrganizations() {
  const client = useApi()
  const { session, selectOrganization, clearOrganization } = useSession()

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
   * Tre casi, e nessuno dei tre è teorico. Il primo: l'organizzazione ricordata
   * non è più fra le proprie — un invito revocato, un'appartenenza chiusa — e da
   * quel momento **ogni** richiesta porta un `X-Org-Id` che l'API rifiuta, cioè
   * un'applicazione intera che risponde `403`. Il secondo: non ce n'è nessuna
   * attiva ma ne esistono, e allora gli elenchi rispondono `200` con zero
   * elementi, che è il modo peggiore di sbagliare perché non somiglia a un
   * errore. Il terzo, da ADR-053: **non esiste più nessuna organizzazione**, ed
   * è lo stato che si raggiunge uscendo dall'unica che si aveva — qui non c'è
   * un ripiego a cui riallinearsi, e l'unico stato coerente è nessuna attiva.
   *
   * Il ruolo si riallinea per la stessa ragione: quello ricordato serve a non far
   * lampeggiare la navigazione al ricaricamento, ma è l'API a dire qual è.
   */
  useEffect(() => {
    if (!items) return
    if (items.length === 0) {
      if (activeId !== null) clearOrganization()
      return
    }
    const active = items.find((organization) => organization.id === activeId)
    if (!active) {
      const fallback = items[0]
      if (fallback) selectOrganization(fallback.id, fallback.role)
      return
    }
    if (active.role !== activeRole) selectOrganization(active.id, active.role)
  }, [items, activeId, activeRole, selectOrganization, clearOrganization])

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
          {/* **Il proprio account**, e sta qui e non nella navigazione principale
              per la stessa ragione dell'invito: non è un modulo del prodotto, è una
              cosa che si fa su di sé, e la si cerca dove si cambia organizzazione e
              si esce. Vale per staff e inquilini, quindi non passa dal ramo sul
              ruolo. */}
          <DropdownMenu.Item asChild>
            <Link
              to="/account"
              className="flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-sm outline-none select-none data-highlighted:bg-surface-muted"
            >
              <UserCog className="size-4" aria-hidden />
              {t.nav.account}
            </Link>
          </DropdownMenu.Item>
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
