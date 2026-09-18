import { Link } from 'react-router'

import { useIsStaff } from '@/app/session'
import { t } from '@/i18n'

/**
 * La guida: come si usa il sito.
 *
 * # Una pagina, due versioni
 *
 * Il ruolo sceglie il testo, non la rotta: due pagine sarebbero il doppio dei
 * collegamenti e dei posti in cui il testo diverge senza che nessuno se ne
 * accorga. Chi non ha un ruolo — la schermata senza organizzazione, che aspetta
 * un invito — legge la versione di chi abita, perché è quella di chi sta per
 * entrare da un invito.
 *
 * # Ha la forma di `PrivacyPage`, e sta **fuori** dal guscio
 *
 * Il guscio senza organizzazione rende `NoOrganizationScreen` al posto
 * dell'`Outlet`, per ogni rotta: una guida dentro il guscio non sarebbe
 * raggiungibile da lì, che è uno dei due posti da cui la si apre. Fuori, è la
 * stessa pagina lunga dell'informativa, con un rimando all'applicazione in fondo:
 * `/` è del ruolo, e porta ciascuno dove atterra.
 *
 * # Le sezioni si chiamano come le voci della navigazione
 *
 * `t.guide.*.sections[].title` è `nav.*`, non una copia, così una voce rinominata
 * rinomina anche la sezione che la spiega. I paragrafi stanno nel catalogo come
 * `privacy.sections`: sono la parte in cui il testo nel JSX è più facile da
 * scrivere, ed è la regola 18.
 */
export function GuidePage() {
  const isStaff = useIsStaff()
  const version = isStaff ? t.guide.staff : t.guide.tenant

  return (
    <main className="mx-auto w-full max-w-2xl px-4 py-10">
      <p className="mb-6 text-sm font-semibold text-accent">{t.app.name}</p>
      <h1 className="text-xl font-semibold tracking-tight text-ink">{t.guide.title}</h1>
      <p className="mt-1 text-sm text-ink-muted">{version.subtitle}</p>

      <div className="mt-8 flex flex-col gap-6">
        {version.sections.map((section) => (
          <section key={section.title}>
            <h2 className="text-sm font-semibold text-ink">{section.title}</h2>
            <p className="mt-1 text-sm leading-relaxed text-ink-muted">{section.body}</p>
          </section>
        ))}
      </div>

      <p className="mt-8 text-sm">
        <Link to="/" className="text-accent underline-offset-4 hover:underline">
          {t.guide.back}
        </Link>
      </p>
    </main>
  )
}
