import { Eye, EyeOff } from 'lucide-react'
import type { ComponentProps } from 'react'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { Link, useNavigate } from 'react-router'

import { ApiError, fieldErrorFor } from '@/api/problem'
import { InlineError } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Field, Input } from '@/components/ui/field'
import { t } from '@/i18n'
import { cn } from '@/lib/utils'
import { useSession } from '@/app/session'

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

/** Dodici caratteri, come la policy del server. */
const PASSWORD_MIN_LENGTH = 12

interface LoginValues {
  email: string
  password: string
}

interface RegisterValues {
  email: string
  fullName: string
  password: string
  organizationName: string
}

function AuthFrame({
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
 */
function PasswordInput({ className, ...props }: ComponentProps<'input'>) {
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
        <Link to="/registrazione" className="text-accent underline-offset-4 hover:underline">
          {t.auth.toRegister}
        </Link>
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

export function RegisterPage() {
  const { store } = useSession()
  const navigate = useNavigate()
  const [error, setError] = useState<unknown>(null)
  const form = useForm<RegisterValues>({
    defaultValues: { email: '', fullName: '', password: '', organizationName: '' },
  })

  const submit = form.handleSubmit(async (values) => {
    setError(null)
    try {
      await store.register({
        email: values.email,
        fullName: values.fullName,
        password: values.password,
        organizationName: values.organizationName || undefined,
      })
      // La registrazione **non** apre una sessione: risponde `201` con gli
      // identificatori e basta. Si entra subito con le stesse credenziali, così
      // chi si è appena registrato non deve digitarle una seconda volta.
      await store.login(values.email, values.password)
    } catch (failure) {
      if (failure instanceof ApiError && failure.code === 'auth.email_taken') {
        form.setError('email', { message: t.errors['auth.email_taken'] })
        return
      }
      const passwordProblem = fieldErrorFor(failure, 'password')
      if (passwordProblem) {
        form.setError('password', { message: passwordProblem })
        return
      }
      setError(failure)
      void navigate('/registrazione', { replace: true })
    }
  })

  return (
    <AuthFrame
      title={t.auth.registerTitle}
      subtitle={t.auth.registerSubtitle}
      footer={
        <Link to="/accesso" className="text-accent underline-offset-4 hover:underline">
          {t.auth.toLogin}
        </Link>
      }
    >
      <form onSubmit={submit} noValidate className="flex flex-col gap-4">
        <InlineError error={error} />
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
                // Verificarlo qui evita un giro di rete per dire una cosa che si
                // sa già, e **non** sostituisce il controllo del server, che resta
                // l'autorità.
                minLength: { value: PASSWORD_MIN_LENGTH, message: t.validation.passwordTooShort },
              })}
              autoComplete="new-password"
            />
          )}
        </Field>
        <Field
          label={t.auth.organizationName}
          hint={t.auth.organizationNameHint}
          error={form.formState.errors.organizationName?.message}
        >
          {(props) => <Input {...props} {...form.register('organizationName')} />}
        </Field>
        <Button type="submit" variant="primary" size="lg" disabled={form.formState.isSubmitting}>
          {form.formState.isSubmitting ? t.common.saving : t.auth.submitRegister}
        </Button>
      </form>
    </AuthFrame>
  )
}
