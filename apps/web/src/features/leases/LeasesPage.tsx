import { Plus } from 'lucide-react'
import { Link, useSearchParams } from 'react-router'

import { listLeases } from '@/api/endpoints'
import { Async, Empty } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Select } from '@/components/ui/field'
import { PageHeader } from '@/components/ui/page'
import { TableCaption, TableFrame, Td, TdNumeric, Th, ThNumeric, Tr } from '@/components/ui/table'
import { keys } from '@/app/query'
import { useApi } from '@/app/session'
import { t } from '@/i18n'
import { formatAmount, formatDate } from '@/lib/format'
import { LoadMore, usePagedQuery } from '../shared/pagination'
import { LeaseStatusBadge } from '../shared/status'

/**
 * **Solo gli stati memorizzati.**
 *
 * `expiring` ed `expired` esistono in `effective_status` e non in database:
 * filtrarli risponde `422` con `lease.derived_status_filter`. Un client che
 * costruisse questo elenco dai valori dell'enum ci finirebbe dentro, ed è la
 * ragione per cui l'elenco è scritto a mano e non derivato.
 */
const STATUSES = ['draft', 'pending_signature', 'active', 'terminated', 'cancelled'] as const

export function LeasesPage() {
  const client = useApi()
  const [params, setParams] = useSearchParams()
  const status = params.get('stato') ?? ''
  const inForce = params.get('inEssere') === '1'

  const query = usePagedQuery({
    queryKey: keys.leases({ status, inForce }),
    fetchPage: (after) =>
      listLeases(client, {
        status: (status || undefined) as (typeof STATUSES)[number] | undefined,
        in_force: inForce ? true : undefined,
        after,
      }),
  })

  function setParam(key: string, value: string | null) {
    const next = new URLSearchParams(params)
    if (value) next.set(key, value)
    else next.delete(key)
    setParams(next, { replace: true })
  }

  return (
    <>
      <PageHeader
        title={t.lease.title}
        description={t.lease.derivedStatusNote}
        actions={
          <Button asChild variant="primary">
            <Link to="/contratti/nuovo">
              <Plus aria-hidden />
              {t.lease.create}
            </Link>
          </Button>
        }
      />

      <Card>
        <div className="flex flex-wrap items-center gap-4 border-b border-border-subtle px-4 py-3">
          <div className="flex items-center gap-2">
            <label htmlFor="filtro-stato-contratto" className="text-xs font-medium text-ink-muted">
              {t.lease.status}
            </label>
            <Select
              id="filtro-stato-contratto"
              className="w-auto min-w-40"
              value={status}
              onChange={(event) => setParam('stato', event.target.value || null)}
            >
              <option value="">{t.common.all}</option>
              {STATUSES.map((value) => (
                <option key={value} value={value}>
                  {t.lease.statuses[value]}
                </option>
              ))}
            </Select>
          </div>
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              className="size-4"
              checked={inForce}
              onChange={(event) => setParam('inEssere', event.target.checked ? '1' : null)}
            />
            {t.lease.inForce}
          </label>
        </div>

        <Async
          query={query}
          isEmpty={(items) => items.length === 0}
          empty={
            <Empty
              title={t.lease.empty}
              hint={t.lease.emptyHint}
              action={
                <Button asChild variant="primary">
                  <Link to="/contratti/nuovo">{t.lease.create}</Link>
                </Button>
              }
            />
          }
        >
          {(items) => (
            <TableFrame>
              <TableCaption>{t.lease.title}</TableCaption>
              <thead>
                <tr>
                  <Th>{t.lease.type}</Th>
                  <Th>{t.lease.status}</Th>
                  <Th>{t.lease.startDate}</Th>
                  <Th>{t.lease.endDate}</Th>
                  <ThNumeric>{t.lease.rentAmount}</ThNumeric>
                </tr>
              </thead>
              <tbody>
                {items.map((lease) => (
                  <Tr key={lease.id}>
                    <Td>
                      <Link
                        to={`/contratti/${lease.id}`}
                        className="font-medium text-accent underline-offset-4 hover:underline"
                      >
                        {t.lease.types[lease.lease_type]}
                      </Link>
                    </Td>
                    <Td>
                      <LeaseStatusBadge
                        status={lease.status}
                        effectiveStatus={lease.effective_status}
                      />
                    </Td>
                    <Td>{formatDate(lease.start_date)}</Td>
                    <Td>{formatDate(lease.end_date)}</Td>
                    <TdNumeric>
                      {formatAmount(lease.rent_amount, { currency: lease.currency })}
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
