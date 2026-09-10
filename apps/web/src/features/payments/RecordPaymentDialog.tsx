import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Banknote } from 'lucide-react'
import { useState } from 'react'

import { getLease, recordPayment } from '@/api/endpoints'
import { useIntentKey } from '@/api/idempotency'
import type { Charge, PaymentMethod } from '@/api/types'
import { InlineError } from '@/components/state'
import { Button } from '@/components/ui/button'
import { FormDialog } from '@/components/ui/confirm'
import { Field, Input, Select } from '@/components/ui/field'
import { keys } from '@/app/query'
import { useApi } from '@/app/session'
import { t } from '@/i18n'
import { isPositive, parseDecimal } from '@/lib/decimal'
import { formatAmount, todayIso } from '@/lib/format'
import { paidAtBody } from './paidAt'

/** L'ordine è quello del menù: il bonifico per primo perché è il caso comune. */
const METHODS: readonly PaymentMethod[] = [
  'bank_transfer',
  'sepa_direct_debit',
  'card',
  'cash',
  'check',
  'other',
] as const

/** Il valore della `<option>` «non attribuito»: una stringa vuota è indistinguibile da «non ho scelto». */
const UNATTRIBUTED = 'non-attribuito'

interface Values {
  amount: string
  method: PaymentMethod
  paid_at: string
  reference: string
  payer: string
}

function emptyValues(): Values {
  return {
    amount: '',
    method: 'bank_transfer',
    paid_at: todayIso(),
    reference: '',
    payer: '',
  }
}

/**
 * Registrare un incasso già avvenuto.
 *
 * # La chiave di idempotenza si genera **all'intento**
 *
 * L'intento è l'apertura del form, non la chiamata: [`useIntentKey`] la crea al
 * primo tentativo e la riusa a ogni successivo — il secondo clic sul bottone, il
 * rinvio dopo un errore di rete. Si rinnova solo dopo un invio **riuscito** e alla
 * chiusura, cioè quando l'intento è davvero un altro.
 *
 * Generarla dentro `mutationFn` la rigenererebbe a ogni tentativo, e un doppio
 * invio produrrebbe **due pagamenti**: il saldo scenderebbe due volte e
 * `audit_log` mostrerebbe due inserimenti perfettamente legittimi. È il difetto
 * che l'header esiste per impedire, ed è invisibile a chi guarda il codice del
 * form.
 *
 * # `payer_user_id` è una scelta, non un campo che si dimentica
 *
 * `payments_select` mostra la riga a chi ha pagato: **senza attribuzione
 * l'incasso resta visibile allo staff soltanto**, e l'inquilino non lo vede nel
 * proprio elenco pur vedendone l'effetto sul residuo. È lecito — un bonifico da un
 * conto che non si sa a chi ricondurre esiste — ma è anche il comportamento di chi
 * semplicemente omette il campo, cioè il caso più facile.
 *
 * Qui il campo **non ha un vuoto di comodo**: o si sceglie una persona, o si
 * dichiara «non attribuito». Il form non si invia finché non si è scelto, e
 * l'opzione dichiarata porta con sé la sua conseguenza scritta.
 *
 * # Solo chi ha accettato l'invito può essere scelto
 *
 * Una parte contrattuale senza `user_id` è un invitato che non ha ancora
 * accettato: attribuirgli l'incasso è impossibile, perché la policy confronta un
 * identificatore che non esiste ancora. Compare quindi il solo «non attribuito»,
 * con la ragione, invece di un menù di nomi che darebbero tutti lo stesso esito.
 */
export function RecordPaymentDialog({ charge }: { charge: Charge }) {
  const client = useApi()
  const queryClient = useQueryClient()
  const intent = useIntentKey()

  const [open, setOpen] = useState(false)
  const [values, setValues] = useState<Values>(emptyValues)
  const [submitted, setSubmitted] = useState(false)

  // Le parti del contratto: servono i nomi e gli identificatori di chi ha
  // accettato. Si legge solo a modale aperta, perché è l'unico momento in cui
  // serve.
  const lease = useQuery({
    queryKey: keys.lease(charge.lease_id),
    queryFn: () => getLease(client, charge.lease_id),
    enabled: open,
  })

  const candidates = (lease.data?.parties ?? []).filter(
    (party) => party.user_id !== undefined && party.left_at === null,
  )

  const amountError = amountProblem(values.amount, submitted)
  const payerError = submitted && values.payer === '' ? t.validation.required : undefined

  const mutation = useMutation({
    // Nessuna chiave generata qui dentro: `intent.current()` restituisce quella
    // dell'intento, e un ritentativo riceve la stessa.
    mutationFn: () =>
      recordPayment(
        client,
        charge.id,
        {
          amount: values.amount.trim().replace(',', '.'),
          method: values.method,
          // Mostrata e non mandata finché è quella di oggi: vedi `paidAtBody`.
          // Qui cambiarla è il gesto **normale** e non l'eccezione — un bonifico si
          // vede sull'estratto conto giorni dopo — e per questo il campo deve
          // dire da che data si parte.
          ...paidAtBody(values.paid_at),
          ...(values.reference.trim() ? { reference: values.reference.trim() } : {}),
          ...(values.payer === UNATTRIBUTED ? {} : { payer_user_id: values.payer }),
        },
        intent.current(),
      ),
    onSuccess: async () => {
      // L'incasso è entrato: da qui in poi un invio è un intento **nuovo**.
      intent.renew()
      setOpen(false)
      setValues(emptyValues())
      setSubmitted(false)
      await queryClient.invalidateQueries({ queryKey: keys.charge(charge.id) })
      await queryClient.invalidateQueries({ queryKey: keys.chargePayments(charge.id) })
      await queryClient.invalidateQueries({ queryKey: ['charges'] })
      await queryClient.invalidateQueries({ queryKey: ['payments'] })
      await queryClient.invalidateQueries({ queryKey: keys.dashboard })
    },
  })

  return (
    <FormDialog
      trigger={
        <Button variant="primary">
          <Banknote aria-hidden />
          {t.payment.record}
        </Button>
      }
      title={t.payment.recordTitle}
      description={t.payment.recordExplain}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) {
          // Chiudere senza inviare **non** butta via la chiave a metà di un invio
          // riuscito: `renew` qui riguarda l'intento successivo, che è un altro
          // incasso.
          intent.renew()
          setValues(emptyValues())
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
          if (amountProblem(values.amount, true) || values.payer === '') return
          mutation.mutate()
        }}
      >
        <InlineError error={mutation.error} />

        <p className="rounded-lg bg-surface-muted px-3 py-2 text-xs text-ink-muted">
          {t.charge.outstandingAmount}:{' '}
          <span className="tabular font-medium text-ink">
            {formatAmount(charge.outstanding_amount, { currency: charge.currency })}
          </span>
        </p>

        <Field label={t.payment.amount} error={amountError} required>
          {(props) => (
            <Input
              {...props}
              inputMode="decimal"
              autoFocus
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

          <Field label={t.payment.paidAt} hint={t.payment.paidAtHint}>
            {(props) => (
              <Input
                {...props}
                type="date"
                value={values.paid_at}
                onChange={(event) => setValues((v) => ({ ...v, paid_at: event.target.value }))}
              />
            )}
          </Field>
        </div>

        <Field
          label={t.payment.payer}
          hint={candidates.length === 0 ? t.payment.payerNoCandidates : t.payment.payerHint}
          error={payerError}
          required
        >
          {(props) => (
            <Select
              {...props}
              value={values.payer}
              onChange={(event) => setValues((v) => ({ ...v, payer: event.target.value }))}
            >
              {/* Un vuoto che **non si può inviare**: è ciò che rende
                  l'attribuzione una scelta invece di un default. */}
              <option value="">{t.common.none}</option>
              {candidates.map((party) => (
                <option key={party.id} value={party.user_id}>
                  {party.full_name}
                </option>
              ))}
              <option value={UNATTRIBUTED}>{t.payment.payerUnattributed}</option>
            </Select>
          )}
        </Field>

        {values.payer === UNATTRIBUTED ? (
          <p className="rounded-lg border border-warning/30 bg-warning-soft px-3 py-2 text-xs text-warning">
            {t.payment.payerUnattributedHint}
          </p>
        ) : null}

        <Field label={t.payment.reference} hint={t.payment.referenceHint}>
          {(props) => (
            <Input
              {...props}
              value={values.reference}
              onChange={(event) => setValues((v) => ({ ...v, reference: event.target.value }))}
            />
          )}
        </Field>

        <div className="flex justify-end gap-2">
          <Button type="button" onClick={() => setOpen(false)} disabled={mutation.isPending}>
            {t.common.cancel}
          </Button>
          <Button type="submit" variant="primary" disabled={mutation.isPending}>
            {mutation.isPending ? t.common.saving : t.payment.record}
          </Button>
        </div>
      </form>
    </FormDialog>
  )
}

/**
 * L'importo, validato senza passare da un float.
 *
 * `parseFloat` è vietato: `"184.20"` letto come numero binario torna
 * `184.19999999999999`, e in contabilità quella differenza si accumula. Il
 * controllo passa da `@/lib/decimal`, che lavora su interi.
 *
 * L'errore compare **dopo il primo invio** e non a ogni tasto: segnalare un campo
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
