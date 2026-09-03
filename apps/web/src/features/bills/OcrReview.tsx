import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Check, RefreshCw } from 'lucide-react'
import { useState } from 'react'

import { getBillOcr, updateBill } from '@/api/endpoints'
import type { BillOcrStatus, ExtractedField, UtilityBill } from '@/api/types'
import { Async, Empty, InlineError } from '@/components/state'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardBody, CardHeader, CardTitle } from '@/components/ui/card'
import { keys } from '@/app/query'
import { useApi } from '@/app/session'
import { t } from '@/i18n'
import { compareDecimals } from '@/lib/decimal'
import { formatAmount, formatDate, formatDecimal, formatFraction } from '@/lib/format'

/**
 * La revisione della proposta dell'OCR.
 *
 * È qui che la fetta precedente diventa visibile: senza questa schermata il
 * percorso dell'OCR funziona e si osserva solo da `psql`.
 *
 * # Perché si accetta **un campo alla volta**
 *
 * Lo schema tiene separato ciò che ha letto la macchina (`ocr_extracted`) da ciò
 * che ha confermato una persona (le colonne della bolletta) **perché l'accuratezza
 * si possa misurare**: confrontando quello che è stato proposto con quello che è
 * stato confermato. Un bottone «applica tutto» distruggerebbe quel dato prima
 * ancora di raccoglierlo — resterebbe una bolletta corretta e nessun modo di
 * sapere se la macchina ci aveva preso.
 *
 * Applicare resta possibile, e in un gesto solo, ma **solo sui campi accettati
 * uno per uno**: è una scelta della persona, non un automatismo.
 *
 * # La confidenza è la peggiore, non la media
 *
 * Dieci campi letti a `0.99` non devono nascondere il totale letto a `0.40`, che
 * è l'unica cifra che qualcuno pagherà. Una confidenza bassa **non è un errore**:
 * il job resta `succeeded`, e il numero è ciò da cui si comincia a guardare.
 */
export function OcrReviewCard({ bill }: { bill: UtilityBill }) {
  const client = useApi()
  const queryClient = useQueryClient()
  const [accepted, setAccepted] = useState<Record<string, ExtractedField>>({})
  const [error, setError] = useState<unknown>(null)

  const ocr = useQuery({
    queryKey: keys.billOcr(bill.id),
    queryFn: () => getBillOcr(client, bill.id),
    // Un job in coda o in corso lo smaltisce il worker, non questa richiesta: si
    // rilegge finché non è concluso, e ci si ferma appena lo è. Senza la
    // condizione, la schermata interrogherebbe l'API per sempre.
    refetchInterval: (query) => {
      const status = query.state.data?.job?.status
      return status === 'queued' || status === 'running' ? 3000 : false
    },
  })

  const apply = useMutation({
    mutationFn: () => updateBill(client, bill.id, patchFromAccepted(accepted)),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.bill(bill.id) })
      await queryClient.invalidateQueries({ queryKey: keys.billOcr(bill.id) })
      await queryClient.invalidateQueries({ queryKey: ['bills'] })
      setAccepted({})
      setError(null)
    },
    onError: setError,
  })

  return (
    <Card>
      <CardHeader>
        <div>
          <CardTitle>{t.ocr.title}</CardTitle>
          <p className="mt-0.5 text-xs text-ink-muted">{t.ocr.subtitle}</p>
        </div>
        <Button
          size="sm"
          onClick={() => void ocr.refetch()}
          disabled={ocr.isFetching}
          aria-label={t.ocr.refresh}
        >
          <RefreshCw aria-hidden className={ocr.isFetching ? 'animate-spin' : undefined} />
          {t.ocr.refresh}
        </Button>
      </CardHeader>

      <Async query={ocr}>
        {(status) => (
          <CardBody className="flex flex-col gap-4">
            <JobState status={status} />
            <InlineError error={error} />

            {status.extracted && status.extracted.length > 0 ? (
              <>
                <ul className="divide-y divide-border-subtle rounded-lg border border-border-subtle">
                  {status.extracted.map((field) => (
                    <FieldRow
                      key={field.field}
                      field={field}
                      bill={bill}
                      accepted={accepted[field.field] !== undefined}
                      onAccept={() =>
                        setAccepted((current) => {
                          const next = { ...current }
                          if (next[field.field] === undefined) next[field.field] = field
                          else delete next[field.field]
                          return next
                        })
                      }
                    />
                  ))}
                </ul>

                <p className="text-xs text-ink-muted">{t.ocr.whyFieldByField}</p>

                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span className="text-xs text-ink-muted">
                    {Object.keys(accepted).length === 0
                      ? t.ocr.nothingAccepted
                      : `${Object.keys(accepted).length} ${t.common.of} ${status.extracted.length}`}
                  </span>
                  <Button
                    variant="primary"
                    disabled={Object.keys(accepted).length === 0 || apply.isPending}
                    onClick={() => apply.mutate()}
                  >
                    {apply.isPending ? t.common.saving : t.ocr.apply}
                  </Button>
                </div>
              </>
            ) : status.extracted ? (
              // Array **vuoto**: la macchina ha letto e non ha trovato niente di
              // interpretabile. È un esito diverso da «non ha ancora letto», e
              // confonderli manderebbe qualcuno ad aspettare per sempre.
              <Empty title={t.ocr.succeeded} hint={t.ocr.none} />
            ) : null}
          </CardBody>
        )}
      </Async>
    </Card>
  )
}

function JobState({ status }: { status: BillOcrStatus }) {
  if (!status.job) {
    return <p className="text-sm text-ink-muted">{t.ocr.none}</p>
  }

  const job = status.job
  const label = t.ocr[job.status]
  const lowConfidence = status.confidence ? compareDecimals(status.confidence, '0.7') < 0 : false

  return (
    <div className="flex flex-wrap items-center gap-3">
      <Badge
        tone={
          job.status === 'succeeded'
            ? 'success'
            : job.status === 'failed' || job.status === 'cancelled'
              ? 'danger'
              : 'warning'
        }
      >
        {label}
      </Badge>
      {status.confidence ? (
        <span className="text-sm">
          <span className="text-ink-muted">{t.ocr.confidence}: </span>
          <span className={lowConfidence ? 'tabular font-medium text-danger' : 'tabular font-medium'}>
            {formatFraction(status.confidence)}
          </span>
        </span>
      ) : null}
      {job.error_code ? (
        <span className="text-sm text-danger">{t.ocr.error[job.error_code]}</span>
      ) : null}
      <span className="text-xs text-ink-muted">
        {t.ocr.provider}: {job.provider_name}
        {job.model ? ` · ${t.ocr.model}: ${job.model}` : ''}
      </span>
      {lowConfidence ? (
        <p className="w-full text-xs text-danger">{t.ocr.lowConfidence}</p>
      ) : null}
      <p className="w-full text-xs text-ink-muted">{t.ocr.confidenceNote}</p>
    </div>
  )
}

/**
 * Il valore attuale della bolletta per il campo che la macchina propone.
 *
 * I nomi sono quelli delle **colonne della bolletta**, ed è deliberato lato
 * server: chi rivede una proposta guarda quella colonna. `consumption` è una
 * coppia — valore e unità — perché `123.4567` da solo non dice niente.
 *
 * **Non si converte niente in numero**: il confronto è fra stringhe come l'API le
 * manda, così `"184.20"` contro `"184.2"` risulta uguale grazie a un confronto
 * decimale e non a un `parseFloat`.
 */
function currentValue(bill: UtilityBill, field: string): string | null {
  switch (field) {
    case 'supplier_name':
      return bill.supplier_name ?? null
    case 'supplier_vat':
      return bill.supplier_vat ?? null
    case 'invoice_number':
      return bill.invoice_number ?? null
    case 'issue_date':
      return bill.issue_date
    case 'due_date':
      return bill.due_date
    case 'period_start':
      return bill.period_start
    case 'period_end':
      return bill.period_end
    case 'total_amount':
      return bill.total_amount
    case 'taxable_amount':
      return bill.taxable_amount
    case 'vat_amount':
      return bill.vat_amount
    case 'consumption':
      return bill.consumption?.value ?? null
    default:
      return null
  }
}

/** L'unità del valore attuale, che esiste solo per il consumo. */
function currentUnit(bill: UtilityBill, field: string): string | null {
  return field === 'consumption' ? (bill.consumption?.unit ?? null) : null
}

function sameValue(field: ExtractedField, current: string | null, currentUnitValue: string | null): boolean {
  if (current === null) return false
  if (field.type === 'amount' || field.type === 'quantity') {
    // Il consumo è uguale solo se lo è anche l'unità: `128,91` in Smc e in kWh
    // sono due letture diverse della stessa bolletta.
    if (field.unit !== undefined && field.unit !== (currentUnitValue ?? undefined)) return false
    return compareDecimals(field.value, current) === 0
  }
  return field.value === current
}

/**
 * Mostra un valore **come si legge**, non come viaggia.
 *
 * Una data resta `2026-06-01` sul filo e diventa `01/06/2026` qui: confrontare a
 * occhio due formati diversi è precisamente il lavoro che questa schermata deve
 * togliere a chi rivede.
 */
function displayValue(
  raw: string | null,
  type: ExtractedField['type'],
  unit: string | null | undefined,
  currency: string,
): string {
  if (raw === null) return t.common.none
  switch (type) {
    case 'date':
      return formatDate(raw)
    case 'amount':
      return formatAmount(raw, { currency })
    case 'quantity':
      return `${formatDecimal(raw)}${unit ? ` ${unit}` : ''}`
    default:
      return raw
  }
}

function FieldRow({
  field,
  bill,
  accepted,
  onAccept,
}: {
  field: ExtractedField
  bill: UtilityBill
  accepted: boolean
  onAccept: () => void
}) {
  const current = currentValue(bill, field.field)
  const unit = currentUnit(bill, field.field)
  const identical = sameValue(field, current, unit)
  const labels: Record<string, string> = t.ocr.fields
  const low = compareDecimals(field.confidence, '0.7') < 0

  return (
    <li className="flex flex-col gap-2 p-3 sm:flex-row sm:items-center sm:gap-4">
      <div className="min-w-40 flex-1">
        <span className="text-xs font-medium tracking-wide text-ink-muted uppercase">
          {labels[field.field] ?? field.field}
        </span>
        <div className="mt-1 grid grid-cols-1 gap-1 text-sm sm:grid-cols-2 sm:gap-4">
          <span>
            <span className="text-xs text-ink-muted">{t.ocr.current}: </span>
            <span className="tabular">
              {displayValue(current, field.type, unit, bill.currency)}
            </span>
          </span>
          <span>
            <span className="text-xs text-ink-muted">{t.ocr.proposed}: </span>
            <span className="tabular font-medium">
              {displayValue(field.value, field.type, field.unit, bill.currency)}
            </span>
          </span>
        </div>
      </div>

      <span
        className={low ? 'tabular text-xs font-medium text-danger' : 'tabular text-xs text-ink-muted'}
      >
        {formatFraction(field.confidence)}
      </span>

      {identical ? (
        <span className="text-xs text-ink-muted">{t.ocr.identical}</span>
      ) : (
        <Button
          size="sm"
          variant={accepted ? 'primary' : 'secondary'}
          aria-pressed={accepted}
          onClick={onAccept}
        >
          {accepted ? <Check aria-hidden /> : null}
          {accepted ? t.ocr.accepted : t.ocr.accept}
        </Button>
      )}
    </li>
  )
}

/**
 * Traduce i campi accettati in un `PATCH` della bolletta.
 *
 * Il **consumo è un oggetto solo**, valore e unità insieme: l'API azzera entrambe
 * le colonne se lo si azzera, e mandarne metà non è rappresentabile. Per questo
 * si accetta la proposta intera e non i due pezzi separatamente — che è anche il
 * modo in cui l'API la manda.
 */
function patchFromAccepted(accepted: Record<string, ExtractedField>): Record<string, unknown> {
  const patch: Record<string, unknown> = {}
  for (const [name, field] of Object.entries(accepted)) {
    if (name === 'consumption') {
      patch['consumption'] = { value: field.value, unit: field.unit ?? '' }
      continue
    }
    patch[name] = field.value
  }
  return patch
}
