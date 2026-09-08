import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { Link, useNavigate } from 'react-router'

import { acceptInvitation } from '@/api/endpoints'
import { ApiError } from '@/api/problem'
import type { OrgRole } from '@/api/types'
import { InlineError } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Field, Input } from '@/components/ui/field'
import { PageHeader, PageSection } from '@/components/ui/page'
import { Card, CardBody } from '@/components/ui/card'
import { keys } from '@/app/query'
import { useApi, useSession } from '@/app/session'
import { TENANT_HOME } from '@/app/shell'
import { t } from '@/i18n'

/**
 * Il codice si **incolla**, e non arriva da un URL.
 *
 * `accept_token` è una credenziale: chi ce l'ha entra in un'organizzazione. Un
 * link `/invito/<token>` costerebbe un clic in meno e scriverebbe quella
 * credenziale in tre archivi che nessuno ripulisce — la cronologia del browser,
 * il `Referer` verso qualunque risorsa esterna, e i log di ogni proxy davanti al
 * client. La modale che consegna il codice lo chiama già «codice» e dice che vale
 * sette giorni: questa schermata resta coerente con quella, ed è un campo.
 *
 * # I ruoli che l'API può rispondere
 *
 * Un invito conferisce soltanto `tenant` — il vincolo è nel dominio e ripetuto
 * nella policy di riga — ma il ruolo si legge **dalla risposta** e non si assume:
 * il giorno in cui l'API ne concedesse un altro, questa schermata manderebbe la
 * persona nel posto giusto invece che in quello previsto oggi.
 */
export function AcceptInvitationPage() {
  const client = useApi()
  const queryClient = useQueryClient()
  const { selectOrganization } = useSession()
  const navigate = useNavigate()
  const [token, setToken] = useState('')

  const mutation = useMutation({
    mutationFn: () => acceptInvitation(client, token.trim()),
    onSuccess: async (accepted) => {
      const role = accepted.role as OrgRole

      // **L'organizzazione appena accettata diventa quella attiva**, ed è l'unico
      // momento in cui il client sa quale si vuole: questa persona l'ha appena
      // detta. Senza, atterrerebbe nella propria organizzazione personale —
      // `pick_default_membership` preferisce il ruolo più alto e la registrazione
      // ne crea sempre una di cui si è owner — cioè su una schermata vuota che
      // invita a registrare il primo immobile.
      selectOrganization(accepted.organization_id, role)

      // L'elenco delle organizzazioni ha `staleTime` di cinque minuti perché
      // «cambia quando si accetta un invito»: è questo il momento.
      await queryClient.invalidateQueries({ queryKey: keys.organizations })

      // La pagina iniziale è quella del **ruolo**, non la radice: per chi abita
      // la radice rimanda comunque allo scadenzario, e passarci in mezzo
      // farebbe lampeggiare una schermata che non è la sua.
      await navigate(role === 'tenant' ? TENANT_HOME : '/', { replace: true })
    },
  })

  return (
    <>
      <PageHeader title={t.invitation.acceptTitle} description={t.invitation.acceptSubtitle} />
      <PageSection>
        <Card>
          <CardBody>
            <form
              noValidate
              className="flex max-w-md flex-col gap-4"
              onSubmit={(event) => {
                event.preventDefault()
                mutation.mutate()
              }}
            >
              <AcceptError error={mutation.error} />
              <Field label={t.invitation.tokenLabel} hint={t.invitation.acceptCodeHint} required>
                {(props) => (
                  <Input
                    {...props}
                    value={token}
                    autoFocus
                    // Un codice incollato da un messaggio si porta dietro spazi e
                    // maiuscole automatiche del telefono: nessuna delle due fa
                    // parte del token.
                    autoCapitalize="off"
                    autoCorrect="off"
                    spellCheck={false}
                    onChange={(event) => setToken(event.target.value)}
                  />
                )}
              </Field>
              <div>
                <Button
                  type="submit"
                  variant="primary"
                  size="lg"
                  disabled={mutation.isPending || token.trim() === ''}
                >
                  {mutation.isPending ? t.common.saving : t.invitation.acceptSubmit}
                </Button>
              </div>
            </form>
          </CardBody>
        </Card>
      </PageSection>
    </>
  )
}

/**
 * I rifiuti di questa rotta, tradotti **qui** e non nel catalogo globale.
 *
 * `auth.invalid_credentials` è la risposta a un codice sbagliato, scaduto o
 * intestato a un altro indirizzo — l'API non li distingue apposta, perché
 * distinguerli direbbe a chi prova quali inviti esistono. Nel catalogo quel codice
 * significa «email o password non corrette», che qui sarebbe una frase su una
 * password che nessuno ha digitato: il messaggio è dello schermo, non del codice.
 */
function AcceptError({ error }: { error: unknown }) {
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

/**
 * La stessa schermata per chi **non ha ancora un account**.
 *
 * `POST /auth/accept-invitation` è autenticata, quindi accettare richiede una
 * sessione. Mandare qui chi arriva senza produrrebbe un `401` su un codice
 * giusto: la conseguenza si progetta invece di subirla, e si dice che cosa fare
 * prima — con i due link, e con la promessa che si torna qui.
 */
export function AcceptInvitationSignedOutPage() {
  return (
    <main className="flex min-h-screen items-center justify-center px-4 py-10">
      <div className="w-full max-w-sm">
        <p className="mb-6 text-sm font-semibold text-accent">{t.app.name}</p>
        <h1 className="text-xl font-semibold tracking-tight text-ink">
          {t.invitation.acceptNeedsAccountTitle}
        </h1>
        <p className="mt-1 mb-6 text-sm text-ink-muted">{t.invitation.acceptNeedsAccountBody}</p>
        <div className="flex flex-col gap-3">
          <Button asChild variant="primary" size="lg">
            <Link to="/registrazione" state={{ from: '/invito' }}>
              {t.auth.submitRegister}
            </Link>
          </Button>
          <Button asChild size="lg">
            <Link to="/accesso" state={{ from: '/invito' }}>
              {t.auth.submitLogin}
            </Link>
          </Button>
        </div>
      </div>
    </main>
  )
}
