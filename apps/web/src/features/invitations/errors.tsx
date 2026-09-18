import { ApiError } from '@/api/problem'
import { InlineError } from '@/components/state'
import { t } from '@/i18n'

/**
 * I rifiuti di un codice d'invito, tradotti **qui** e non nel catalogo globale.
 *
 * `auth.invalid_credentials` è la risposta a un codice sbagliato, scaduto o
 * intestato a un altro indirizzo — l'API non li distingue apposta, perché
 * distinguerli direbbe a chi prova quali inviti esistono. Nel catalogo quel codice
 * significa «email o password non corrette», che qui sarebbe una frase su una
 * password che nessuno ha digitato: il messaggio è dello schermo, non del codice.
 *
 * **In un file a sé**, non dentro `AcceptInvitationPage.tsx`: la stessa forma di
 * rifiuto la incontrano tre schermate — l'accettazione da autenticati, la
 * registrazione dall'invito fuori sessione, e la schermata del guscio senza
 * organizzazione — e l'ultima vive in `app/shell.tsx`, che `AcceptInvitationPage`
 * già importa per `TENANT_HOME`. Esportarla da lì avrebbe chiuso un ciclo fra i
 * due moduli.
 */
export function InvitationCodeError({ error }: { error: unknown }) {
  if (!error) return null

  const local =
    error instanceof ApiError
      ? error.code === 'auth.invalid_credentials'
        ? t.invitation.acceptCodeRejected
        : error.code === 'invitation.already_accepted'
          ? t.invitation.acceptAlreadyDone
          : null
      : null

  if (!local) return <InlineError error={error} />
  return (
    <p
      role="alert"
      className="rounded-lg border border-danger/30 bg-danger-soft px-3 py-2 text-sm text-danger"
    >
      {local}
    </p>
  )
}
