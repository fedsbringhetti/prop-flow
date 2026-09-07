import { useMutation, useQueryClient } from '@tanstack/react-query'
import { HandCoins } from 'lucide-react'
import { useState } from 'react'

import { declareSettlement } from '@/api/endpoints'
import type { ExpenseGroup, GroupBalance, PaymentMethod } from '@/api/types'
import { InlineError } from '@/components/state'
import { Button } from '@/components/ui/button'
import { FormDialog } from '@/components/ui/confirm'
import { Field, Input, Select } from '@/components/ui/field'
import { keys } from '@/app/query'
import { useApi } from '@/app/session'
import { t } from '@/i18n'
import { absDecimal, compareDecimals, isPositive, parseDecimal } from '@/lib/decimal'
import { formatAmount } from '@/lib/format'
import type { NameOf } from './names'

/** L'ordine è quello del menù dei pagamenti: il bonifico per primo. */
const METHODS: readonly PaymentMethod[] = [
  'bank_transfer',
  'sepa_direct_debit',
  'card',
  'cash',
  'check',
  'other',
]

interface Values {
  to_user_id: string
  amount: string
  method: PaymentMethod
  settled_at: string
  note: string
}

/**
 * Dichiarare di aver rimborsato un coinquilino.
 *
 * # Il corpo **non ha `from_user_id`**
 *
 * Chi dichiara è sempre chi chiama: `settlements_insert` lo esige a chi non è
 * staff, e passarlo produrrebbe zero righe inserite — `auth.denied_by_policy`, il
 * codice riservato ai nostri difetti.
 *
 * # A chi si paga si sceglie fra i **creditori**, e l'importo è un suggerimento
 *
 * L'API dà i netti per persona e **non** una minimizzazione dei trasferimenti. Il
 * client non la calcola nemmeno: due client con euristiche diverse proporrebbero
 * giri di bonifici diversi sugli stessi saldi, e il prodotto direbbe due cose. Il
 * campo propone il minimo fra il proprio debito e il credito di quella persona —
 * un suggerimento visibile e correggibile, non un algoritmo nascosto.
 *
 * # Il ritiro **esiste**, ed è il contrario dei pagamenti
 *
 * `settlements_update` nomina anche `from_user_id`, mentre `payments_update` è
 * staff soltanto: fra pari il ritiro è lecito, verso il proprietario no. Chi ha
 * letto l'altra schermata — dove il form avverte che non si può ritirare — si
 * aspetta il contrario, ed è la ragione per cui la nota qui lo dice nell'altro
 * verso.
 *
 * # Nessuna chiave di idempotenza
 *
 * `POST /expense-groups/{id}/settlements` non la richiede, a differenza delle due
 * rotte che creano un pagamento. Non è una svista dell'API ed è da non replicare
 * per simmetria.
 */
export function DeclareSettlementDialog({
  group,
  balances,
  nameOf,
  userId,
}: {
  group: ExpenseGroup
  balances: readonly GroupBalance[]
  nameOf: NameOf
  userId: string | null
}) {
  const client = useApi()
  const queryClient = useQueryClient()

  // I creditori: `net_balance` positivo vuol dire che il gruppo gli deve. Arrivano
  // già per primi dall'API, e il client non li riordina.
  const creditors = balances.filter(
    (balance) => balance.user_id !== userId && isPositive(balance.net_balance),
  )
  const mine = balances.find((balance) => balance.user_id === userId)
  const myDebt = mine && !isPositive(mine.net_balance) ? absDecimal(mine.net_balance) : '0.00'

  const [open, setOpen] = useState(false)
  const [submitted, setSubmitted] = useState(false)
  const [values, setValues] = useState<Values>(() => ({
    to_user_id: '',
    amount: '',
    method: 'bank_transfer',
    settled_at: '',
    note: '',
  }))

  const mutation = useMutation({
    mutationFn: () =>
      declareSettlement(client, group.id, {
        to_user_id: values.to_user_id,
        amount: values.amount.trim().replace(',', '.'),
        method: values.method,
        // Assente significa «adesso». `settled_at` è un istante, quindi qui la
        // data del campo diventa un istante nel fuso di chi guarda.
        ...(values.settled_at ? { settled_at: new Date(values.settled_at).toISOString() } : {}),
        ...(values.note.trim() ? { note: values.note.trim() } : {}),
      }),
    onSuccess: async () => {
      // Nasce `pending` e **non muove i saldi**: si rilegge comunque l'elenco, e i
      // saldi perché la schermata li mostra accanto — chi guarda deve vedere che
      // non si sono mossi, non doverlo dedurre.
      await queryClient.invalidateQueries({ queryKey: keys.settlements(group.id) })
      await queryClient.invalidateQueries({ queryKey: keys.groupBalances(group.id) })
      setOpen(false)
      setValues({ to_user_id: '', amount: '', method: 'bank_transfer', settled_at: '', note: '' })
      setSubmitted(false)
    },
  })

  function chooseCreditor(creditorId: string) {
    const creditor = creditors.find((balance) => balance.user_id === creditorId)
    setValues((current) => ({
      ...current,
      to_user_id: creditorId,
      // Il minimo fra quello che devo e quello che gli devono: sopra a quella
      // cifra il rimborso creerebbe un credito nuovo al posto di chiuderne uno.
      amount: creditor ? suggested(myDebt, creditor.net_balance) : current.amount,
    }))
  }

  const toError = submitted && !values.to_user_id ? t.validation.required : undefined
  const amountError = amountProblem(values.amount, submitted)

  return (
    <FormDialog
      trigger={
        <Button>
          <HandCoins aria-hidden />
          {t.expense.declare}
        </Button>
      }
      title={t.expense.declareTitle}
      description={t.expense.declareExplain}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) {
          setValues({ to_user_id: '', amount: '', method: 'bank_transfer', settled_at: '', note: '' })
          setSubmitted(false)
          mutation.reset()
        }
      }}
    >
      {creditors.length === 0 ? (
        <p className="rounded-lg bg-surface-muted px-3 py-2 text-sm text-ink-muted">
          {t.expense.declareNoCreditors}
        </p>
      ) : (
        <form
          noValidate
          className="flex flex-col gap-4"
          onSubmit={(event) => {
            event.preventDefault()
            setSubmitted(true)
            if (!values.to_user_id || amountProblem(values.amount, true)) return
            mutation.mutate()
          }}
        >
          <InlineError error={mutation.error} />

          <Field label={t.expense.declareTo} hint={t.expense.declareToHint} error={toError} required>
            {(props) => (
              <Select
                {...props}
                autoFocus
                value={values.to_user_id}
                onChange={(event) => chooseCreditor(event.target.value)}
              >
                <option value="">{t.common.none}</option>
                {creditors.map((creditor) => (
                  <option key={creditor.user_id} value={creditor.user_id}>
                    {nameOf(creditor.user_id)} —{' '}
                    {formatAmount(creditor.net_balance, { currency: group.currency })}{' '}
                    {t.expense.inCredit}
                  </option>
                ))}
              </Select>
            )}
          </Field>

          <Field label={t.expense.declareAmount} error={amountError} required>
            {(props) => (
              <Input
                {...props}
                inputMode="decimal"
                value={values.amount}
                onChange={(event) => setValues((v) => ({ ...v, amount: event.target.value }))}
              />
            )}
          </Field>

          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <Field label={t.payment.method}>
              {(props) => (
                <Select
                  {...props}
                  value={values.method}
                  onChange={(event) =>
                    setValues((v) => ({ ...v, method: event.target.value as PaymentMethod }))
                  }
                >
                  {METHODS.map((method) => (
                    <option key={method} value={method}>
                      {t.payment.methods[method]}
                    </option>
                  ))}
                </Select>
              )}
            </Field>

            <Field label={t.expense.settledAt} hint={t.expense.settledAtHint}>
              {(props) => (
                <Input
                  {...props}
                  type="date"
                  value={values.settled_at}
                  onChange={(event) => setValues((v) => ({ ...v, settled_at: event.target.value }))}
                />
              )}
            </Field>
          </div>

          <Field label={t.expense.note} hint={t.expense.noteHint}>
            {(props) => (
              <Input
                {...props}
                value={values.note}
                onChange={(event) => setValues((v) => ({ ...v, note: event.target.value }))}
              />
            )}
          </Field>

          <p className="rounded-lg bg-surface-muted px-3 py-2 text-xs text-ink-muted">
            {t.expense.pendingDoesNotMove} {t.expense.declareCanWithdraw}
          </p>

          <div className="flex justify-end gap-2">
            <Button type="button" onClick={() => setOpen(false)} disabled={mutation.isPending}>
              {t.common.cancel}
            </Button>
            <Button type="submit" variant="primary" disabled={mutation.isPending}>
              {mutation.isPending ? t.common.saving : t.expense.declare}
            </Button>
          </div>
        </form>
      )}
    </FormDialog>
  )
}

/**
 * Il minimo fra il proprio debito e il credito di chi si sta pagando, **confrontato
 * sugli interi**.
 *
 * `parseFloat` è vietato, e qui sbagliare di un centesimo si vedrebbe: il campo è
 * il numero che una persona sta per bonificare a un'altra.
 */
function suggested(debt: string, credit: string): string {
  return compareDecimals(debt, credit) <= 0 ? debt : credit
}

/** L'importo, senza float e con l'errore solo dopo il primo invio. */
function amountProblem(raw: string, submitted: boolean): string | undefined {
  if (!submitted) return undefined
  const value = raw.trim().replace(',', '.')
  if (!value) return t.validation.required
  if (!parseDecimal(value)) return t.validation.amount
  if (!isPositive(value)) return t.validation.positiveAmount
  return undefined
}
