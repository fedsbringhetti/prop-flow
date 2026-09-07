import { useQueries, useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus } from 'lucide-react'
import { useState } from 'react'
import { Link } from 'react-router'

import {
  createExpenseGroup,
  getGroupBalances,
  listExpenseGroups,
  listProperties,
} from '@/api/endpoints'
import type { ExpenseGroup, ExpenseGroupKind, GroupBalance, SplitMethod } from '@/api/types'
import { Async, Empty, ErrorState, InlineError, Loading } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Card, CardBody } from '@/components/ui/card'
import { FormDialog } from '@/components/ui/confirm'
import { Field, Input, Select } from '@/components/ui/field'
import { PageHeader, PageSection } from '@/components/ui/page'
import { keys } from '@/app/query'
import { useApi, useSession } from '@/app/session'
import { t } from '@/i18n'
import { absDecimal, isPositive, isZero, sumAmounts } from '@/lib/decimal'
import { formatAmount } from '@/lib/format'
import { LoadMore, usePagedQuery } from '../shared/pagination'
import { NetAmount } from './balance'
import { ownName, useResidents } from './names'

/** Il menù ha **due** voci: gli altri tre valori dell'enum danno `422`. */
const SPLIT_METHODS: readonly Extract<SplitMethod, 'equal' | 'weighted'>[] = ['equal', 'weighted']
const KINDS: readonly ExpenseGroupKind[] = ['coliving', 'household', 'property']

/**
 * I gruppi di spesa, e in cima **il proprio netto complessivo**.
 *
 * # Come si scopre di dovere qualcosa a un coinquilino
 *
 * `/addebiti` risponde a «quanto devo al proprietario». Il debito verso chi abita
 * con te è un'altra tasca, e prima di questa schermata nessuna pagina lo
 * annunciava: si sarebbe scoperto solo entrando. L'indicatore è il netto
 * complessivo qui in cima.
 *
 * Costa `1 + N` richieste — l'elenco dei gruppi, più i saldi di ognuno — e con uno
 * o due gruppi è niente. **Non è un contatore sulla voce di navigazione**: quello
 * si vedrebbe da ogni schermata, e per calcolarlo servirebbe la stessa `1 + N` a
 * ogni caricamento del guscio, perché non esiste una rotta che dia il netto
 * complessivo di una persona.
 *
 * # Il totale si mostra solo quando è **tutto** letto
 *
 * Un netto calcolato su una parte dei gruppi non è un'approssimazione, è un numero
 * sbagliato che sembra giusto. Finché una delle letture è in volo si dice che si
 * sta leggendo; se una fallisce si dice quello, e non si somma ciò che è arrivato.
 *
 * # L'elenco non mostra chi c'è dentro
 *
 * `GET /expense-groups` risponde con `members` assente — il dettaglio no — quindi
 * una colonna «con chi» mostrerebbe `undefined`. Al suo posto c'è il **proprio**
 * saldo in quel gruppo, che i saldi già letti per il totale danno senza costare
 * una richiesta in più.
 */
export function ExpenseGroupsPage() {
  const client = useApi()
  const { session } = useSession()

  // Nessun filtro sull'archiviazione: un gruppo archiviato può avere ancora dei
  // conti aperti — si chiude apposta anche così — e nasconderlo toglierebbe dal
  // totale un debito che resta dovuto.
  const groups = usePagedQuery({
    queryKey: keys.expenseGroups(),
    fetchPage: (after) => listExpenseGroups(client, { after }),
  })

  const items = groups.data ?? []

  const balances = useQueries({
    queries: items.map((group) => ({
      queryKey: keys.groupBalances(group.id),
      queryFn: () => getGroupBalances(client, group.id),
    })),
  })

  // Una forma sola per i due usi — il totale in cima e la riga di ogni gruppo —
  // perché sono la **stessa** lettura: la colonna del proprio saldo non costa una
  // richiesta in più, usa quelle che il totale ha già fatto.
  const slices: BalanceSlice[] = balances.map((balance) => ({
    isPending: balance.isPending,
    isError: balance.isError,
    items: balance.data?.items,
  }))

  return (
    <>
      <PageHeader
        title={t.expense.title}
        description={t.expense.subtitle}
        actions={<NewGroupDialog />}
      />

      {items.length > 0 ? (
        <PageSection>
          <NetBalance
            userId={session?.userId ?? null}
            balances={slices}
          />
        </PageSection>
      ) : null}

      <Card>
        <Async
          query={groups}
          isEmpty={(rows) => rows.length === 0}
          empty={<Empty title={t.expense.groupsEmpty} hint={t.expense.groupsEmptyHint} />}
        >
          {(rows) => (
            /* Un elenco e non una tabella: le colonne sarebbero tre, e la
               larghezza minima che una tabella tiene per reggere gli elenchi del
               proprietario spingerebbe **il saldo** fuori dallo schermo a 375 px —
               cioè il numero per cui si apre questa pagina. Con due fatti per riga
               un elenco li tiene accanto a qualunque larghezza. */
            <ul className="divide-y divide-border-subtle">
              {rows.map((group, index) => (
                <GroupRow
                  key={group.id}
                  group={group}
                  userId={session?.userId ?? null}
                  balance={slices[index]}
                />
              ))}
            </ul>
          )}
        </Async>

        <LoadMore
          hasNextPage={groups.hasNextPage}
          isFetchingNextPage={groups.isFetchingNextPage}
          fetchNextPage={() => void groups.fetchNextPage()}
        />
      </Card>
    </>
  )
}

interface BalanceSlice {
  isPending: boolean
  isError: boolean
  items: readonly GroupBalance[] | undefined
}

/**
 * Il netto complessivo, sommato **con il modulo decimale**.
 *
 * `parseFloat` è vietato: sommare `"184.20"` come binario dà un totale che a volte
 * non è il totale, e qui il numero decide se una persona crede di dovere qualcosa.
 */
function NetBalance({ userId, balances }: { userId: string | null; balances: BalanceSlice[] }) {
  if (balances.some((balance) => balance.isPending)) {
    return (
      <Card>
        <Loading label={t.expense.netLoading} />
      </Card>
    )
  }

  if (balances.some((balance) => balance.isError) || !userId) {
    return (
      <Card>
        <CardBody>
          <p role="alert" className="text-sm text-danger">
            {t.expense.netFailed}
          </p>
        </CardBody>
      </Card>
    )
  }

  const mine = balances.flatMap((balance) =>
    (balance.items ?? [])
      .filter((entry) => entry.user_id === userId)
      .map((entry) => entry.net_balance),
  )
  const net = sumAmounts(mine)

  return (
    <Card>
      <CardBody className="flex flex-wrap items-baseline justify-between gap-x-6 gap-y-2">
        <div className="min-w-0">
          <p className="text-xs font-medium tracking-wide text-ink-muted uppercase">
            {t.expense.netTitle}
          </p>
          <p className="mt-1 text-sm text-ink-muted">{t.expense.netHint}</p>
        </div>
        <p className="tabular text-lg font-semibold">
          {isZero(net) ? (
            <span className="text-ink-muted">{t.expense.netEven}</span>
          ) : (
            <>
              {/* La parola dice il verso, quindi il numero si mostra **senza il
                  segno**: «Devi −61,40 €» è un doppio negativo che fa dubitare di
                  chi deve a chi. Nelle tabelle il segno resta, perché lì è la
                  colonna a portare l'informazione e non una frase. */}
              <span className="mr-2 text-sm font-normal text-ink-muted">
                {isPositive(net) ? t.expense.netCredit : t.expense.netDebt}
              </span>
              <NetAmount value={net} display={absDecimal(net)} />
            </>
          )}
        </p>
      </CardBody>
    </Card>
  )
}

function GroupRow({
  group,
  userId,
  balance,
}: {
  group: ExpenseGroup
  userId: string | null
  balance: BalanceSlice | undefined
}) {
  const mine = balance?.items?.find((entry) => entry.user_id === userId)

  return (
    <li className="flex flex-wrap items-center justify-between gap-x-4 gap-y-1 px-4 py-3">
      <span className="min-w-0">
        <Link
          to={`/spese/${group.id}`}
          className="font-medium text-accent underline-offset-4 hover:underline"
        >
          {group.name}
        </Link>
        <span className="mt-0.5 flex items-center gap-2 text-xs text-ink-muted">
          {t.expense.kinds[group.kind]}
          {/* Un gruppo archiviato resta nell'elenco: il conto è chiuso alle
              scritture, non ai conti aperti, e toglierlo di qui toglierebbe dal
              totale in cima un debito che resta dovuto. */}
          {group.is_archived ? <Badge tone="muted">{t.expense.archived}</Badge> : null}
        </span>
      </span>
      <span className="tabular text-sm font-medium">
        {balance?.isPending ? (
          <span className="text-xs font-normal text-ink-muted">{t.state.loadingTitle}</span>
        ) : balance?.isError || !mine ? (
          // Nessuna riga per sé stessi vuol dire netto zero: la vista non elenca
          // chi è uscito in pari. Il trattino resta per la lettura fallita, che è
          // un'altra cosa e non va confusa con uno zero.
          <span className="font-normal text-ink-muted">
            {balance?.isError ? t.common.none : formatAmount('0.00', { currency: group.currency })}
          </span>
        ) : (
          <NetAmount value={mine.net_balance} currency={group.currency} />
        )}
      </span>
    </li>
  )
}

interface NewGroupValues {
  name: string
  kind: ExpenseGroupKind
  property_id: string
  default_split_method: 'equal' | 'weighted'
  /** `null` finché non lo si è toccato: allora vale quello proposto dai residenti. */
  display_name: string | null
  weight: string
}

/**
 * Creare un gruppo, che è una modale e non una schermata: un gruppo ha un nome e
 * poco altro — come le stanze, e non come gli immobili.
 *
 * # L'immobile si chiede sempre, anche se l'API lo accetta assente
 *
 * `property_id` è facoltativo, ma senza non c'è un elenco di residenti da cui
 * pescare i membri, e un gruppo di una persona sola non serve a niente. Chi crea
 * un gruppo dall'interfaccia lo crea con l'immobile, perché è il caso reale; i
 * gruppi senza immobile esistono e le schermate li reggono, ma non nascono da qui.
 *
 * # Il proprio nome si scrive **adesso**, perché dopo non si recupera
 *
 * È l'unico momento in cui il client può dare un nome alla propria membership, ed
 * è l'unico posto in cui quel nome vivrà: i saldi, le quote e i rimborsi
 * identificano le persone solo con `user_id`. Il campo è precompilato con il
 * proprio `full_name` letto dai residenti dell'immobile scelto, e resta
 * modificabile — chi non compare fra i residenti di casa propria (un proprietario
 * che ci abita, per esempio) lo scrive a mano invece di restare senza nome.
 */
function NewGroupDialog() {
  const client = useApi()
  const queryClient = useQueryClient()
  const { session } = useSession()

  const [open, setOpen] = useState(false)
  const [submitted, setSubmitted] = useState(false)
  const [error, setError] = useState<unknown>(null)
  const [pending, setPending] = useState(false)
  const [values, setValues] = useState<NewGroupValues>({
    name: '',
    kind: 'coliving',
    property_id: '',
    default_split_method: 'equal',
    display_name: null,
    weight: '',
  })

  const properties = useQuery({
    queryKey: keys.properties({ perGruppo: true }),
    queryFn: () => listProperties(client),
    enabled: open,
  })

  const residents = useResidents(values.property_id || undefined)

  // Il nome proposto **si deriva**, non si scrive nello stato da un effetto:
  // arriva dopo la scelta dell'immobile, quindi non può essere un valore
  // iniziale, e un effetto che lo scrivesse lo sovrascriverebbe addosso a chi sta
  // digitando. `null` significa «non l'ho toccato»: da lì in poi vince ciò che è
  // stato scritto, anche se è una stringa vuota.
  const displayName = values.display_name ?? ownName(session?.userId, residents.data?.items)

  function reset() {
    setValues({
      name: '',
      kind: 'coliving',
      property_id: '',
      default_split_method: 'equal',
      display_name: null,
      weight: '',
    })
    setSubmitted(false)
    setError(null)
  }

  const nameError = submitted && !values.name.trim() ? t.validation.required : undefined
  const propertyError = submitted && !values.property_id ? t.validation.required : undefined
  const displayNameError = submitted && !displayName.trim() ? t.validation.required : undefined

  async function submit() {
    setSubmitted(true)
    if (!values.name.trim() || !values.property_id || !displayName.trim()) return
    setPending(true)
    setError(null)
    try {
      const group = await createExpenseGroup(client, {
        name: values.name.trim(),
        kind: values.kind,
        property_id: values.property_id,
        default_split_method: values.default_split_method,
        display_name: displayName.trim(),
        ...(values.weight.trim() ? { weight: values.weight.trim().replace(',', '.') } : {}),
      })
      await queryClient.invalidateQueries({ queryKey: ['expense-groups'] })
      await queryClient.invalidateQueries({ queryKey: keys.expenseGroup(group.id) })
      setOpen(false)
      reset()
    } catch (failure) {
      setError(failure)
    } finally {
      setPending(false)
    }
  }

  return (
    <FormDialog
      trigger={
        <Button variant="primary">
          <Plus aria-hidden />
          {t.expense.create}
        </Button>
      }
      title={t.expense.createTitle}
      description={t.expense.createExplain}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        // Il form non perde ciò che è stato digitato su un errore: si azzera solo
        // quando la modale si chiude.
        if (!next) reset()
      }}
    >
      <form
        noValidate
        className="flex flex-col gap-4"
        onSubmit={(event) => {
          event.preventDefault()
          void submit()
        }}
      >
        <InlineError error={error} />

        <Field label={t.expense.name} hint={t.expense.nameHint} error={nameError} required>
          {(props) => (
            <Input
              {...props}
              autoFocus
              value={values.name}
              onChange={(event) => setValues((v) => ({ ...v, name: event.target.value }))}
            />
          )}
        </Field>

        {properties.isPending ? (
          <Loading />
        ) : properties.error ? (
          <ErrorState error={properties.error} onRetry={() => void properties.refetch()} />
        ) : properties.data && properties.data.items.length === 0 ? (
          <p className="rounded-lg bg-surface-muted px-3 py-2 text-sm text-ink-muted">
            {t.expense.propertiesEmpty}
          </p>
        ) : (
          <Field
            label={t.expense.property}
            hint={t.expense.propertyHint}
            error={propertyError}
            required
          >
            {(props) => (
              <Select
                {...props}
                value={values.property_id}
                onChange={(event) => setValues((v) => ({ ...v, property_id: event.target.value }))}
              >
                <option value="">{t.common.none}</option>
                {(properties.data?.items ?? []).map((property) => (
                  <option key={property.id} value={property.id}>
                    {property.label}
                  </option>
                ))}
              </Select>
            )}
          </Field>
        )}

        <Field
          label={t.expense.displayName}
          hint={t.expense.displayNameHint}
          error={displayNameError}
          required
        >
          {(props) => (
            <Input
              {...props}
              value={displayName}
              onChange={(event) => setValues((v) => ({ ...v, display_name: event.target.value }))}
            />
          )}
        </Field>

        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <Field label={t.expense.kind}>
            {(props) => (
              <Select
                {...props}
                value={values.kind}
                onChange={(event) =>
                  setValues((v) => ({ ...v, kind: event.target.value as ExpenseGroupKind }))
                }
              >
                {KINDS.map((kind) => (
                  <option key={kind} value={kind}>
                    {t.expense.kinds[kind]}
                  </option>
                ))}
              </Select>
            )}
          </Field>

          <Field label={t.expense.defaultSplit} hint={t.expense.defaultSplitHint}>
            {(props) => (
              <Select
                {...props}
                value={values.default_split_method}
                onChange={(event) =>
                  setValues((v) => ({
                    ...v,
                    default_split_method: event.target.value as 'equal' | 'weighted',
                  }))
                }
              >
                {SPLIT_METHODS.map((method) => (
                  <option key={method} value={method}>
                    {t.expense.splitMethods[method]}
                  </option>
                ))}
              </Select>
            )}
          </Field>
        </div>

        <Field label={t.expense.weight} hint={t.expense.weightHint}>
          {(props) => (
            <Input
              {...props}
              inputMode="decimal"
              placeholder="1"
              value={values.weight}
              onChange={(event) => setValues((v) => ({ ...v, weight: event.target.value }))}
            />
          )}
        </Field>

        <div className="flex justify-end gap-2">
          <Button type="button" onClick={() => setOpen(false)} disabled={pending}>
            {t.common.cancel}
          </Button>
          <Button type="submit" variant="primary" disabled={pending}>
            {pending ? t.common.saving : t.common.create}
          </Button>
        </div>
      </form>
    </FormDialog>
  )
}
