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
import { useApi, useIsStaff } from '@/app/session'
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
  const isStaff = useIsStaff()
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
          /* Redigere un contratto è dello staff: `DraftLease` è `ensure_staff`. */
          isStaff ? (
            <Button asChild variant="primary">
              <Link to="/contratti/nuovo">
                <Plus aria-hidden />
                {t.lease.create}
              </Link>
            </Button>
          ) : null
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
          {/* **Dove nasce la domanda.** Il badge mostra lo stato derivato, quindi
              chi chiede «attivo» può vedersi tornare una riga marcata «Scaduto»:
              la spiegazione sta accanto al filtro che l'ha prodotta, e compare
              solo quando un filtro è attivo. Prima viveva fra parentesi su ogni
              riga, che è `R30`. */}
          {status ? (
            <p className="text-xs text-ink-muted">{t.lease.filteredByDeclaredStatus}</p>
          ) : null}
        </div>

        <Async
          query={query}
          isEmpty={(items) => items.length === 0}
          empty={
            <Empty
              title={t.lease.empty}
              hint={isStaff ? t.lease.emptyHint : t.lease.emptyForTenant}
              {...(isStaff
                ? {
                    action: (
                      <Button asChild variant="primary">
                        <Link to="/contratti/nuovo">{t.lease.create}</Link>
                      </Button>
                    ),
                  }
                : {})}
            />
          }
        >
          {(items) => (
            <TableFrame>
              <TableCaption>{t.lease.title}</TableCaption>
              <thead>
                <tr>
                  <Th>{t.lease.property}</Th>
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
                    {/* **Il nome di ciò che si affitta, non il tipo.** Tipo più
                        date rendeva ogni riga uguale all'altra, che è il difetto
                        per cui questa colonna esiste. La stanza sta sotto perché
                        distingue due contratti nella stessa casa. */}
                    <Td>
                      <Link
                        to={`/contratti/${lease.id}`}
                        className="font-medium text-accent underline-offset-4 hover:underline"
                      >
                        {lease.property_label ?? t.lease.unknownProperty}
                      </Link>
                      {lease.room_name ? (
                        <span className="block text-xs text-ink-muted">{lease.room_name}</span>
                      ) : null}
                    </Td>
                    <Td>{t.lease.types[lease.lease_type]}</Td>
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
