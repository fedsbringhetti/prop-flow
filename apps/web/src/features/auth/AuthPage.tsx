import { zodResolver } from '@hookform/resolvers/zod'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { Link, useNavigate } from 'react-router'
import { z } from 'zod'

import { ApiError, fieldErrorFor } from '@/api/problem'
import { InlineError } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Field, Input } from '@/components/ui/field'
import { t } from '@/i18n'
import { useSession } from '@/app/session'

const loginSchema = z.object({
  email: z.email({ message: t.validation.email }),
  password: z.string().min(1, t.validation.required),
})

const registerSchema = z.object({
  email: z.email({ message: t.validation.email }),
  fullName: z.string().min(1, t.validation.required),
  // Dodici caratteri, come la policy del server: verificarlo qui evita un giro
  // di rete per dire una cosa che si sa già, e **non** sostituisce il controllo
  // del server, che resta l'autorità.
  password: z.string().min(12, t.validation.passwordTooShort),
  organizationName: z.string().optional(),
})

type LoginValues = z.infer<typeof loginSchema>
type RegisterValues = z.infer<typeof registerSchema>

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

export function LoginPage() {
  const { store } = useSession()
  const [error, setError] = useState<unknown>(null)
  const form = useForm<LoginValues>({
    resolver: zodResolver(loginSchema),
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
              {...form.register('email')}
              type="email"
              autoComplete="email"
              autoFocus
            />
          )}
        </Field>
        <Field label={t.auth.password} error={form.formState.errors.password?.message} required>
          {(props) => (
            <Input
              {...props}
              {...form.register('password')}
              type="password"
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
    resolver: zodResolver(registerSchema),
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
            <Input {...props} {...form.register('fullName')} autoComplete="name" autoFocus />
          )}
        </Field>
        <Field label={t.auth.email} error={form.formState.errors.email?.message} required>
          {(props) => (
            <Input {...props} {...form.register('email')} type="email" autoComplete="email" />
          )}
        </Field>
        <Field
          label={t.auth.password}
          hint={t.auth.passwordHint}
          error={form.formState.errors.password?.message}
          required
        >
          {(props) => (
            <Input
              {...props}
              {...form.register('password')}
              type="password"
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
