import { Eye, EyeOff } from 'lucide-react'
import type { ComponentProps } from 'react'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { Link } from 'react-router'

import { confirmPasswordReset, requestPasswordReset } from '@/api/endpoints'
import { InlineError } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Field, Input } from '@/components/ui/field'
import { t } from '@/i18n'
import { cn } from '@/lib/utils'
import { useApi, useSession } from '@/app/session'

/**
 * **Questi due form non usano `zod`, e sono gli unici.**
 *
 * `NewLeasePage`, `NewPropertyPage` e `NewBillPage` validano con `zodResolver`, e
 * la coerenza direbbe di fare lo stesso anche qui. Non si fa apposta: quelle sono
 * rotte **pigre**, e il loro peso non è sulla prima visita; `/accesso` invece è la
 * prima schermata che un utente nuovo vede, senza niente in cache, e `zod` era un
 * chunk da 107,9 kB (32,5 gz) che nessun'altra rotta iniziale tirava dentro.
 *
 * Le regole di questi form sono tre — un'email formalmente valida, dodici
 * caratteri di password, dei campi obbligatori — e `react-hook-form` le esprime da
 * sé. Chi passasse di qui per «uniformare» rimetterebbe il chunk esattamente dove
 * costa di più.
 *
 * L'espressione dell'email è **volutamente larga**: serve a fermare un refuso
 * evidente prima del giro di rete, non a decidere che cosa sia un indirizzo — quello
 * lo dice il server, che resta l'autorità.
 */
const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/

/**
 * Dodici caratteri, come la policy del server.
 *
 * Esportata da `F7`: il recupero della password, qui in basso, la condivide con
 * il form di registrazione dall'invito
 * (`features/invitations/AcceptInvitationPage.tsx`) — lo stesso campo per
 * entrambi i generi di invito, da proprietario o da inquilino, perché è la
 * stessa regola del server (`password_policy::validate`) ripetuta qui per non
 * far fare un giro di rete a chi ha appena sbagliato a contare i caratteri.
 */
export const PASSWORD_MIN_LENGTH = 12

interface LoginValues {
  email: string
  password: string
}

export function AuthFrame({
  title,
  subtitle,
  children,
  footer,
}: {
  title: string
  subtitle: string
  children: React.ReactNode
  footer: React.ReactNode
}) {
  return (
    <main className="flex min-h-screen items-center justify-center px-4 py-10">
      <div className="w-full max-w-sm">
        <p className="mb-6 text-sm font-semibold text-accent">{t.app.name}</p>
        <h1 className="text-xl font-semibold tracking-tight text-ink">{title}</h1>
        <p className="mt-1 mb-6 text-sm text-ink-muted">{subtitle}</p>
        {children}
        <p className="mt-6 text-sm text-ink-muted">{footer}</p>
        {/* L'informativa va letta **prima** di consegnare i propri dati, quindi
            sta su ogni schermata in cui si consegnano. */}
        <p className="mt-4 text-xs text-ink-muted">
          <Link to="/privacy" className="underline-offset-4 hover:underline">
            {t.privacy.title}
          </Link>
        </p>
      </div>
    </main>
  )
}

/**
 * La password si può guardare, e il bottone sta **dentro** il campo.
 *
 * Non è una comodità estetica: dodici caratteri digitati alla cieca si sbagliano, e
 * chi non riesce a entrare non ha modo di sapere se ha sbagliato la password o il
 * tasto maiuscolo. Le tre cose che rendono il gesto vero e non decorativo:
 *
 * 1. **è raggiungibile da tastiera** — è un `<button>` nel giro di tabulazione, non
 *    un'icona con un `onClick`;
 * 2. **dice che cosa fa e in che stato è** — `aria-label` che cambia, e
 *    `aria-pressed`, perché l'icona da sola è ambigua fra «adesso è visibile» e
 *    «premi per renderla visibile»;
 * 3. **non ruba il fuoco al campo**: il `mousedown` è annullato, così chi sta
 *    digitando continua a digitare dove stava.
 *
 * Il `type` non è mai `text` di default: si torna nascosti a ogni montaggio.
 *
 * **Esportato**: lo riusa il form di registrazione dall'invito
 * (`features/invitations/AcceptInvitationPage.tsx`), che è un form di password
 * quanto questi — riscriverlo lì vorrebbe dire tenere due copie allineate a
 * mano.
 */
export function PasswordInput({ className, ...props }: ComponentProps<'input'>) {
  const [visible, setVisible] = useState(false)

  return (
    <div className="relative">
      <Input {...props} type={visible ? 'text' : 'password'} className={cn('pr-10', className)} />
      <Button
        variant="ghost"
        size="icon"
        onMouseDown={(event) => event.preventDefault()}
        onClick={() => setVisible((shown) => !shown)}
        aria-label={visible ? t.auth.hidePassword : t.auth.showPassword}
        aria-pressed={visible}
        className="absolute inset-y-0 right-0 text-ink-muted"
      >
        {visible ? <EyeOff aria-hidden /> : <Eye aria-hidden />}
      </Button>
    </div>
  )
}

export function LoginPage() {
  const { store } = useSession()
  const [error, setError] = useState<unknown>(null)
  const form = useForm<LoginValues>({
    defaultValues: { email: '', password: '' },
  })

  const submit = form.handleSubmit(async (values) => {
    setError(null)
    try {
      await store.login(values.email, values.password)
    } catch (failure) {
      // Il form **non si svuota**: chi ha sbagliato la password non deve
      // ridigitare anche l'email. È la differenza fra un'interfaccia che si prova
      // e una che si usa.
      setError(failure)
    }
  })

  return (
    <AuthFrame
      title={t.auth.loginTitle}
      subtitle={t.auth.loginSubtitle}
      footer={
        <>
          {/* `/invito`, non `/registrazione`: da `F6` non esiste una porta aperta
              — si entra solo da un invito, da proprietario o da inquilino, e
              questo link porta a dove quel codice si incolla (ADR-053). */}
          <Link to="/invito" className="text-accent underline-offset-4 hover:underline">
            {t.auth.toRegister}
          </Link>
          {/* **La via d'uscita sta dove ci si accorge di non ricordarla**, cioè
              sotto il form che ha appena rifiutato la password. Metterla altrove
              vorrebbe dire cercarla nel momento peggiore. */}
          <span className="mt-1 block">
            <Link
              to="/recupero-password"
              className="text-accent underline-offset-4 hover:underline"
            >
              {t.auth.forgotten}
            </Link>
          </span>
        </>
      }
    >
      <form onSubmit={submit} noValidate className="flex flex-col gap-4">
        <InlineError error={error} />
        <Field label={t.auth.email} error={form.formState.errors.email?.message} required>
          {(props) => (
            <Input
              {...props}
              {...form.register('email', {
                required: t.validation.required,
                pattern: { value: EMAIL_PATTERN, message: t.validation.email },
              })}
              type="email"
              autoComplete="email"
              autoFocus
            />
          )}
        </Field>
        <Field label={t.auth.password} error={form.formState.errors.password?.message} required>
          {(props) => (
            <PasswordInput
              {...props}
              {...form.register('password', { required: t.validation.required })}
              autoComplete="current-password"
            />
          )}
        </Field>
        <Button type="submit" variant="primary" size="lg" disabled={form.formState.isSubmitting}>
          {form.formState.isSubmitting ? t.common.saving : t.auth.submitLogin}
        </Button>
      </form>
    </AuthFrame>
  )
}

/**
 * `/registrazione`, da fuori sessione: non più una pagina con un form (`F6` ha
 * chiuso `POST /auth/register`), ma un rimando che dice **perché** — chi arriva
 * da un segnalibro vecchio o da un motore di ricerca non deve leggere un `404`
 * muto, deve leggere che si entra da un invito e dove andare (ADR-053).
 */
export function RegistrationMovedPage() {
  return (
    <AuthFrame
      title={t.auth.registrationMovedTitle}
      subtitle={t.auth.registrationMovedBody}
      footer={
        <Link to="/accesso" className="text-accent underline-offset-4 hover:underline">
          {t.auth.toLogin}
        </Link>
      }
    >
      <Button asChild variant="primary" size="lg">
        <Link to="/invito">{t.auth.registrationMovedAction}</Link>
      </Button>
    </AuthFrame>
  )
}

/**
 * Il recupero della password, in **due tempi e una schermata sola**.
 *
 * Due tempi perché il codice arriva per posta: si chiede, si aspetta, si incolla.
 * Una schermata sola perché fra i due passaggi non c'è niente da ricordare tranne
 * l'indirizzo, e mandare una persona già frustrata su una seconda pagina — con un
 * link da ritrovare o un URL da riaprire — è attrito che non compra niente.
 *
 * # Il codice si incolla, non si clicca
 *
 * È [ADR-052](../../../../docs/08-decisioni.md), che riprende la stessa decisione già
 * presa per l'invito: il token è una **credenziale**, e un link
 * `/nuova-password/<token>` la scriverebbe in tre archivi che nessuno ripulisce — la
 * cronologia del browser, il `Referer` verso qualunque risorsa esterna, e i log di
 * ogni proxy davanti al client. Il costo è reale — incollare è più scomodo di un
 * clic — e si paga per non avere due forme diverse per due credenziali che si
 * somigliano.
 *
 * # La schermata dice la verità sulla risposta
 *
 * L'API risponde `202` a chiunque, perché distinguere un indirizzo noto da uno
 * sconosciuto direbbe chi ha un account. Il messaggio quindi **non** promette
 * «ti abbiamo mandato un'email»: dice «se quell'indirizzo ha un account». È la
 * verità, e una promessa che il prodotto non può mantenere sarebbe peggio.
 *
 * # E dice la conseguenza **prima**
 *
 * Cambiare la password revoca le sessioni aperte ovunque, ed è la parte che si
 * scopre dopo trovandosi disconnessi sul telefono. Sta scritta accanto al bottone.
 */
export function PasswordResetPage() {
  const [stage, setStage] = useState<'request' | 'confirm'>('request')
  const [email, setEmail] = useState('')
  const [error, setError] = useState<unknown>(null)
  const [done, setDone] = useState(false)
  const client = useApi()

  const request = useForm<{ email: string }>({ defaultValues: { email: '' } })
  const confirm = useForm<{ code: string; newPassword: string }>({
    defaultValues: { code: '', newPassword: '' },
  })

  const askForCode = request.handleSubmit(async (values) => {
    setError(null)
    try {
      await requestPasswordReset(client, values.email)
      setEmail(values.email)
      setStage('confirm')
    } catch (failure) {
      setError(failure)
    }
  })

  const useCode = confirm.handleSubmit(async (values) => {
    setError(null)
    try {
      await confirmPasswordReset(client, values.code.trim(), values.newPassword)
      setDone(true)
    } catch (failure) {
      // Il form **non si svuota**: chi ha sbagliato la password nuova non deve
      // farsi rimandare il codice, che vale una volta sola.
      setError(failure)
    }
  })

  if (done) {
    return (
      <AuthFrame
        title={t.auth.resetConfirmTitle}
        subtitle={t.auth.resetDone}
        footer={
          <Link to="/accesso" className="text-accent underline-offset-4 hover:underline">
            {t.auth.backToLogin}
          </Link>
        }
      >
        <Button asChild variant="primary" size="lg">
          <Link to="/accesso">{t.auth.submitLogin}</Link>
        </Button>
      </AuthFrame>
    )
  }

  return (
    <AuthFrame
      title={t.auth.resetTitle}
      subtitle={stage === 'request' ? t.auth.resetSubtitle : t.auth.resetConfirmSubtitle}
      footer={
        <Link to="/accesso" className="text-accent underline-offset-4 hover:underline">
          {t.auth.backToLogin}
        </Link>
      }
    >
      {stage === 'request' ? (
        <form onSubmit={askForCode} noValidate className="flex flex-col gap-4">
          <InlineError error={error} />
          <Field label={t.auth.email} error={request.formState.errors.email?.message} required>
            {(props) => (
              <Input
                {...props}
                {...request.register('email', {
                  required: t.validation.required,
                  pattern: { value: EMAIL_PATTERN, message: t.validation.email },
                })}
                type="email"
                autoComplete="email"
                autoFocus
              />
            )}
          </Field>
          <Button
            type="submit"
            variant="primary"
            size="lg"
            disabled={request.formState.isSubmitting}
          >
            {request.formState.isSubmitting ? t.common.saving : t.auth.resetSubmit}
          </Button>
        </form>
      ) : (
        <form onSubmit={useCode} noValidate className="flex flex-col gap-4">
          <InlineError error={error} />
          {/* La verità, non una promessa: l'API risponde `202` anche a un indirizzo
              che non ha un account, e dire «ti abbiamo mandato un'email» sarebbe
              falso proprio nel caso in cui non è partito niente. */}
          <p className="rounded-lg bg-surface-muted px-3 py-2 text-xs text-ink-muted">
            {t.auth.resetSent}
          </p>
          <Field
            label={t.auth.resetCode}
            hint={t.auth.resetCodeHint}
            error={confirm.formState.errors.code?.message}
            required
          >
            {(props) => (
              <Input
                {...props}
                {...confirm.register('code', { required: t.validation.required })}
                autoComplete="one-time-code"
                autoFocus
              />
            )}
          </Field>
          <Field
            label={t.auth.newPassword}
            hint={t.auth.passwordHint}
            error={confirm.formState.errors.newPassword?.message}
            required
          >
            {(props) => (
              <PasswordInput
                {...props}
                {...confirm.register('newPassword', {
                  required: t.validation.required,
                  minLength: { value: PASSWORD_MIN_LENGTH, message: t.validation.passwordTooShort },
                })}
                autoComplete="new-password"
              />
            )}
          </Field>
          {/* La conseguenza **prima**, non dopo: chi cambia la password si ritrova
              disconnesso anche sul telefono, e scoprirlo dopo sembra un guasto. */}
          <p className="rounded-lg border border-warning/30 bg-warning-soft px-3 py-2 text-xs text-warning">
            {t.auth.resetRevokesSessions}
          </p>
          <Button
            type="submit"
            variant="primary"
            size="lg"
            disabled={confirm.formState.isSubmitting}
          >
            {confirm.formState.isSubmitting ? t.common.saving : t.auth.resetConfirmSubmit}
          </Button>
          {/* L'indirizzo resta a schermo: chi ha sbagliato a digitarlo se ne
              accorge qui, invece di aspettare un'email che non arriverà. */}
          <p className="text-center text-xs text-ink-muted">{email}</p>
        </form>
      )}
    </AuthFrame>
  )
}
