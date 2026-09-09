import { Plus } from 'lucide-react'
import { Link, useSearchParams } from 'react-router'

import { listBills } from '@/api/endpoints'
import type { BillStatus, UtilityType } from '@/api/types'
import { Async, Empty } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Select } from '@/components/ui/field'
import { PageHeader } from '@/components/ui/page'
import { TableCaption, TableFrame, Td, TdNumeric, Th, ThNumeric, Tr } from '@/components/ui/table'
import { keys } from '@/app/query'
import { useApi, useIsStaff } from '@/app/session'
import { t } from '@/i18n'
import { formatAmount, formatDate } from '@/lib/format'
import { LoadMore, usePagedQuery } from '../shared/pagination'
import { BillStatusBadge } from '../shared/status'

/**
 * Gli stati che una bolletta può davvero avere.
 *
 * **`disputed` non c'è**, e non è una dimenticanza: nessuna transizione lo scrive,
 * quindi quell'opzione non poteva corrispondere a niente — è la forma di `R26`.
 * Contestare una bolletta non è uno stato ma un processo, con una ragione, una
 * data e un esito, e offrirlo come etichetta darebbe meno di quanto chi lo usa si
 * aspetta. `void` resta fuori per un'altra ragione: una bolletta annullata
 * sparisce dagli elenchi.
 */
const STATUSES: BillStatus[] = ['draft', 'pending_review', 'validated', 'allocated', 'paid']

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

export function BillsPage() {
  const client = useApi()
  const isStaff = useIsStaff()
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

        <Async
          query={query}
          isEmpty={(items) => items.length === 0}
          empty={
            <Empty
              title={t.bill.empty}
              hint={isStaff ? t.bill.emptyHint : t.bill.emptyForTenant}
              {...(isStaff
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
                  <Th>{t.bill.utilityType}</Th>
                  <Th>{t.bill.supplierName}</Th>
                  <Th>{t.bill.status}</Th>
                  <Th>{t.bill.periodStart}</Th>
                  <ThNumeric>{t.bill.totalAmount}</ThNumeric>
                </tr>
              </thead>
              <tbody>
                {items.map((bill) => (
                  <Tr key={bill.id}>
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
