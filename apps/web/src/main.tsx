import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter } from 'react-router'

import { App } from './app/App'
import { SessionProvider } from './app/session'
import './index.css'

const root = document.getElementById('root')
if (!root) {
  // Nessun `!` e nessun `expect`: un errore parlante batte un `null` che si
  // manifesta trenta righe più in là come «cannot read property of null».
  throw new Error('elemento #root non trovato in index.html')
}

createRoot(root).render(
  <StrictMode>
    <BrowserRouter>
      {/* La sessione sta **sopra** il router: il ripristino dal cookie parte una
          volta sola, e non a ogni cambio di pagina. */}
      <SessionProvider>
        <App />
      </SessionProvider>
    </BrowserRouter>
  </StrictMode>,
)
