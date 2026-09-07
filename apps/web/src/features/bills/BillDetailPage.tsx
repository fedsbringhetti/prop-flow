import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Ban, CheckCircle2, Upload } from 'lucide-react'
import { useRef, useState } from 'react'
import { Link, useParams } from 'react-router'

import {
  UPLOAD_LIMIT_BYTES,
  documentContentPath,
  getBill,
  listDocuments,
  listLeases,
  rechargeBill,
  uploadDocument,
  validateBill,
  voidBill,
} from '@/api/endpoints'
import { ApiError } from '@/api/problem'
import { Async, Empty, InlineError } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card, CardBody, CardHeader, CardTitle } from '@/components/ui/card'
import { ConfirmAction, FormDialog } from '@/components/ui/confirm'
import { Field, Input, Select } from '@/components/ui/field'
import { DescriptionItem, DescriptionList, PageHeader, PageSection } from '@/components/ui/page'
import { keys } from '@/app/query'
import { useApi, useIsStaff } from '@/app/session'
import { t } from '@/i18n'
import { formatAmount, formatDate, formatDecimal, todayIso } from '@/lib/format'
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
 * La scansione, e il controllo della dimensione **prima** di inviarla.
 *
 * Il limite del corpo su queste rotte è 25 MiB, e oltre quello la risposta è un
 * `413` che non dice quale limite né su quale file. Una fotografia da 30 MB scelta
 * da un telefono ci finisce dentro senza fatica: controllare prima costa una riga
 * e trasforma un errore muto in una frase che si capisce.
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
  const client = useApi()
  const queryClient = useQueryClient()
  const input = useRef<HTMLInputElement>(null)
  const [error, setError] = useState<unknown>(null)

  const documents = useQuery({
    queryKey: keys.documents('utility-bills', billId),
    queryFn: () => listDocuments(client, { type: 'utility-bills', id: billId }),
  })

  const upload = useMutation({
    mutationFn: (file: File) =>
      uploadDocument(client, { type: 'utility-bills', id: billId }, file, 'utility_bill'),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.documents('utility-bills', billId) })
      // Su una bolletta in bozza il caricamento **accoda la lettura automatica**:
      // il job non compare nella risposta del caricamento, quindi va riletto.
      await queryClient.invalidateQueries({ queryKey: keys.billOcr(billId) })
      await queryClient.invalidateQueries({ queryKey: keys.bill(billId) })
      setError(null)
    },
    onError: setError,
  })

  return (
    <Card>
      <CardHeader>
        <div>
          <CardTitle>{t.bill.scan}</CardTitle>
          {draft && canUpload ? (
            <p className="mt-0.5 text-xs text-ink-muted">{t.bill.uploadHint}</p>
          ) : null}
        </div>
        {canUpload ? (
          <div>
            <input
              ref={input}
              type="file"
              className="sr-only"
              accept="application/pdf,image/*"
              onChange={(event) => {
                const file = event.target.files?.[0]
                event.target.value = ''
                if (!file) return
                if (file.size > UPLOAD_LIMIT_BYTES) {
                  setError(new ApiError({ status: 413, code: 'request.payload_too_large' }))
                  return
                }
                upload.mutate(file)
              }}
            />
            <Button size="sm" onClick={() => input.current?.click()} disabled={upload.isPending}>
              <Upload aria-hidden />
              {upload.isPending ? t.common.saving : t.bill.uploadScan}
            </Button>
          </div>
        ) : null}
      </CardHeader>
      <CardBody className="flex flex-col gap-3">
        <InlineError error={error} />
        <Async
          query={documents}
          isEmpty={(page) => page.items.length === 0}
          empty={<Empty title={t.ocr.none} />}
        >
          {(page) => (
            <ul className="divide-y divide-border-subtle">
              {page.items.map((document) => (
                <li
                  key={document.id}
                  className="flex flex-wrap items-center justify-between gap-2 py-2 text-sm"
                >
                  <span className="min-w-0 truncate">{document.original_filename}</span>
                  <a
                    href={documentContentPath(document.id)}
                    target="_blank"
                    rel="noreferrer"
                    className="text-accent underline-offset-4 hover:underline"
                  >
                    {t.bill.download}
                  </a>
                </li>
              ))}
            </ul>
          )}
        </Async>
      </CardBody>
    </Card>
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
