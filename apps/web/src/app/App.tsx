import { Suspense, lazy } from 'react'
import { Navigate, Route, Routes, useLocation } from 'react-router'

import { ErrorState, Loading } from '@/components/state'
import { QueryProvider } from './query'
import { useIsStaff, useSession } from './session'
import { AppShell, TENANT_HOME } from './shell'

/**
 * Ogni rotta è un pezzo a sé, e il guscio non è.
 *
 * Il bundle era **un pezzo solo da 690 kB** (202 compressi): chi apriva il
 * riepilogo scaricava anche le bollette, le segnalazioni e le spese co-living. Su
 * desktop non si nota, e il proprietario è su desktop; **l'inquilino è su rete
 * mobile**, e dalla fetta B esiste davvero.
 *
 * `lazy` sta **fuori** dal componente: dentro, ogni render creerebbe un modulo
 * pigro nuovo, cioè smonterebbe e rimonterebbe la schermata a ogni giro — un
 * difetto che si manifesta come uno sfarfallio e non come un errore.
 *
 * Le pagine di autenticazione restano pigre come le altre: chi non ha una sessione
 * ne carica **una sola**, e chi ce l'ha non ne carica nessuna.
 */
const AuthPage = () => import('@/features/auth/AuthPage')
const LoginPage = lazy(async () => ({ default: (await AuthPage()).LoginPage }))
// `/registrazione` non è più un form (`F6`, ADR-053): è un rimando che dice
// perché, così chi arriva da un segnalibro o da un motore di ricerca legge la
// ragione invece di un `404` muto.
const RegistrationMovedPage = lazy(async () => ({
  default: (await AuthPage()).RegistrationMovedPage,
}))
const PasswordResetPage = lazy(async () => ({ default: (await AuthPage()).PasswordResetPage }))
const PrivacyPage = lazy(async () => ({
  default: (await import('@/features/legal/PrivacyPage')).PrivacyPage,
}))

const InvitationPage = () => import('@/features/invitations/AcceptInvitationPage')
const AcceptInvitationPage = lazy(async () => ({
  default: (await InvitationPage()).AcceptInvitationPage,
}))
const AcceptInvitationSignedOutPage = lazy(async () => ({
  default: (await InvitationPage()).AcceptInvitationSignedOutPage,
}))

const DashboardPage = lazy(async () => ({
  default: (await import('@/features/dashboard/DashboardPage')).DashboardPage,
}))
const AccountPage = lazy(async () => ({
  default: (await import('@/features/account/AccountPage')).AccountPage,
}))
const GuidePage = lazy(async () => ({
  default: (await import('@/features/guide/GuidePage')).GuidePage,
}))
const PropertiesPage = lazy(async () => ({
  default: (await import('@/features/properties/PropertiesPage')).PropertiesPage,
}))
const NewPropertyPage = lazy(async () => ({
  default: (await import('@/features/properties/NewPropertyPage')).NewPropertyPage,
}))
const PropertyDetailPage = lazy(async () => ({
  default: (await import('@/features/properties/PropertyDetailPage')).PropertyDetailPage,
}))
const LeasesPage = lazy(async () => ({
  default: (await import('@/features/leases/LeasesPage')).LeasesPage,
}))
const NewLeasePage = lazy(async () => ({
  default: (await import('@/features/leases/NewLeasePage')).NewLeasePage,
}))
const LeaseDetailPage = lazy(async () => ({
  default: (await import('@/features/leases/LeaseDetailPage')).LeaseDetailPage,
}))
const TicketsPage = lazy(async () => ({
  default: (await import('@/features/tickets/TicketsPage')).TicketsPage,
}))
const TicketDetailPage = lazy(async () => ({
  default: (await import('@/features/tickets/TicketDetailPage')).TicketDetailPage,
}))
const BillsPage = lazy(async () => ({
  default: (await import('@/features/bills/BillsPage')).BillsPage,
}))
const NewBillPage = lazy(async () => ({
  default: (await import('@/features/bills/NewBillPage')).NewBillPage,
}))
const BillDetailPage = lazy(async () => ({
  default: (await import('@/features/bills/BillDetailPage')).BillDetailPage,
}))
const ChargesPage = lazy(async () => ({
  default: (await import('@/features/charges/ChargesPage')).ChargesPage,
}))
const ChargeDetailPage = lazy(async () => ({
  default: (await import('@/features/charges/ChargeDetailPage')).ChargeDetailPage,
}))
const PaymentsPage = lazy(async () => ({
  default: (await import('@/features/payments/PaymentsPage')).PaymentsPage,
}))
const ExpenseGroupsPage = lazy(async () => ({
  default: (await import('@/features/expenses/ExpenseGroupsPage')).ExpenseGroupsPage,
}))
const ExpenseGroupDetailPage = lazy(async () => ({
  default: (await import('@/features/expenses/ExpenseGroupDetailPage')).ExpenseGroupDetailPage,
}))

/**
 * I percorsi sono **in italiano**, come l'interfaccia.
 *
 * Non contraddice la regola per cui ciò che si vede da fuori il repository è in
 * inglese: un URL è prodotto, non superficie di progetto. Il giorno della
 * traduzione questi percorsi restano — cambiare gli indirizzi rompe i segnalibri
 * di chi li ha salvati, e non è ciò che una traduzione deve fare.
 */
export function App() {
  const { session, restoring, restoreError, retryRestore } = useSession()
  const isStaff = useIsStaff()
  const location = useLocation()

  /**
   * Dove si torna dopo essere entrati.
   *
   * Lo `state` lo scrive la redirezione al login qui sotto, e non è fidato — è
   * pur sempre lo stato di una `history` che chiunque può fabbricare — quindi si
   * accetta **solo un percorso interno**: un valore che cominci con `//` sarebbe
   * un'origine esterna scritta come se fosse un percorso, cioè una redirezione
   * aperta regalata da tre righe di comodità.
   */
  const requested = (location.state as { from?: unknown } | null)?.from
  const returnTo =
    typeof requested === 'string' && requested.startsWith('/') && !requested.startsWith('//')
      ? requested
      : '/'

  if (restoring) {
    return <Loading className="min-h-screen justify-center" />
  }

  /**
   * **Prima** del ramo «nessuna sessione», perché non lo è: il rinnovo non ha
   * raggiunto il server — o il server non c'era, e ha risposto il proxy — e il
   * cookie può essere ancora buono. Mandare al form di accesso chi ha solo perso
   * il segnale, o ha ricaricato durante un deploy, sarebbe un logout apparente
   * senza una riga di spiegazione — sul telefono, in galleria, il caso normale.
   * La forma è quella del terzo stato del guscio, senza «Esci»: senza rete un
   * logout non arriva al server, e un bottone che fallisce è peggio di uno che
   * manca.
   */
  if (restoreError) {
    return (
      <main className="flex min-h-screen flex-col items-center justify-center px-4 py-10">
        <ErrorState error={restoreError} onRetry={retryRestore} />
      </main>
    )
  }

  if (!session) {
    return (
      /* **Un confine di `Suspense` senza il suo stato è una pagina bianca** dove
         prima c'era uno scheletro, ed è il difetto che la divisione del bundle
         introduce se la si fa distrattamente. Qui il ripiego è lo stesso
         `Loading` a schermo intero che il ripristino della sessione mostra un
         istante prima: chi arriva vede una cosa sola che continua, non un lampo
         di bianco fra due schermate. */
      <Suspense fallback={<Loading className="min-h-screen justify-center" />}>
        <Routes>
        <Route path="/accesso" element={<LoginPage />} />
        {/* Non un form: da `F6` non c'è una porta aperta, si entra solo da un
            invito (ADR-053). Chi arriva qui — un segnalibro vecchio, un motore
            di ricerca — legge perché e dove andare, invece di un `404` muto. */}
        <Route path="/registrazione" element={<RegistrationMovedPage />} />
        {/* Il recupero vive fuori dalla sessione per definizione: chi lo apre non
            riesce a entrare. Una rotta sola per i due tempi — si chiede il codice,
            si incolla — perché fra i due non c'è niente da ricordare tranne
            l'indirizzo, e mandare altrove chi è già frustrato non compra niente. */}
        <Route path="/recupero-password" element={<PasswordResetPage />} />
        {/* L'informativa si legge **prima** di dare i propri dati, quindi sta
            fra le rotte di chi non è entrato e non solo dentro il guscio. */}
        <Route path="/privacy" element={<PrivacyPage />} />
        {/* Il codice **prima**, il form di registrazione **dopo**, con il codice
            già dentro (ADR-053): non serve più mandare a `/registrazione`, che
            non esiste più come form, e non serve una sessione per accettare —
            questa pagina registra direttamente dall'invito. */}
        <Route path="/invito" element={<AcceptInvitationSignedOutPage />} />
        {/* Chi arriva su una pagina interna senza sessione va al login **e non
            perde dove stava andando**: il percorso resta nello stato, e un
            ritorno al login da una scadenza non fa ricominciare da capo. */}
        <Route
          path="*"
          element={<Navigate to="/accesso" replace state={{ from: location.pathname }} />}
        />
        </Routes>
      </Suspense>
    )
  }

  return (
    <QueryProvider>
      <Routes>
        {/* Il confine sta **dentro** il guscio e non attorno, ed è la
            differenza fra una schermata che arriva e un'applicazione che
            sparisce: fuori, il cambio di rotta smonterebbe navigazione e
            selettore dell'organizzazione a ogni passaggio. Il ripiego è lo
            `SkeletonRows` che le schermate usano già mentre caricano i propri
            dati, quindi il salto di layout è quello di sempre. */}
        <Route element={<AppShell />}>
          {/* La radice è del ruolo, non della schermata. Per lo staff è il
              riepilogo; per chi abita è lo scadenzario, perché la dashboard gli
              risponde `403 org.staff_role_required` — `ensure_staff` *stringe* la
              policy invece di ripeterla, e senza quel controllo leggerebbe i
              numeri dell'organizzazione intera. È anche la domanda che si fa
              davvero: quanto devo. */}
          <Route
            path="/"
            element={isStaff ? <DashboardPage /> : <Navigate to={TENANT_HOME} replace />}
          />
          <Route path="/immobili" element={<PropertiesPage />} />
          <Route path="/immobili/nuovo" element={<NewPropertyPage />} />
          <Route path="/immobili/:id" element={<PropertyDetailPage />} />
          <Route path="/contratti" element={<LeasesPage />} />
          <Route path="/contratti/nuovo" element={<NewLeasePage />} />
          <Route path="/contratti/:id" element={<LeaseDetailPage />} />
          <Route path="/segnalazioni" element={<TicketsPage />} />
          <Route path="/segnalazioni/:id" element={<TicketDetailPage />} />
          <Route path="/bollette" element={<BillsPage />} />
          <Route path="/bollette/nuova" element={<NewBillPage />} />
          <Route path="/bollette/:id" element={<BillDetailPage />} />
          <Route path="/addebiti" element={<ChargesPage />} />
          <Route path="/addebiti/:id" element={<ChargeDetailPage />} />
          <Route path="/pagamenti" element={<PaymentsPage />} />
          {/* Fuori dalla navigazione di chi amministra e raggiungibili lo stesso:
              un proprietario che abiti in una delle proprie case è in un gruppo
              come chiunque altro, e per lui `/spese` risponde. È la stessa scelta
              di `PaymentsPage`. */}
          <Route path="/spese" element={<ExpenseGroupsPage />} />
          <Route path="/spese/:id" element={<ExpenseGroupDetailPage />} />
          {/* Il proprio account: dati, password, uscita. Del ruolo non è: ci si
              arriva dal menù dell'organizzazione, e vale per staff e inquilini. */}
          <Route path="/account" element={<AccountPage />} />
          <Route path="/invito" element={<AcceptInvitationPage />} />
        </Route>
        {/* La guida sta **fuori** dal guscio, come l'informativa: senza
            un'organizzazione il guscio rende la schermata dedicata al posto di
            ogni rotta, e la guida deve aprirsi anche da lì. Il ruolo sceglie la
            versione; senza ruolo si legge quella di chi abita. */}
        <Route
          path="/guida"
          element={
            <Suspense fallback={<Loading className="min-h-screen justify-center" />}>
              <GuidePage />
            </Suspense>
          }
        />
        {/* Chi è già entrato non deve poter tornare al login — o a una rotta di
            registrazione che ormai non esiste come form — da un segnalibro:
            vedrebbe un form che non serve a niente. **Torna dove stava
            andando**, che è la promessa che lo `state` qui sopra scrive: chi
            arriva su `/invito` senza sessione, registra e accetta insieme
            (ADR-053), e non passa più da qui per farlo. */}
        <Route path="/accesso" element={<Navigate to={returnTo} replace />} />
        <Route path="/registrazione" element={<Navigate to={returnTo} replace />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </QueryProvider>
  )
}
