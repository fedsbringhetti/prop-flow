import { useQueryClient } from '@tanstack/react-query'
import { Ban, Plus } from 'lucide-react'
import { Link, useSearchParams } from 'react-router'

import { listBills, voidBill } from '@/api/endpoints'
import type { BillStatus, UtilityBill, UtilityType } from '@/api/types'
import { Async, Empty } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Select } from '@/components/ui/field'
import { PageHeader } from '@/components/ui/page'
import {
  TableCaption,
  TableFrame,
  Td,
  TdNumeric,
  TdSelect,
  Th,
  ThNumeric,
  ThSelect,
  Tr,
  useSelection,
} from '@/components/ui/table'
import { keys } from '@/app/query'
import { useApi, useIsStaff } from '@/app/session'
import { t } from '@/i18n'
import { formatAmount, formatDate } from '@/lib/format'
import { BulkBar, useBulkAction, VerbButton, type Verb } from '../shared/bulk'
import { LoadMore, usePagedQuery } from '../shared/pagination'
import { BillStatusBadge } from '../shared/status'

/**
 * Gli stati che una bolletta può davvero avere.
 *
 * **`disputed` e `paid` non ci sono**, e non è una dimenticanza: nessuna
 * transizione li scrive (`BillStatus` nel dominio li porta come «mai scritto
 * dall'applicazione»), quindi quelle opzioni non potevano corrispondere a niente —
 * è la forma di `R26`. Contestare una bolletta non è uno stato ma un processo, con
 * una ragione, una data e un esito, e offrirlo come etichetta darebbe meno di
 * quanto chi lo usa si aspetta; «pagata» è una **data**, `paid_at`, che la sezione 4
 * ha dato alla bolletta al posto dello stato. `void` resta fuori per un'altra
 * ragione: una bolletta annullata sparisce dagli elenchi.
 */
const STATUSES: BillStatus[] = ['draft', 'pending_review', 'validated', 'allocated']

const TYPES: UtilityType[] = [
  'electricity',
  'gas',
  'water',
  'heating',
  'waste',
  'internet',
  'condo_fees',
  'other',
]

/**
 * Il verbo di questa tabella è **Annulla**: una bolletta annullata sparisce
 * dagli elenchi e libera il numero di fattura, e la conferma della scheda lo
 * dice. Nessun «Elimina».
 */
const VOID: Verb = {
  label: t.bill.void,
  confirmOne: t.bill.voidConfirm,
  confirmMany: t.bill.voidConfirmMany,
  explain: t.bill.voidExplain,
}

/**
 * Si annulla finché la bolletta è **modificabile**: `draft`, `pending_review`,
 * `validated` — è `BillStatus::is_editable` nel dominio. Da `allocated` in poi è
 * la base di un addebito già emesso, e `paid` è un fatto contabile: l'API
 * risponderebbe `409 utility_bill.invalid_transition`, e la riga non si offre.
 */
function voidable(bill: UtilityBill): boolean {
  return bill.status === 'draft' || bill.status === 'pending_review' || bill.status === 'validated'
}

export function BillsPage() {
  const client = useApi()
  const isStaff = useIsStaff()
  const queryClient = useQueryClient()
  const [params, setParams] = useSearchParams()
  const status = (params.get('stato') ?? '') as BillStatus | ''
  const utilityType = (params.get('utenza') ?? '') as UtilityType | ''

  const query = usePagedQuery({
    queryKey: keys.bills({ status, utilityType }),
    fetchPage: (after) =>
      listBills(client, {
        status: status || undefined,
        utility_type: utilityType || undefined,
        after,
      }),
  })

  // Annullare è dello staff: l'inquilino legge, e non ha la colonna.
  const selection = useSelection(
    isStaff ? (query.data ?? []).filter(voidable).map((bill) => bill.id) : [],
  )
  const discard = useBulkAction({
    selection,
    action: (id) => voidBill(client, id),
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: ['bills'] })
      await queryClient.invalidateQueries({ queryKey: keys.dashboard })
    },
  })

  function setParam(key: string, value: string) {
    const next = new URLSearchParams(params)
    if (value) next.set(key, value)
    else next.delete(key)
    setParams(next, { replace: true })
  }

  return (
    <>
      <PageHeader
        title={t.bill.title}
        actions={
          /* Registrare una bolletta è dello staff. L'inquilino le **legge**, ed è
             la ragione per cui la voce resta nella sua navigazione: sono la
             ragione di metà dei suoi addebiti. */
          isStaff ? (
            <Button asChild variant="primary">
              <Link to="/bollette/nuova">
                <Plus aria-hidden />
                {t.bill.create}
              </Link>
            </Button>
          ) : null
        }
      />

      <Card>
        <div className="flex flex-wrap items-center gap-4 border-b border-border-subtle px-4 py-3">
          <div className="flex items-center gap-2">
            <label htmlFor="filtro-stato-bolletta" className="text-xs font-medium text-ink-muted">
              {t.bill.status}
            </label>
            <Select
              id="filtro-stato-bolletta"
              className="w-auto min-w-40"
              value={status}
              onChange={(event) => setParam('stato', event.target.value)}
            >
              <option value="">{t.common.all}</option>
              {STATUSES.map((value) => (
                <option key={value} value={value}>
                  {t.bill.statuses[value]}
                </option>
              ))}
            </Select>
          </div>
          <div className="flex items-center gap-2">
            <label htmlFor="filtro-utenza" className="text-xs font-medium text-ink-muted">
              {t.bill.utilityType}
            </label>
            <Select
              id="filtro-utenza"
              className="w-auto min-w-40"
              value={utilityType}
              onChange={(event) => setParam('utenza', event.target.value)}
            >
              <option value="">{t.common.all}</option>
              {TYPES.map((value) => (
                <option key={value} value={value}>
                  {t.bill.types[value]}
                </option>
              ))}
            </Select>
          </div>
        </div>

        {isStaff ? (
          <BulkBar selection={selection} verb={VOID} bulk={discard} icon={<Ban aria-hidden />} />
        ) : null}

        <Async
          query={query}
          isEmpty={(items) => items.length === 0}
          empty={
            <Empty
              title={status || utilityType ? t.state.filteredTitle : t.bill.empty}
              hint={
                status || utilityType
                  ? t.state.filteredHint
                  : isStaff
                    ? t.bill.emptyHint
                    : t.bill.emptyForTenant
              }
              {...(isStaff && !status && !utilityType
                ? {
                    action: (
                      <Button asChild variant="primary">
                        <Link to="/bollette/nuova">{t.bill.create}</Link>
                      </Button>
                    ),
                  }
                : {})}
            />
          }
        >
          {(items) => (
            <TableFrame>
              <TableCaption>{t.bill.title}</TableCaption>
              <thead>
                <tr>
                  {isStaff ? <ThSelect selection={selection} /> : null}
                  <Th>{t.bill.utilityType}</Th>
                  <Th>{t.bill.supplierName}</Th>
                  <Th>{t.bill.status}</Th>
                  <Th>{t.bill.periodStart}</Th>
                  <ThNumeric>{t.bill.totalAmount}</ThNumeric>
                  {isStaff ? (
                    <Th>
                      <span className="sr-only">{t.common.actions}</span>
                    </Th>
                  ) : null}
                </tr>
              </thead>
              <tbody>
                {items.map((bill) => (
                  <Tr key={bill.id}>
                    {isStaff ? (
                      <TdSelect
                        selection={selection}
                        id={bill.id}
                        label={`${t.bill.types[bill.utility_type]} ${bill.invoice_number ?? formatDate(bill.period_start)}`}
                        failure={discard.failureFor(bill.id)}
                        disabled={!voidable(bill)}
                      />
                    ) : null}
                    <Td>
                      <Link
                        to={`/bollette/${bill.id}`}
                        className="font-medium text-accent underline-offset-4 hover:underline"
                      >
                        {t.bill.types[bill.utility_type]}
                      </Link>
                      {bill.invoice_number ? (
                        <span className="block font-numeric text-xs text-ink-muted">
                          {bill.invoice_number}
                        </span>
                      ) : null}
                    </Td>
                    <Td>{bill.supplier_name ?? t.common.none}</Td>
                    <Td>
                      <BillStatusBadge status={bill.status} />
                    </Td>
                    <Td>
                      {formatDate(bill.period_start)} — {formatDate(bill.period_end)}
                    </Td>
                    <TdNumeric>
                      {formatAmount(bill.total_amount, { currency: bill.currency })}
                    </TdNumeric>
                    {isStaff ? (
                      <Td className="text-right whitespace-nowrap">
                        {voidable(bill) ? (
                          <VerbButton
                            verb={VOID}
                            ids={[bill.id]}
                            run={discard.run}
                            rowLabel={`${t.bill.types[bill.utility_type]} ${bill.invoice_number ?? formatDate(bill.period_start)}`}
                          />
                        ) : null}
                      </Td>
                    ) : null}
                  </Tr>
                ))}
              </tbody>
            </TableFrame>
          )}
        </Async>

        <LoadMore
          hasNextPage={query.hasNextPage}
          isFetchingNextPage={query.isFetchingNextPage}
          fetchNextPage={() => void query.fetchNextPage()}
        />
      </Card>
    </>
  )
}
