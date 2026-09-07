import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Plus } from 'lucide-react'
import { useState } from 'react'

import { recordExpense } from '@/api/endpoints'
import type { ExpenseCategory, ExpenseGroup, SplitMethod } from '@/api/types'
import { InlineError } from '@/components/state'
import { Button } from '@/components/ui/button'
import { FormDialog } from '@/components/ui/confirm'
import { Field, Input, Select, Textarea } from '@/components/ui/field'
import { keys } from '@/app/query'
import { useApi, useSession } from '@/app/session'
import { t } from '@/i18n'
import { isPositive, parseDecimal } from '@/lib/decimal'
import { useMemberNames } from './names'

const CATEGORIES: readonly ExpenseCategory[] = [
  'groceries',
  'utilities',
  'cleaning',
  'connectivity',
  'maintenance',
  'furniture',
  'rent',
  'other',
]

/**
 * Il menù della ripartizione ha **due** voci, scritte a mano.
 *
 * `percentage`, `exact` e `by_usage` compaiono nell'enum e l'API li rifiuta con
 * `422`: costruire il menù dall'unione dei valori offrirebbe tre metodi che danno
 * errore. È la stessa ragione per cui i filtri di stato delle segnalazioni si
 * scrivono a mano invece di derivarli dal tipo.
 */
const SPLIT_METHODS: readonly Extract<SplitMethod, 'equal' | 'weighted'>[] = ['equal', 'weighted']

interface Values {
  description: string
  amount: string
  category: ExpenseCategory
  paid_by_user_id: string
  expense_date: string
  split_method: 'equal' | 'weighted'
  notes: string
}

/**
 * Registrare una spesa, che è il gesto da cui i saldi esistono.
 *
 * # La ripartizione si sceglie **qui**, non sul gruppo
 *
 * `default_split_method` è una consuetudine e non una regola — il DTO lo dice, e
 * ogni spesa può dichiararne un'altra: la bombola del gas di chi ha la stufa
 * esiste. Il form propone quella del gruppo e la lascia cambiare.
 *
 * # Chi ha anticipato dev'essere un membro **attivo**
 *
 * A chi non lo è l'API risponde `409 expense.payer_not_in_group`, e il danno che
 * quel controllo evita è silenzioso: la vista dei saldi parte dai membri, quindi
 * un credito intestato a un estraneo non comparirebbe a nessuno e la somma dei
 * netti smetterebbe di fare zero. Il menù si costruisce quindi da chi ha `left_at`
 * nullo — chi è uscito resta nei saldi e non in questo elenco.
 *
 * # Le quote non si calcolano qui
 *
 * Le fa il server, e la loro somma è **esattamente** l'importo: lo garantisce un
 * trigger, e il centesimo di resto va ai primi in ordine di identificatore. Il
 * client le legge dal dettaglio della spesa e non le ricalcola.
 *
 * # Nessuna chiave di idempotenza
 *
 * Questa rotta non la richiede, a differenza delle due che creano un pagamento.
 * Non è una svista dell'API, ed è da non replicare per simmetria.
 */
export function RecordExpenseDialog({ group }: { group: ExpenseGroup }) {
  const client = useApi()
  const queryClient = useQueryClient()
  const { session } = useSession()
  const nameOf = useMemberNames(group.members, group.property_id)

  const [open, setOpen] = useState(false)
  const [submitted, setSubmitted] = useState(false)
  const [values, setValues] = useState<Values>(() => initial(group, session?.userId))

  const active = (group.members ?? []).filter((member) => !member.left_at)

  const mutation = useMutation({
    mutationFn: () =>
      recordExpense(client, group.id, {
        description: values.description.trim(),
        amount: values.amount.trim().replace(',', '.'),
        category: values.category,
        split_method: values.split_method,
        // Assente significa «io»: si manda solo quando si sceglie qualcun altro.
        ...(values.paid_by_user_id && values.paid_by_user_id !== session?.userId
          ? { paid_by_user_id: values.paid_by_user_id }
          : {}),
        // Assente significa «oggi». È una **data** e non un istante: nessun fuso
        // da cui sbagliare di un giorno, e il valore del campo è già ISO.
        ...(values.expense_date ? { expense_date: values.expense_date } : {}),
        ...(values.notes.trim() ? { notes: values.notes.trim() } : {}),
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.expenses(group.id) })
      await queryClient.invalidateQueries({ queryKey: keys.groupBalances(group.id) })
      // Il netto complessivo in cima a `/spese` legge i saldi di ogni gruppo.
      await queryClient.invalidateQueries({ queryKey: ['group-balances'] })
      setOpen(false)
      setValues(initial(group, session?.userId))
      setSubmitted(false)
    },
  })

  const descriptionError =
    submitted && !values.description.trim() ? t.validation.required : undefined
  const amountError = amountProblem(values.amount, submitted)

  return (
    <FormDialog
      trigger={
        <Button variant="primary">
          <Plus aria-hidden />
          {t.expense.record}
        </Button>
      }
      title={t.expense.recordTitle}
      description={t.expense.recordExplain}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) {
          setValues(initial(group, session?.userId))
          setSubmitted(false)
          mutation.reset()
        }
      }}
    >
      <form
        noValidate
        className="flex flex-col gap-4"
        onSubmit={(event) => {
          event.preventDefault()
          setSubmitted(true)
          if (!values.description.trim() || amountProblem(values.amount, true)) return
          mutation.mutate()
        }}
      >
        <InlineError error={mutation.error} />

        <Field label={t.expense.description} error={descriptionError} required>
          {(props) => (
            <Input
              {...props}
              autoFocus
              value={values.description}
              onChange={(event) => setValues((v) => ({ ...v, description: event.target.value }))}
            />
          )}
        </Field>

        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <Field label={t.expense.amount} error={amountError} required>
            {(props) => (
              <Input
                {...props}
                inputMode="decimal"
                placeholder="100.00"
                value={values.amount}
                onChange={(event) => setValues((v) => ({ ...v, amount: event.target.value }))}
              />
            )}
          </Field>

          <Field label={t.expense.category}>
            {(props) => (
              <Select
                {...props}
                value={values.category}
                onChange={(event) =>
                  setValues((v) => ({ ...v, category: event.target.value as ExpenseCategory }))
                }
              >
                {CATEGORIES.map((category) => (
                  <option key={category} value={category}>
                    {t.expense.categories[category]}
                  </option>
                ))}
              </Select>
            )}
          </Field>
        </div>

        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <Field label={t.expense.paidBy} hint={t.expense.paidByHint}>
            {(props) => (
              <Select
                {...props}
                value={values.paid_by_user_id}
                onChange={(event) =>
                  setValues((v) => ({ ...v, paid_by_user_id: event.target.value }))
                }
              >
                {active.map((member) => (
                  <option key={member.user_id} value={member.user_id}>
                    {member.user_id === session?.userId
                      ? t.expense.paidByMe
                      : nameOf(member.user_id)}
                  </option>
                ))}
              </Select>
            )}
          </Field>

          <Field label={t.expense.expenseDate} hint={t.expense.expenseDateHint}>
            {(props) => (
              <Input
                {...props}
                type="date"
                value={values.expense_date}
                onChange={(event) => setValues((v) => ({ ...v, expense_date: event.target.value }))}
              />
            )}
          </Field>
        </div>

        <Field label={t.expense.splitMethod} hint={t.expense.defaultSplitHint}>
          {(props) => (
            <Select
              {...props}
              value={values.split_method}
              onChange={(event) =>
                setValues((v) => ({
                  ...v,
                  split_method: event.target.value as 'equal' | 'weighted',
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

        <Field label={t.expense.notes}>
          {(props) => (
            <Textarea
              {...props}
              value={values.notes}
              onChange={(event) => setValues((v) => ({ ...v, notes: event.target.value }))}
            />
          )}
        </Field>

        <p className="text-xs text-ink-muted">{t.expense.sharesSum}</p>

        <div className="flex justify-end gap-2">
          <Button type="button" onClick={() => setOpen(false)} disabled={mutation.isPending}>
            {t.common.cancel}
          </Button>
          <Button type="submit" variant="primary" disabled={mutation.isPending}>
            {mutation.isPending ? t.common.saving : t.expense.record}
          </Button>
        </div>
      </form>
    </FormDialog>
  )
}

/**
 * Il metodo proposto è quello del gruppo, **se è uno dei due che l'API accetta**.
 *
 * `default_split_method` può portare uno dei tre valori rifiutati con `422` — lo
 * schema li ammette anche se l'API no — e proporlo produrrebbe un form che
 * fallisce senza che si sia toccato niente.
 *
 * **Chi ha anticipato parte da chi guarda**, e non da un valore vuoto: una
 * `<select>` il cui valore non corrisponde a nessuna opzione mostra la prima e ne
 * invia un'altra, cioè il modo più silenzioso di attribuire una spesa alla persona
 * sbagliata. Chi guarda e non è un membro attivo — un proprietario, per esempio —
 * parte dal primo dell'elenco per la stessa ragione.
 */
function initial(group: ExpenseGroup, userId: string | undefined): Values {
  const proposed =
    group.default_split_method === 'weighted' || group.default_split_method === 'equal'
      ? group.default_split_method
      : 'equal'
  const active = (group.members ?? []).filter((member) => !member.left_at)
  const payer =
    userId && active.some((member) => member.user_id === userId)
      ? userId
      : (active[0]?.user_id ?? '')
  return {
    description: '',
    amount: '',
    category: 'groceries',
    paid_by_user_id: payer,
    expense_date: '',
    split_method: proposed,
    notes: '',
  }
}

/**
 * L'importo, validato **senza passare da un float**: `parseFloat` è vietato.
 *
 * L'errore compare dopo il primo invio e non a ogni tasto — segnalare un campo
 * incompleto mentre lo si sta scrivendo è rumore.
 */
function amountProblem(raw: string, submitted: boolean): string | undefined {
  if (!submitted) return undefined
  const value = raw.trim().replace(',', '.')
  if (!value) return t.validation.required
  if (!parseDecimal(value)) return t.validation.amount
  if (!isPositive(value)) return t.validation.positiveAmount
  return undefined
}
