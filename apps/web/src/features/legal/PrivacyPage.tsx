import { Link } from 'react-router'

import { t } from '@/i18n'
import { formatDate } from '@/lib/format'

/**
 * L'informativa sulla privacy.
 *
 * # Perché esiste adesso e non dopo
 *
 * È l'unica voce di questa fase che **cresce di costo con il tempo**: scriverla
 * prima che dentro ci siano i dati di qualcuno è un pomeriggio, scriverla dopo sei
 * mesi di raccolta è un'operazione su dati che esistono già. Appena i dati degli
 * inquilini di un proprietario entrano nel server, si stanno trattando dati
 * personali di terzi.
 *
 * # È raggiungibile **senza sessione**
 *
 * Sta fra le rotte di chi non è entrato, e nel piè di pagina delle schermate di
 * accesso e registrazione: un'informativa che si legge solo da dentro non è
 * un'informativa, perché la si deve poter leggere **prima** di dare i propri dati.
 *
 * # Dice quello che il prodotto fa, non quello che vorrebbe
 *
 * La cancellazione è una richiesta scritta a cui si risponde entro trenta giorni, e
 * non un bottone: il ruolo applicativo non ha il privilegio `DELETE`, tutto passa da
 * `deleted_at`, e `audit_log` e gli eventi continuano a citare le righe. Promettere
 * un bottone sarebbe promettere ciò che lo schema vieta — la capacità vera è `R37`.
 */
export function PrivacyPage() {
  return (
    <main className="mx-auto w-full max-w-2xl px-4 py-10">
      <p className="mb-6 text-sm font-semibold text-accent">{t.app.name}</p>
      <h1 className="text-xl font-semibold tracking-tight text-ink">{t.privacy.title}</h1>
      <p className="mt-1 text-sm text-ink-muted">{t.privacy.subtitle}</p>
      <p className="mt-1 text-xs text-ink-muted">
        {t.privacy.updated}: {formatDate(PRIVACY_UPDATED)}
      </p>

      <div className="mt-8 flex flex-col gap-6">
        {t.privacy.sections.map((section) => (
          <section key={section.title}>
            <h2 className="text-sm font-semibold text-ink">{section.title}</h2>
            <p className="mt-1 text-sm leading-relaxed text-ink-muted">{section.body}</p>
          </section>
        ))}
      </div>

      {/* **L'indirizzo è una configurazione del deploy, non una stringa del
          catalogo**: cambia da installazione a installazione, e scriverlo qui
          vorrebbe dire ricompilare il client per correggere un contatto. Il
          ripiego non è uno spazio vuoto — che farebbe sembrare la pagina rotta —
          ma il nome della variabile da valorizzare. */}
      <p className="mt-8 rounded-lg bg-surface-muted px-3 py-2 text-sm text-ink-muted">
        {t.privacy.contactLabel}: {PRIVACY_CONTACT}
      </p>

      <p className="mt-8 text-sm">
        <Link to="/accesso" className="text-accent underline-offset-4 hover:underline">
          {t.auth.backToLogin}
        </Link>
      </p>
    </main>
  )
}

/**
 * La data dell'ultima revisione, **scritta a mano**.
 *
 * Non è `new Date()`: un'informativa che si dichiara aggiornata a oggi ogni volta
 * che qualcuno la apre è un'informativa che mente. Si cambia quando il testo cambia.
 */
const PRIVACY_UPDATED = '2026-09-10'

/**
 * Il contatto del titolare, dal `.env` del client.
 *
 * `import.meta.env` è sostituito a build time da Vite: cambia per installazione, e
 * non è un dato del prodotto.
 */
const PRIVACY_CONTACT: string =
  (import.meta.env['VITE_PRIVACY_CONTACT'] as string | undefined) ?? 'VITE_PRIVACY_CONTACT'
