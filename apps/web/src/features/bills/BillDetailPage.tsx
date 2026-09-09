import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { BadgeEuro, Ban, CheckCircle2 } from 'lucide-react'
import { useState } from 'react'
import { Link, useParams } from 'react-router'

import {
  getBill,
  listLeases,
  markBillPaid,
  rechargeBill,
  validateBill,
  voidBill,
} from '@/api/endpoints'
import { Async, InlineError } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card, CardBody, CardHeader, CardTitle } from '@/components/ui/card'
import { ConfirmAction, FormDialog } from '@/components/ui/confirm'
import { Field, Input, Select } from '@/components/ui/field'
import { DescriptionItem, DescriptionList, PageHeader, PageSection } from '@/components/ui/page'
import { keys } from '@/app/query'
import { useApi, useIsStaff } from '@/app/session'
import { t } from '@/i18n'
import { formatAmount, formatDate, formatDecimal, todayIso } from '@/lib/format'
import { AttachmentsCard } from '../shared/attachments'
import { BillStatusBadge } from '../shared/status'
import { OcrReviewCard } from './OcrReview'

export function BillDetailPage() {
  const { id = '' } = useParams()
  const client = useApi()
  const queryClient = useQueryClient()

  const isStaff = useIsStaff()

  const bill = useQuery({ queryKey: keys.bill(id), queryFn: () => getBill(client, id) })

  async function refresh() {
    await queryClient.invalidateQueries({ queryKey: keys.bill(id) })
    await queryClient.invalidateQueries({ queryKey: ['bills'] })
    await queryClient.invalidateQueries({ queryKey: keys.dashboard })
  }

  const validate = useMutation({ mutationFn: () => validateBill(client, id), onSuccess: refresh })
  const discard = useMutation({ mutationFn: () => voidBill(client, id), onSuccess: refresh })
  const pay = useMutation({ mutationFn: () => markBillPaid(client, id), onSuccess: refresh })

  return (
    <Async query={bill}>
      {(data) => (
        <>
          <PageHeader
            title={t.bill.types[data.utility_type]}
            description={
              data.invoice_number
                ? `${data.supplier_name ?? ''} ${data.invoice_number}`.trim()
                : (data.supplier_name ?? undefined)
            }
            breadcrumb={
              <Link to="/bollette" className="hover:underline">
                {t.bill.title}
              </Link>
            }
            actions={
              /* Validare, riaddebitare e annullare sono dello staff. L'inquilino
                 la **legge**: è la ragione di metà dei suoi addebiti, e un
                 riaddebito che non si può risalire a un documento è una cifra da
                 accettare sulla fiducia. */
              !isStaff ? null : (
                <>
                  {data.status === 'draft' || data.status === 'pending_review' ? (
                  <ConfirmAction
                    trigger={
                      <Button variant="primary">
                        <CheckCircle2 aria-hidden />
                        {t.bill.validate}
                      </Button>
                    }
                    title={t.bill.validateConfirm}
                    explanation={t.bill.validateExplain}
                    confirmLabel={t.bill.validate}
                    variant="primary"
                    onConfirm={() => validate.mutateAsync()}
                  />
                ) : null}
                {data.status === 'validated' || data.status === 'allocated' ? (
                  <RechargeDialog billId={id} propertyId={data.property_id} total={data.total_amount} />
                ) : null}
                {/* **Dichiarare, non registrare un pagamento.** Il denaro in
                    uscita non è modellato: `app.payments` è per ciò che entra, e
                    un fornitore non è un utente del sistema. Si offre da
                    `validated` e da `allocated`, perché il fornitore non aspetta
                    il riaddebito. */}
                {data.status === 'validated' || data.status === 'allocated' ? (
                  <ConfirmAction
                    trigger={
                      <Button>
                        <BadgeEuro aria-hidden />
                        {t.bill.pay}
                      </Button>
                    }
                    title={t.bill.payConfirm}
                    explanation={t.bill.payExplain}
                    confirmLabel={t.bill.pay}
                    onConfirm={() => pay.mutateAsync()}
                  />
                ) : null}
                {data.status === 'allocated' || data.status === 'void' ? null : (
                  <ConfirmAction
                    trigger={
                      <Button variant="dangerGhost">
                        <Ban aria-hidden />
                        {t.bill.void}
                      </Button>
                    }
                    title={t.bill.voidConfirm}
                    explanation={t.bill.voidExplain}
                    confirmLabel={t.bill.void}
                    onConfirm={() => discard.mutateAsync()}
                  />
                )}
                </>
              )
            }
          />

          <PageSection>
            <Card>
              <CardHeader>
                <CardTitle>{t.bill.title}</CardTitle>
                <BillStatusBadge status={data.status} />
              </CardHeader>
              <CardBody className="flex flex-col gap-4">
                <DescriptionList>
                  <DescriptionItem label={t.bill.totalAmount} numeric>
                    {formatAmount(data.total_amount, { currency: data.currency })}
                  </DescriptionItem>
                  <DescriptionItem label={t.bill.consumption} numeric>
                    {data.consumption
                      ? `${formatDecimal(data.consumption.value)} ${data.consumption.unit}`
                      : t.common.none}
                  </DescriptionItem>
                  <DescriptionItem label={t.bill.periodStart}>
                    {formatDate(data.period_start)} — {formatDate(data.period_end)}
                  </DescriptionItem>
                  <DescriptionItem label={t.bill.supplierVat}>
                    {data.supplier_vat ?? t.common.none}
                  </DescriptionItem>
                  <DescriptionItem label={t.bill.lease}>
                    {data.lease_id ? (
                      <Link
                        to={`/contratti/${data.lease_id}`}
                        className="text-accent underline-offset-4 hover:underline"
                      >
                        {t.lease.title}
                      </Link>
                    ) : (
                      t.common.none
                    )}
                  </DescriptionItem>
                  <DescriptionItem label={t.bill.validatedAt}>
                    {data.validated_at ? formatDate(data.validated_at) : t.common.none}
                  </DescriptionItem>
                  {data.paid_at ? (
                    <DescriptionItem label={t.bill.paidAt}>
                      {formatDate(data.paid_at)}
                    </DescriptionItem>
                  ) : null}
                </DescriptionList>
                {data.status === 'validated' ? (
                  <p className="rounded-lg bg-surface-muted px-3 py-2 text-xs text-ink-muted">
                    {t.bill.correctionUndoesValidation}
                  </p>
                ) : null}
              </CardBody>
            </Card>
          </PageSection>

          <PageSection>
            <ScansCard billId={id} draft={data.status === 'draft'} canUpload={isStaff} />
          </PageSection>

          {/* La revisione dell'OCR è un lavoro di chi valida: propone valori da
              scrivere sulla bolletta, e `UpdateUtilityBill` è `ensure_staff`.
              Mostrarla a chi abita sarebbe un modulo intero di bottoni inerti. */}
          {isStaff ? (
            <PageSection>
              <OcrReviewCard bill={data} />
            </PageSection>
          ) : null}
        </>
      )}
    </Async>
  )
}

/**
 * La scansione della bolletta, che è la scheda degli allegati **con un lavoro in
 * più**.
 *
 * Era il modello da cui `AttachmentsCard` è stata estratta, e qui resta un
 * involucro sottile per una ragione che le altre cinque non hanno: su una bolletta
 * in bozza il caricamento **accoda la lettura automatica**, e il job non compare
 * nella risposta del caricamento. Le due chiavi dell'OCR vanno quindi rilette
 * dopo, e sono le sole invalidazioni che non appartengono alla scheda condivisa.
 */
function ScansCard({
  billId,
  draft,
  canUpload,
}: {
  billId: string
  draft: boolean
  /** Caricare è dello staff; **scaricare no**: un allegato è la prova di ciò che
   *  si è riaddebitato, e chi paga deve poterlo aprire. */
  canUpload: boolean
}) {
  const queryClient = useQueryClient()

  return (
    <AttachmentsCard
      owner={{ type: 'utility-bills', id: billId }}
      title={t.bill.scan}
      {...(draft ? { hint: t.bill.uploadHint } : {})}
      kind="utility_bill"
      canUpload={canUpload}
      emptyTitle={t.ocr.none}
      onUploaded={async () => {
        // Su una bolletta in bozza il caricamento **accoda la lettura
        // automatica**: il job non compare nella risposta del caricamento,
        // quindi va riletto. Sulle altre non c'è niente da rileggere.
        if (!draft) return
        await queryClient.invalidateQueries({ queryKey: keys.billOcr(billId) })
        await queryClient.invalidateQueries({ queryKey: keys.bill(billId) })
      }}
    />
  )
}

/**
 * Il riaddebito, con l'avvertenza scritta e non nascosta.
 *
 * Nessuno verifica che le quote riaddebitate sommino al totale della bolletta: lo
 * schema non ha il vincolo, e un controllo applicativo sarebbe soggetto a race
 * condition fra due riaddebiti concorrenti. Riaddebitare più del totale **è
 * possibile**, e chi lo fa deve saperlo prima e non dopo.
 */
function RechargeDialog({
  billId,
  propertyId,
  total,
}: {
  billId: string
  propertyId: string
  total: string
}) {
  const client = useApi()
  const queryClient = useQueryClient()
  const [open, setOpen] = useState(false)
  const [error, setError] = useState<unknown>(null)
  const [values, setValues] = useState({ lease_id: '', amount: total, due_date: todayIso() })

  const leases = useQuery({
    queryKey: keys.leases({ property: propertyId, perRiaddebito: true }),
    queryFn: () => listLeases(client, { property_id: propertyId }),
    enabled: open,
  })

  const mutation = useMutation({
    mutationFn: () =>
      rechargeBill(client, billId, {
        lease_id: values.lease_id,
        amount: values.amount,
        due_date: values.due_date,
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.bill(billId) })
      await queryClient.invalidateQueries({ queryKey: ['bills'] })
      await queryClient.invalidateQueries({ queryKey: ['charges'] })
      await queryClient.invalidateQueries({ queryKey: keys.dashboard })
      setOpen(false)
      setError(null)
    },
    onError: setError,
  })

  return (
    <FormDialog
      trigger={<Button variant="primary">{t.bill.recharge}</Button>}
      title={t.bill.rechargeTitle}
      description={t.bill.rechargeExplain}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) setError(null)
      }}
    >
      <form
        noValidate
        className="flex flex-col gap-4"
        onSubmit={(event) => {
          event.preventDefault()
          mutation.mutate()
        }}
      >
        <InlineError error={error} />
        <Field label={t.charge.lease} required>
          {(props) => (
            <Select
              {...props}
              value={values.lease_id}
              onChange={(event) => setValues((v) => ({ ...v, lease_id: event.target.value }))}
            >
              <option value="">—</option>
              {(leases.data?.items ?? []).map((lease) => (
                <option key={lease.id} value={lease.id}>
                  {t.lease.types[lease.lease_type]} · {formatDate(lease.start_date)}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <Field label={t.bill.rechargeAmount} required>
            {(props) => (
              <Input
                {...props}
                inputMode="decimal"
                value={values.amount}
                onChange={(event) => setValues((v) => ({ ...v, amount: event.target.value }))}
              />
            )}
          </Field>
          <Field label={t.bill.rechargeDueDate} required>
            {(props) => (
              <Input
                {...props}
                type="date"
                value={values.due_date}
                onChange={(event) => setValues((v) => ({ ...v, due_date: event.target.value }))}
              />
            )}
          </Field>
        </div>
        <div className="flex justify-end gap-2">
          <Button onClick={() => setOpen(false)}>{t.common.cancel}</Button>
          <Button type="submit" variant="primary" disabled={mutation.isPending || !values.lease_id}>
            {mutation.isPending ? t.common.saving : t.bill.recharge}
          </Button>
        </div>
      </form>
    </FormDialog>
  )
}
