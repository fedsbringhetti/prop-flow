import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { Link, useNavigate } from 'react-router'

import { acceptInvitation } from '@/api/endpoints'
import { ApiError, fieldErrorFor } from '@/api/problem'
import type { OrgRole } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Field, Input } from '@/components/ui/field'
import { PageHeader, PageSection } from '@/components/ui/page'
import { Card, CardBody } from '@/components/ui/card'
import { AuthFrame, PASSWORD_MIN_LENGTH, PasswordInput } from '@/features/auth/AuthPage'
import { InvitationCodeError } from './errors'
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
              <InvitationCodeError error={mutation.error} />
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

interface InviteRegisterValues {
  fullName: string
  password: string
  organizationName: string
}

/**
 * `/invito`, da fuori sessione (`F7`, ADR-053): il codice **prima**, e solo dopo
 * il form di registrazione, con il codice già dentro e **senza il campo
 * email** — non ne serve uno, l'email è quella dell'invito.
 *
 * # Due passi, e non una rotta di sola lettura prima del form
 *
 * Sapere se un codice è da proprietario o da inquilino **prima** di mostrare il
 * form giusto vorrebbe dire una rotta che lo dica senza consumarlo — cioè un
 * oracolo su quali codici esistono, raggiungibile da chiunque ne provi uno a
 * caso. Il form si manda e basta: se il codice è da proprietario, il server
 * risponde `409 invitation.owner_token` e questa schermata chiede il nome
 * dell'organizzazione e rimanda lo stesso corpo a `accept-owner-invitation`,
 * senza far ridigitare nome e password. Chi indovina un codice riceve `401`,
 * come per un token inesistente — non un secondo oracolo.
 *
 * # Due passi, e non un modulo solo
 *
 * Il codice arriva da un messaggio e si incolla; nome e password si scelgono.
 * Chiederli in un unico form costringerebbe a tenere a mente un codice lungo
 * mentre si pensa a una password, ed è la stessa ragione per cui il recupero
 * della password li separa (ADR-052).
 *
 * # `POST /auth/accept-invitation` resta per chi ha già un account
 *
 * Questa schermata copre solo chi **non** ne ha uno: chi torna — l'inquilino di
 * [ADR-051](../../../../docs/08-decisioni.md), o un proprietario invitato altrove
 * — passa dal login e trova `AcceptInvitationPage`, sopra in questo file.
 */
export function AcceptInvitationSignedOutPage() {
  const { store } = useSession()
  const navigate = useNavigate()

  const [stage, setStage] = useState<'code' | 'register'>('code')
  const [token, setToken] = useState('')
  // Compare solo dopo il `409`: prima non si sa ancora di che genere sia il
  // codice, e mostrarlo da subito chiederebbe un nome d'organizzazione anche a
  // chi sta solo accettando un invito da inquilino.
  const [needsOrganizationName, setNeedsOrganizationName] = useState(false)
  const [error, setError] = useState<unknown>(null)

  const form = useForm<InviteRegisterValues>({
    defaultValues: { fullName: '', password: '', organizationName: '' },
  })

  const submit = form.handleSubmit(async (values) => {
    setError(null)
    try {
      const session = needsOrganizationName
        ? await store.acceptOwnerInvitation({
            token,
            fullName: values.fullName,
            password: values.password,
            organizationName: values.organizationName || undefined,
          })
        : await store.registerFromInvitation({
            token,
            fullName: values.fullName,
            password: values.password,
          })
      // Niente da invalidare qui, e non per dimenticanza: questa pagina vive
      // **fuori** da `QueryProvider` — le rotte da fuori sessione non hanno un
      // client, e `useQueryClient()` qui è una pagina bianca, non un errore
      // gestito. Quando la sessione nasce, `App` monta il ramo autenticato con
      // un `QueryClient` nuovo: l'elenco delle organizzazioni parte da zero.
      // La pagina iniziale è quella del **ruolo**, non la radice: per chi abita
      // la radice rimanda comunque allo scadenzario, e passarci in mezzo
      // farebbe lampeggiare una schermata che non è la sua.
      await navigate(session.role === 'tenant' ? TENANT_HOME : '/', { replace: true })
    } catch (failure) {
      // Il codice era giusto, serviva solo l'altra rotta: non è un errore da
      // mostrare, è il segnale che manca un campo. Nome e password restano.
      if (
        !needsOrganizationName &&
        failure instanceof ApiError &&
        failure.code === 'invitation.owner_token'
      ) {
        setNeedsOrganizationName(true)
        return
      }
      const passwordProblem = fieldErrorFor(failure, 'password')
      if (passwordProblem) {
        form.setError('password', { message: passwordProblem })
        return
      }
      setError(failure)
    }
  })

  if (stage === 'code') {
    return (
      <AuthFrame
        title={t.invitation.entryTitle}
        subtitle={t.invitation.entrySubtitle}
        footer={
          <Link to="/accesso" className="text-accent underline-offset-4 hover:underline">
            {t.auth.toLogin}
          </Link>
        }
      >
        <form
          noValidate
          className="flex flex-col gap-4"
          onSubmit={(event) => {
            event.preventDefault()
            if (token.trim() === '') return
            setStage('register')
          }}
        >
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
          <Button type="submit" variant="primary" size="lg" disabled={token.trim() === ''}>
            {t.invitation.entryContinue}
          </Button>
        </form>
      </AuthFrame>
    )
  }

  return (
    <AuthFrame
      title={t.invitation.registerTitle}
      subtitle={t.invitation.registerSubtitle}
      footer={
        <button
          type="button"
          onClick={() => {
            setStage('code')
            setNeedsOrganizationName(false)
            setError(null)
          }}
          className="text-accent underline-offset-4 hover:underline"
        >
          {t.invitation.backToCode}
        </button>
      }
    >
      <form onSubmit={submit} noValidate className="flex flex-col gap-4">
        <InvitationCodeError error={error} />
        <Field label={t.auth.fullName} error={form.formState.errors.fullName?.message} required>
          {(props) => (
            <Input
              {...props}
              {...form.register('fullName', { required: t.validation.required })}
              autoComplete="name"
              autoFocus
            />
          )}
        </Field>
        <Field
          label={t.auth.password}
          hint={t.auth.passwordHint}
          error={form.formState.errors.password?.message}
          required
        >
          {(props) => (
            <PasswordInput
              {...props}
              {...form.register('password', {
                required: t.validation.required,
                // Verificarlo qui evita un giro di rete per dire una cosa che
                // si sa già, e **non** sostituisce il controllo del server,
                // che resta l'autorità.
                minLength: { value: PASSWORD_MIN_LENGTH, message: t.validation.passwordTooShort },
              })}
              autoComplete="new-password"
            />
          )}
        </Field>
        {/* Solo dopo il `409 invitation.owner_token`: il codice crea anche
            un'organizzazione, e solo lui la chiede. */}
        {needsOrganizationName ? (
          <>
            <p className="rounded-lg bg-surface-muted px-3 py-2 text-xs text-ink-muted">
              {t.invitation.ownerTokenPrompt}
            </p>
            <Field
              label={t.auth.organizationName}
              hint={t.auth.organizationNameHint}
              error={form.formState.errors.organizationName?.message}
            >
              {(props) => <Input {...props} {...form.register('organizationName')} autoFocus />}
            </Field>
          </>
        ) : null}
        <Button type="submit" variant="primary" size="lg" disabled={form.formState.isSubmitting}>
          {form.formState.isSubmitting ? t.common.saving : t.invitation.registerSubmit}
        </Button>
      </form>
    </AuthFrame>
  )
}
