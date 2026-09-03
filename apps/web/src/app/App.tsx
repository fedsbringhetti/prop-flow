import { Navigate, Route, Routes } from 'react-router'

import { Loading } from '@/components/state'
import { LoginPage, RegisterPage } from '@/features/auth/AuthPage'
import { useSession } from './session'

/**
 * I percorsi sono **in italiano**, come l'interfaccia.
 *
 * Non contraddice la regola per cui ciò che si vede da fuori il repository è in
 * inglese: un URL è prodotto, non superficie di progetto. Il giorno della
 * traduzione questi percorsi restano — cambiare gli indirizzi rompe i segnalibri
 * di chi li ha salvati, e non è ciò che una traduzione deve fare.
 */
export function App() {
  const { restoring } = useSession()

  // Finché il ripristino dal cookie è in corso non si decide niente: mostrare il
  // login e poi toglierlo mezzo secondo dopo è peggio di un'attesa dichiarata.
  if (restoring) {
    return <Loading className="min-h-screen justify-center" />
  }

  return (
    <Routes>
      <Route path="/accesso" element={<LoginPage />} />
      <Route path="/registrazione" element={<RegisterPage />} />
      <Route path="*" element={<Navigate to="/accesso" replace />} />
    </Routes>
  )
}
