import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Link, useSearchParams } from 'react-router'

import { listCharges, voidCharge, writeOffCharge } from '@/api/endpoints'
import type { Charge } from '@/api/types'
import { Async, Empty } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { ConfirmAction } from '@/components/ui/confirm'
import { PageHeader } from '@/components/ui/page'
import { TableCaption, TableFrame, Td, TdNumeric, Th, ThNumeric, Tr } from '@/components/ui/table'
import { keys } from '@/app/query'
import { useApi } from '@/app/session'
import { t } from '@/i18n'
import { isNegative } from '@/lib/decimal'
import { formatAmount, formatDate } from '@/lib/format'
import { LoadMore, usePagedQuery } from '../shared/pagination'
import { ChargeStatusBadge } from '../shared/status'

/**
 * Lo scadenzario.
 *
 * **Non c'è un filtro «scaduto»**, ed è deliberato: `overdue` è derivato dalla
 * scadenza e `?status=overdue` risponde `422` con `charge.derived_status_filter`.
 * Il filtro che serve è `?outstanding=true`, che enumera i tre stati dell'insoluto
 * e usa l'indice parziale che esiste per quello.
 */
export function ChargesPage() {
  const client = useApi()
  const [params, setParams] = useSearchParams()
  const outstanding = params.get('insoluti') === '1'

  const query = usePagedQuery({
    queryKey: keys.charges({ outstanding }),
    fetchPage: (after) =>
      listCharges(client, { outstanding: outstanding ? true : undefined, after }),
  })

  return (
    <>
      <PageHeader title={t.charge.title} description={t.charge.derivedStatusNote} />

      <Card>
        <div className="flex flex-wrap items-center gap-4 border-b border-border-subtle px-4 py-3">
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              className="size-4"
              checked={outstanding}
              onChange={(event) => {
                const next = new URLSearchParams(params)
                if (event.target.checked) next.set('insoluti', '1')
                else next.delete('insoluti')
                setParams(next, { replace: true })
              }}
            />
            {t.charge.outstandingOnly}
          </label>
        </div>

        <Async
          query={query}
          isEmpty={(items) => items.length === 0}
          empty={
            <Empty title={outstanding ? t.charge.emptyOutstanding : t.charge.empty} />
          }
        >
          {(items) => (
            <TableFrame>
              <TableCaption>{t.charge.title}</TableCaption>
              <thead>
                <tr>
                  <Th>{t.charge.type}</Th>
                  <Th>{t.charge.status}</Th>
                  <Th>{t.charge.dueDate}</Th>
                  <ThNumeric>{t.charge.amount}</ThNumeric>
                  <ThNumeric>{t.charge.paidAmount}</ThNumeric>
                  <ThNumeric>{t.charge.outstandingAmount}</ThNumeric>
                  <Th>
                    <span className="sr-only">{t.common.actions}</span>
                  </Th>
                </tr>
              </thead>
              <tbody>
                {items.map((charge) => (
                  <ChargeRow key={charge.id} charge={charge} />
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

function ChargeRow({ charge }: { charge: Charge }) {
  const client = useApi()
  const queryClient = useQueryClient()

  async function refresh() {
    await queryClient.invalidateQueries({ queryKey: ['charges'] })
    await queryClient.invalidateQueries({ queryKey: keys.dashboard })
  }

  const discard = useMutation({ mutationFn: () => voidCharge(client, charge.id), onSuccess: refresh })
  const writeOff = useMutation({
    mutationFn: () => writeOffCharge(client, charge.id),
    onSuccess: refresh,
  })

  const closed = charge.status === 'void' || charge.status === 'written_off' || charge.status === 'paid'
  // Un residuo **negativo** è un credito verso l'inquilino: non compensa gli
  // altri insoluti e non compare nella dashboard, quindi almeno qui va detto.
  const credit = isNegative(charge.outstanding_amount)

  return (
    <Tr>
      <Td>
        <Link
          to={`/contratti/${charge.lease_id}`}
          className="font-medium text-accent underline-offset-4 hover:underline"
        >
          {t.charge.types[charge.charge_type]}
        </Link>
        {charge.utility_bill_id ? (
          <Link
            to={`/bollette/${charge.utility_bill_id}`}
            className="block text-xs text-ink-muted underline-offset-4 hover:underline"
          >
            {t.charge.fromBill}
          </Link>
        ) : null}
      </Td>
      <Td>
        <ChargeStatusBadge status={charge.status} effectiveStatus={charge.effective_status} />
      </Td>
      <Td>{formatDate(charge.due_date)}</Td>
      <TdNumeric>{formatAmount(charge.amount, { currency: charge.currency })}</TdNumeric>
      <TdNumeric>{formatAmount(charge.paid_amount, { currency: charge.currency })}</TdNumeric>
      <TdNumeric className={credit ? 'text-success' : undefined}>
        {formatAmount(charge.outstanding_amount, { currency: charge.currency })}
      </TdNumeric>
      <Td className="text-right whitespace-nowrap">
        {closed ? null : (
          <span className="inline-flex gap-1">
            <ConfirmAction
              trigger={
                <Button size="sm" variant="ghost">
                  {t.charge.void}
                </Button>
              }
              title={t.charge.voidConfirm}
              explanation={t.charge.voidExplain}
              confirmLabel={t.charge.void}
              onConfirm={() => discard.mutateAsync()}
            />
            <ConfirmAction
              trigger={
                <Button size="sm" variant="ghost">
                  {t.charge.writeOff}
                </Button>
              }
              title={t.charge.writeOffConfirm}
              explanation={t.charge.writeOffExplain}
              confirmLabel={t.charge.writeOff}
              onConfirm={() => writeOff.mutateAsync()}
            />
          </span>
        )}
      </Td>
    </Tr>
  )
}
