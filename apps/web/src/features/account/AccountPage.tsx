import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Eye, EyeOff } from 'lucide-react'
import { useState, type ComponentProps } from 'react'
import { useForm } from 'react-hook-form'

import { changePassword, getAccount, leaveOrganization, updateAccount } from '@/api/endpoints'
import type { Account, Items, Organization } from '@/api/types'
import { Async, InlineError } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card, CardBody, CardHeader, CardTitle } from '@/components/ui/card'
import { ConfirmAction } from '@/components/ui/confirm'
import { Field, Input } from '@/components/ui/field'
import { PageHeader } from '@/components/ui/page'
import { t } from '@/i18n'
import { cn } from '@/lib/utils'
import { keys } from '@/app/query'
import { useApi, useSession } from '@/app/session'
import { useOrganizations } from '@/app/shell'

const PASSWORD_MIN_LENGTH = 12

/**
 * Il proprio account: i dati anagrafici, la password, l'uscita dalle
 * organizzazioni.
 *
 * # Perché una schermata sola e non tre
 *
 * Sono le tre cose che una persona fa **sul proprio account** e non su un modulo
 * del prodotto, e cercarle in tre posti diversi sarebbe cercarle. Il menù
 * dell'organizzazione le raggiunge da un'unica voce.
 *
 * # `GET /me` **non dipende dall'organizzazione attiva**
 *
 * Come `getOrganizations`: i propri dati non appartengono a nessuna organizzazione.
 * È anche ciò che rende questa schermata l'unica utile a chi è appena uscito
 * dall'ultima in cui abitava, e non ha più niente di attivo.
 */
export function AccountPage() {
  const client = useApi()
  const query = useQuery({
    queryKey: keys.account,
    queryFn: () => getAccount(client),
  })

  return (
    <div className="mx-auto w-full max-w-2xl">
      <PageHeader title={t.account.title} description={t.account.subtitle} />
      <Async query={query}>
        {(account) => (
          <div className="flex flex-col gap-6">
            <ProfileCard account={account} />
            <PasswordCard />
            <LeaveCard />
          </div>
        )}
      </Async>
    </div>
  )
}

/**
 * I dati anagrafici, e l'email che si mostra e non si tocca.
 *
 * **Il campo dell'email è disabilitato con una riga che dice perché**: un campo
 * spento senza spiegazione si legge come un difetto, mentre l'email è la
 * credenziale d'accesso e cambiarla richiede la verifica del nuovo indirizzo, che
 * è fuori perimetro.
 */
function ProfileCard({ account }: { account: Account }) {
  const client = useApi()
  const queryClient = useQueryClient()
  const [saved, setSaved] = useState(false)
  const form = useForm({
    defaultValues: {
      fullName: account.full_name,
      phone: account.phone ?? '',
    },
  })

  const save = useMutation({
    mutationFn: (values: { fullName: string; phone: string }) =>
      updateAccount(client, {
        full_name: values.fullName.trim(),
        // Stringa vuota significa «togli», che sul filo è `null` esplicito e non
        // campo assente: senza la distinzione un telefono non si potrebbe mai
        // cancellare. Un valore c'è invece va inviato così com'è.
        phone: values.phone.trim() === '' ? null : values.phone.trim(),
      }),
    onSuccess: (updated) => {
      setSaved(true)
      queryClient.setQueryData(keys.account, updated)
      form.reset({ fullName: updated.full_name, phone: updated.phone ?? '' })
    },
  })

  const submit = form.handleSubmit((values) => {
    setSaved(false)
    return save.mutateAsync(values)
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t.account.profileTitle}</CardTitle>
      </CardHeader>
      <CardBody>
        <form onSubmit={submit} noValidate className="flex flex-col gap-4">
          <InlineError error={save.error} />

          <Field label={t.account.fullName} error={form.formState.errors.fullName?.message} required>
            {(props) => (
              <Input
                {...props}
                {...form.register('fullName', { required: t.validation.required })}
                autoComplete="name"
              />
            )}
          </Field>

          <Field label={t.account.phone} hint={t.account.phoneHint}>
            {(props) => (
              <Input {...props} {...form.register('phone')} type="tel" autoComplete="tel" />
            )}
          </Field>

          {/* L'email si mostra e **non si modifica**, con la riga che dice perché.
              `disabled` più `readOnly` insieme: il primo la toglie dal `Tab` e la
              colora da spenta, e la spiegazione è in `hint`. */}
          <Field label={t.account.email} hint={t.account.emailLocked}>
            {(props) => (
              <Input {...props} value={account.email} disabled readOnly autoComplete="email" />
            )}
          </Field>

          <div className="flex items-center justify-end gap-3">
            {saved && !form.formState.isDirty ? (
              <p role="status" className="text-sm text-success">
                {t.account.profileSaved}
              </p>
            ) : null}
            <Button
              type="submit"
              variant="primary"
              disabled={form.formState.isSubmitting || !form.formState.isDirty}
            >
              {form.formState.isSubmitting ? t.common.saving : t.account.saveProfile}
            </Button>
          </div>
        </form>
      </CardBody>
    </Card>
  )
}

/**
 * Il cambio password, con la conseguenza **detta prima del bottone**.
 *
 * Cambiare la password revoca tutte le sessioni, questa compresa: chi lo scopre
 * dopo, ritrovandosi disconnesso sul telefono, lo legge come un guasto. La frase è
 * la stessa del recupero (`auth.resetRevokesSessions` là, `passwordRevokesSessions`
 * qui), perché la conseguenza è identica dai due lati.
 *
 * A cambio riuscito **si esce**: l'API ha già revocato i refresh token, quindi la
 * sessione è comunque condannata al primo rinnovo. Uscire subito lo rende esplicito
 * invece di lasciarlo scoprire a metà di un'altra schermata.
 */
function PasswordCard() {
  const client = useApi()
  const { store } = useSession()
  const [error, setError] = useState<unknown>(null)
  const form = useForm({
    defaultValues: { current: '', next: '' },
  })

  const submit = form.handleSubmit(async (values) => {
    setError(null)
    try {
      await changePassword(client, {
        current_password: values.current,
        new_password: values.next,
      })
      // La sessione è già condannata: i refresh token sono revocati. `logout`
      // pulisce lo stato locale e riporta all'accesso, dove il messaggio dice di
      // rientrare con la password nuova.
      await store.logout()
    } catch (failure) {
      setError(failure)
    }
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t.account.passwordTitle}</CardTitle>
      </CardHeader>
      <CardBody>
        <form onSubmit={submit} noValidate className="flex flex-col gap-4">
          <InlineError error={error} />

          {/* **Prima del form, non dopo il bottone.** */}
          <p className="rounded-lg bg-surface-muted px-3 py-2 text-sm text-ink-muted">
            {t.account.passwordRevokesSessions}
          </p>

          <Field
            label={t.account.currentPassword}
            error={form.formState.errors.current?.message}
            required
          >
            {(props) => (
              <PasswordInput
                {...props}
                {...form.register('current', { required: t.validation.required })}
                autoComplete="current-password"
              />
            )}
          </Field>

          <Field
            label={t.auth.newPassword}
            hint={t.auth.passwordHint}
            error={form.formState.errors.next?.message}
            required
          >
            {(props) => (
              <PasswordInput
                {...props}
                {...form.register('next', {
                  required: t.validation.required,
                  minLength: { value: PASSWORD_MIN_LENGTH, message: t.validation.passwordTooShort },
                })}
                autoComplete="new-password"
              />
            )}
          </Field>

          <div className="flex justify-end">
            <Button type="submit" variant="primary" disabled={form.formState.isSubmitting}>
              {form.formState.isSubmitting ? t.common.saving : t.account.changePassword}
            </Button>
          </div>
        </form>
      </CardBody>
    </Card>
  )
}

/**
 * L'uscita dalle organizzazioni, una riga per ognuna, con l'avviso **prima**.
 *
 * # La propria organizzazione non ha un bottone «esci»
 *
 * Ne è l'unico proprietario, e l'API risponderebbe `membership.last_owner`: una
 * voce che porta a un errore è peggio di una voce che manca. Si mostra comunque
 * nell'elenco, senza l'azione, così chi la cerca sa che c'è e perché non si lascia.
 *
 * # L'avviso dice che cosa resta **prima** di uscire
 *
 * `ConfirmAction` spiega che non si vedranno più contratti, addebiti e
 * segnalazioni di quell'organizzazione, e che un reinvito fa rientrare. È la stessa
 * forma dell'avviso sulla revoca delle sessioni: la conseguenza si legge prima, non
 * si scopre dopo con un riepilogo vuoto.
 */
function LeaveCard() {
  const { query } = useOrganizations()

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t.account.leaveTitle}</CardTitle>
      </CardHeader>
      <CardBody>
        <p className="mb-3 text-sm text-ink-muted">{t.account.leaveHint}</p>
        <Async query={query} isEmpty={(data) => data.items.length === 0}>
          {(data) => (
            <ul className="divide-y divide-border-subtle">
              {data.items.map((organization) => (
                <OrganizationRow key={organization.id} organization={organization} />
              ))}
            </ul>
          )}
        </Async>
      </CardBody>
    </Card>
  )
}

function OrganizationRow({ organization }: { organization: Organization }) {
  const client = useApi()
  const queryClient = useQueryClient()
  // Della propria organizzazione si è proprietario, e non se ne esce: niente
  // bottone, ma la riga resta, così si vede che c'è.
  const canLeave = organization.role !== 'owner'

  const leave = useMutation({
    mutationFn: () => leaveOrganization(client, organization.id),
    onSuccess: async () => {
      // **Si toglie dall'elenco in cache prima di rileggerlo**, e non è
      // un'ottimizzazione: se l'organizzazione lasciata era quella attiva, ogni
      // richiesta fatta da qui in avanti porterebbe il suo `X-Org-Id` e
      // riceverebbe `403 org.not_a_member` — il riallineamento di
      // `useOrganizations` parte solo quando l'elenco **non** la contiene più, e
      // aspettare la rilettura lascerebbe una finestra in cui l'applicazione
      // intera risponde `403`.
      //
      // Scriverlo a mano rende il riallineamento immediato e sincrono con la
      // risposta che lo autorizza. La rilettura resta, perché è l'elenco vero.
      queryClient.setQueryData<Items<Organization>>(keys.organizations, (previous) =>
        previous
          ? { items: previous.items.filter((item) => item.id !== organization.id) }
          : previous,
      )
      await queryClient.invalidateQueries({ queryKey: keys.organizations })
    },
  })

  return (
    <li className="flex items-center justify-between gap-3 py-3">
      <span className="min-w-0">
        <span className="block truncate text-sm font-medium text-ink">{organization.name}</span>
        <span className="block text-xs text-ink-muted">{t.org.role[organization.role]}</span>
      </span>
      {canLeave ? (
        <ConfirmAction
          trigger={
            <Button variant="dangerGhost" size="sm">
              {t.account.leave}
            </Button>
          }
          title={t.account.leaveConfirmTitle.replace('{org}', organization.name)}
          explanation={t.account.leaveConfirmBody}
          confirmLabel={t.account.leaveConfirm}
          onConfirm={() => leave.mutateAsync()}
        />
      ) : (
        <span className="text-xs text-ink-muted">{t.account.leaveLastOwnerShort}</span>
      )}
    </li>
  )
}

/**
 * Un campo password con l'occhio, locale a questa schermata.
 *
 * Non si importa quello di `AuthPage`: tirarlo dentro porterebbe l'intera pagina di
 * accesso nel bundle di questa, che è pigra e a sé. Sono dieci righe, e la
 * duplicazione costa meno del legame.
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
