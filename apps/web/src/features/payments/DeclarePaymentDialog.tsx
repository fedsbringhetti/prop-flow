import { useMutation, useQueryClient } from '@tanstack/react-query'
import { HandCoins } from 'lucide-react'
import { useState } from 'react'

import { declarePayment } from '@/api/endpoints'
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
import { formatAmount } from '@/lib/format'

/** L'ordine è quello del menù: il bonifico per primo perché è il caso comune. */
const METHODS: readonly PaymentMethod[] = [
  'bank_transfer',
  'sepa_direct_debit',
  'card',
  'cash',
  'check',
  'other',
] as const

interface Values {
  amount: string
  method: PaymentMethod
  paid_at: string
  reference: string
}

/**
 * Dichiarare di aver pagato.
 *
 * È il gesto che chiude il giro che la fetta A ha aperto: là la coda delle
 * dichiarazioni è stata costruita e collaudata **da `curl`**, perché un inquilino
 * non aveva un'interfaccia. Da qui quella coda diventa una cosa che accade da sé.
 *
 * # La riga nasce `pending` e **non muove il saldo**
 *
 * È tutto il punto della rotta: rendere visibile un pagamento senza renderlo vero.
 * A muovere il residuo è la conferma dello staff, e finché quella non arriva
 * l'addebito resta dov'era — il messaggio lo dice, perché una schermata che
 * lasciasse credere il contrario produrrebbe due persone convinte di due cose
 * diverse sullo stesso numero.
 *
 * # Il corpo **non ha `payer_user_id`**
 *
 * Chi dichiara è sempre chi chiama: `payments_insert` ammette
 * `is_org_staff(org_id) OR payer_user_id = app.current_user_id()`, e quelle due
 * condizioni sono le due rotte. Passarlo produrrebbe zero righe inserite, cioè
 * `auth.denied_by_policy` — il codice riservato ai nostri difetti.
 *
 * # E **non c'è un bottone per ritirarla**
 *
 * È controintuitivo abbastanza da doverlo dire: si crea qualcosa che poi non si
 * può disfare. Non è una scelta di prodotto rivedibile qui — `payments_update` è
 * **staff soltanto**, quindi un ritiro otterrebbe zero righe aggiornate. L'uscita
 * da `pending` esiste ed è il rifiuto, che è dello staff. Il ritiro di chi ha
 * dichiarato esiste invece **fra i regolamenti** delle spese co-living, dove
 * `settlements_update` ammette `from_user_id`: le due cose si somigliano e non
 * sono la stessa.
 *
 * # La chiave di idempotenza si riusa, non si riscrive
 *
 * `useIntentKey` è quella della fetta A, con i suoi cinque test. La condizione è
 * la stessa: **si genera all'intento** — l'apertura del form — e si riusa a ogni
 * tentativo, il secondo clic e il rinvio dopo un errore di rete compresi.
 * Generarla dentro `mutationFn` sembra corretto perché il codice starebbe accanto
 * al corpo, e non lo è: quella funzione gira una volta per tentativo, quindi un
 * doppio invio produrrebbe **due dichiarazioni**.
 */
export function DeclarePaymentDialog({ charge }: { charge: Charge }) {
  const client = useApi()
  const queryClient = useQueryClient()
  const intent = useIntentKey()

  const [open, setOpen] = useState(false)
  const [values, setValues] = useState<Values>(() => emptyValues(charge))
  const [submitted, setSubmitted] = useState(false)

  const amountError = amountProblem(values.amount, submitted)

  const mutation = useMutation({
    // Nessuna chiave generata qui dentro: `intent.current()` restituisce quella
    // dell'intento, e un ritentativo riceve la stessa.
    mutationFn: () =>
      declarePayment(
        client,
        charge.id,
        {
          amount: values.amount.trim().replace(',', '.'),
          method: values.method,
          ...(values.paid_at ? { paid_at: new Date(values.paid_at).toISOString() } : {}),
          ...(values.reference.trim() ? { reference: values.reference.trim() } : {}),
        },
        intent.current(),
      ),
    onSuccess: async () => {
      // La dichiarazione è partita: da qui in poi un invio è un intento **nuovo**.
      intent.renew()
      setOpen(false)
      setValues(emptyValues(charge))
      setSubmitted(false)
      await queryClient.invalidateQueries({ queryKey: keys.charge(charge.id) })
      await queryClient.invalidateQueries({ queryKey: keys.chargePayments(charge.id) })
      await queryClient.invalidateQueries({ queryKey: ['charges'] })
      await queryClient.invalidateQueries({ queryKey: ['payments'] })
    },
  })

  return (
    <FormDialog
      trigger={
        <Button variant="primary">
          <HandCoins aria-hidden />
          {t.payment.declare}
        </Button>
      }
      title={t.payment.declareTitle}
      description={t.payment.declareExplain}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) {
          // Chiudere senza inviare non butta via la chiave a metà di un invio
          // riuscito: `renew` qui riguarda l'intento successivo, che è un altro
          // pagamento.
          intent.renew()
          setValues(emptyValues(charge))
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
          if (amountProblem(values.amount, true)) return
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

        <Field label={t.payment.declaredAmount} hint={t.payment.declaredAmountHint} error={amountError} required>
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

        <Field label={t.payment.reference} hint={t.payment.declaredReferenceHint}>
          {(props) => (
            <Input
              {...props}
              value={values.reference}
              onChange={(event) => setValues((v) => ({ ...v, reference: event.target.value }))}
            />
          )}
        </Field>

        <p className="rounded-lg border border-warning/30 bg-warning-soft px-3 py-2 text-xs text-warning">
          {t.payment.declareNoWithdraw}
        </p>

        <div className="flex justify-end gap-2">
          <Button type="button" onClick={() => setOpen(false)} disabled={mutation.isPending}>
            {t.common.cancel}
          </Button>
          <Button type="submit" variant="primary" disabled={mutation.isPending}>
            {mutation.isPending ? t.common.saving : t.payment.declare}
          </Button>
        </div>
      </form>
    </FormDialog>
  )
}

/**
 * L'importo parte dal **residuo**, e non è un default nascosto.
 *
 * Sta scritto nel campo, dove si legge e si corregge: il caso comune è pagare
 * tutto quello che si deve, e far ridigitare `184,20` a chi ce l'ha davanti agli
 * occhi è attrito senza contropartita. Un residuo negativo — un credito da
 * sovrapagamento — non si può dichiarare, quindi il campo resta vuoto.
 */
function emptyValues(charge: Charge): Values {
  const outstanding = charge.outstanding_amount
  return {
    amount: isPositive(outstanding) ? outstanding : '',
    method: 'bank_transfer',
    paid_at: '',
    reference: '',
  }
}

/**
 * L'importo, validato **senza passare da un float**.
 *
 * `parseFloat` è vietato: `"184.20"` letto come binario torna
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
