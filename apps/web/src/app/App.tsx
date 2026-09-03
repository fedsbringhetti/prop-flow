import { Navigate, Route, Routes, useLocation } from 'react-router'

import { Loading } from '@/components/state'
import { LoginPage, RegisterPage } from '@/features/auth/AuthPage'
import { DashboardPage } from '@/features/dashboard/DashboardPage'
import { NewPropertyPage } from '@/features/properties/NewPropertyPage'
import { PropertiesPage } from '@/features/properties/PropertiesPage'
import { PropertyDetailPage } from '@/features/properties/PropertyDetailPage'
import { QueryProvider } from './query'
import { useSession } from './session'
import { AppShell } from './shell'

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
  const location = useLocation()

  if (restoring) {
    return <Loading className="min-h-screen justify-center" />
  }

  if (!session) {
    return (
      <Routes>
        <Route path="/accesso" element={<LoginPage />} />
        <Route path="/registrazione" element={<RegisterPage />} />
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
          <Route path="/" element={<DashboardPage />} />
          <Route path="/immobili" element={<PropertiesPage />} />
          <Route path="/immobili/nuovo" element={<NewPropertyPage />} />
          <Route path="/immobili/:id" element={<PropertyDetailPage />} />
        </Route>
        {/* Chi è già entrato non deve poter tornare al login da un segnalibro:
            vedrebbe un form che non serve a niente. */}
        <Route path="/accesso" element={<Navigate to="/" replace />} />
        <Route path="/registrazione" element={<Navigate to="/" replace />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </QueryProvider>
  )
}
