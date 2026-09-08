import { Navigate, Route, Routes, useLocation } from 'react-router'

import { Loading } from '@/components/state'
import { LoginPage, RegisterPage } from '@/features/auth/AuthPage'
import { BillDetailPage } from '@/features/bills/BillDetailPage'
import { BillsPage } from '@/features/bills/BillsPage'
import { NewBillPage } from '@/features/bills/NewBillPage'
import { ChargeDetailPage } from '@/features/charges/ChargeDetailPage'
import { ChargesPage } from '@/features/charges/ChargesPage'
import { DashboardPage } from '@/features/dashboard/DashboardPage'
import {
  AcceptInvitationPage,
  AcceptInvitationSignedOutPage,
} from '@/features/invitations/AcceptInvitationPage'
import { ExpenseGroupDetailPage } from '@/features/expenses/ExpenseGroupDetailPage'
import { ExpenseGroupsPage } from '@/features/expenses/ExpenseGroupsPage'
import { LeaseDetailPage } from '@/features/leases/LeaseDetailPage'
import { LeasesPage } from '@/features/leases/LeasesPage'
import { NewLeasePage } from '@/features/leases/NewLeasePage'
import { PaymentsPage } from '@/features/payments/PaymentsPage'
import { TicketDetailPage } from '@/features/tickets/TicketDetailPage'
import { TicketsPage } from '@/features/tickets/TicketsPage'
import { NewPropertyPage } from '@/features/properties/NewPropertyPage'
import { PropertiesPage } from '@/features/properties/PropertiesPage'
import { PropertyDetailPage } from '@/features/properties/PropertyDetailPage'
import { QueryProvider } from './query'
import { useIsStaff, useSession } from './session'
import { AppShell, TENANT_HOME } from './shell'

/**
 * I percorsi sono **in italiano**, come l'interfaccia.
 *
 * Non contraddice la regola per cui ciò che si vede da fuori il repository è in
 * inglese: un URL è prodotto, non superficie di progetto. Il giorno della
 * traduzione questi percorsi restano — cambiare gli indirizzi rompe i segnalibri
 * di chi li ha salvati, e non è ciò che una traduzione deve fare.
 */
export function App() {
  const { session, restoring } = useSession()
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

  if (!session) {
    return (
      <Routes>
        <Route path="/accesso" element={<LoginPage />} />
        <Route path="/registrazione" element={<RegisterPage />} />
        {/* Accettare un invito richiede una sessione — la rotta dell'API è
            autenticata — quindi chi arriva qui con un codice in mano e senza
            account non va al login in silenzio: gli si dice che cosa fare
            prima. Mandarlo al login gli farebbe leggere «entra» senza sapere
            che deve prima registrarsi, e il codice giusto darebbe `401`. */}
        <Route path="/invito" element={<AcceptInvitationSignedOutPage />} />
        {/* Chi arriva su una pagina interna senza sessione va al login **e non
            perde dove stava andando**: il percorso resta nello stato, e un
            ritorno al login da una scadenza non fa ricominciare da capo. */}
        <Route
          path="*"
          element={<Navigate to="/accesso" replace state={{ from: location.pathname }} />}
        />
      </Routes>
    )
  }

  return (
    <QueryProvider>
      <Routes>
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
          <Route path="/invito" element={<AcceptInvitationPage />} />
        </Route>
        {/* Chi è già entrato non deve poter tornare al login da un segnalibro:
            vedrebbe un form che non serve a niente. **Torna dove stava
            andando**, che è la promessa che lo `state` qui sopra scrive e che
            fino a questo passo nessuno manteneva: chi si registra per accettare
            un invito ripassa da `/invito` invece di atterrare sul riepilogo
            vuoto della propria organizzazione. */}
        <Route path="/accesso" element={<Navigate to={returnTo} replace />} />
        <Route path="/registrazione" element={<Navigate to={returnTo} replace />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </QueryProvider>
  )
}
