import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Ban, HandCoins } from 'lucide-react'
import { Link, useParams } from 'react-router'

import { getCharge, listChargePayments, voidCharge, writeOffCharge } from '@/api/endpoints'
import type { Charge, Payment } from '@/api/types'
import { Async, Empty } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card, CardBody, CardHeader, CardTitle } from '@/components/ui/card'
import { ConfirmAction } from '@/components/ui/confirm'
import { DescriptionItem, DescriptionList, PageHeader, PageSection } from '@/components/ui/page'
import { TableCaption, TableFrame, Td, TdNumeric, Th, ThNumeric, Tr } from '@/components/ui/table'
import { keys } from '@/app/query'
import { useApi, useIsStaff } from '@/app/session'
import { t } from '@/i18n'
import { isNegative } from '@/lib/decimal'
import { formatAmount, formatDate, formatDateTime } from '@/lib/format'
import { RecordPaymentDialog } from '../payments/RecordPaymentDialog'
import { LoadMore, usePagedQuery } from '../shared/pagination'
import { ChargeStatusBadge, PaymentStatusBadge } from '../shared/status'

/**
 * La scheda di un addebito: quanto è dovuto, che cosa è entrato, e le due uscite.
 *
 * È da qui che si registra un incasso, ed è la ragione per cui questa schermata
 * esiste: prima di questa fetta un addebito si poteva solo annullare o stornare,
 * quindi l'insoluto saliva e non scendeva mai.
 *
 * # Le due uscite non sono simmetriche
 *
 * **L'annullamento sparisce quando è entrato del denaro.** `void` significa «non
 * doveva essere chiesto», e su un addebito parzialmente pagato sarebbe falso:
 * qualcuno ha versato qualcosa contro quella richiesta. L'API risponde `409`, e
 * qui il bottone non compare affatto — con al suo posto la frase che dice perché e
 * dove andare. Un bottone che produce sempre un errore è peggio di un bottone
 * assente.
 *
 * **Lo storno resta**, anche da parzialmente pagato: l'incassato resta incassato e
 * si rinuncia al residuo. È il caso in cui una perdita si materializza davvero.
 */
export function ChargeDetailPage() {
  const { id = '' } = useParams()
  const client = useApi()
  const isStaff = useIsStaff()
  const queryClient = useQueryClient()

  const charge = useQuery({ queryKey: keys.charge(id), queryFn: () => getCharge(client, id) })

  async function refresh() {
    await queryClient.invalidateQueries({ queryKey: keys.charge(id) })
    await queryClient.invalidateQueries({ queryKey: ['charges'] })
    await queryClient.invalidateQueries({ queryKey: keys.dashboard })
  }

  const discard = useMutation({ mutationFn: () => voidCharge(client, id), onSuccess: refresh })
  const writeOff = useMutation({
    mutationFn: () => writeOffCharge(client, id),
    onSuccess: refresh,
  })

  return (
    <Async query={charge}>
      {(data) => {
        const settled = data.status === 'void' || data.status === 'written_off'
        const partlyPaid = data.status === 'partially_paid'
        // Un residuo negativo è un **credito** verso l'inquilino: la vista lo
        // esclude dall'insoluto (`WHERE outstanding_amount > 0`), quindi non
        // compare nel riepilogo. Se non si vedesse qui non si vedrebbe da nessuna
        // parte.
        const credit = isNegative(data.outstanding_amount)

        return (
          <>
            <PageHeader
              title={t.charge.types[data.charge_type]}
              description={data.description ?? undefined}
              breadcrumb={
                <Link to="/addebiti" className="hover:underline">
                  {t.charge.backToList}
                </Link>
              }
              actions={
                <>
                  {settled || data.status === 'paid' ? null : isStaff ? (
                    <RecordPaymentDialog charge={data} />
                  ) : null}
                  {!isStaff || settled || data.status === 'paid' ? null : (
                    <ConfirmAction
                      trigger={
                        <Button variant="dangerGhost">
                          <HandCoins aria-hidden />
                          {t.charge.writeOff}
                        </Button>
                      }
                      title={t.charge.writeOffConfirm}
                      explanation={t.charge.writeOffExplain}
                      confirmLabel={t.charge.writeOff}
                      onConfirm={() => writeOff.mutateAsync()}
                    />
                  )}
                  {!isStaff || settled || partlyPaid || data.status === 'paid' ? null : (
                    <ConfirmAction
                      trigger={
                        <Button variant="dangerGhost">
                          <Ban aria-hidden />
                          {t.charge.void}
                        </Button>
                      }
                      title={t.charge.voidConfirm}
                      explanation={t.charge.voidExplain}
                      confirmLabel={t.charge.void}
                      onConfirm={() => discard.mutateAsync()}
                    />
                  )}
                </>
              }
            />

            <PageSection>
              <Card>
                <CardHeader>
                  <CardTitle>{t.charge.detailTitle}</CardTitle>
                  <ChargeStatusBadge
                    status={data.status}
                    effectiveStatus={data.effective_status}
                  />
                </CardHeader>
                <CardBody className="flex flex-col gap-4">
                  <DescriptionList>
                    <DescriptionItem label={t.charge.amount} numeric>
                      {formatAmount(data.amount, { currency: data.currency })}
                    </DescriptionItem>
                    <DescriptionItem label={t.charge.paidAmount} numeric>
                      {formatAmount(data.paid_amount, { currency: data.currency })}
                    </DescriptionItem>
                    <DescriptionItem
                      label={credit ? t.charge.credit : t.charge.outstandingAmount}
                      numeric
                    >
                      <span className={credit ? 'font-medium text-success' : undefined}>
                        {formatAmount(data.outstanding_amount, { currency: data.currency })}
                      </span>
                    </DescriptionItem>
                    <DescriptionItem label={t.charge.dueDate}>
                      {formatDate(data.due_date)}
                    </DescriptionItem>
                    <DescriptionItem label={t.charge.lease}>
                      <Link
                        to={`/contratti/${data.lease_id}`}
                        className="text-accent underline-offset-4 hover:underline"
                      >
                        {t.lease.title}
                      </Link>
                    </DescriptionItem>
                    <DescriptionItem label={t.charge.issuedAt}>
                      {data.issued_at ? formatDate(data.issued_at) : t.common.none}
                    </DescriptionItem>
                  </DescriptionList>

                  {credit ? (
                    <p className="rounded-lg border border-success/30 bg-success-soft px-3 py-2 text-xs text-success">
                      {t.charge.creditExplain}
                    </p>
                  ) : null}

                  {isStaff && partlyPaid ? (
                    <p className="rounded-lg bg-surface-muted px-3 py-2 text-xs text-ink-muted">
                      {t.charge.voidNotWhenPartlyPaid}
                    </p>
                  ) : null}
                </CardBody>
              </Card>
            </PageSection>

            <PageSection>
              <ChargePayments charge={data} />
            </PageSection>
          </>
        )
      }}
    </Async>
  )
}

/** I movimenti dell'addebito: quello che è entrato, e quello che aspetta. */
function ChargePayments({ charge }: { charge: Charge }) {
  const client = useApi()

  const query = usePagedQuery({
    queryKey: keys.chargePayments(charge.id),
    fetchPage: (after) => listChargePayments(client, charge.id, after),
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t.payment.chargePayments}</CardTitle>
      </CardHeader>

      <Async
        query={query}
        isEmpty={(items) => items.length === 0}
        empty={<Empty title={t.payment.empty} />}
      >
        {(items) => (
          <TableFrame>
            <TableCaption>{t.payment.chargePayments}</TableCaption>
            <thead>
              <tr>
                <Th>{t.payment.declaredAt}</Th>
                <Th>{t.payment.movedAt}</Th>
                <Th>{t.payment.status}</Th>
                <Th>{t.payment.method}</Th>
                <Th>{t.payment.reference}</Th>
                <ThNumeric>{t.payment.amount}</ThNumeric>
              </tr>
            </thead>
            <tbody>
              {items.map((payment) => (
                <PaymentRow key={payment.id} payment={payment} />
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
  )
}

function PaymentRow({ payment }: { payment: Payment }) {
  return (
    <Tr>
      <Td>{formatDateTime(payment.created_at)}</Td>
      {/* `paid_at` manca finché nessuno ha detto quando: una dichiarazione può
          non portarlo, e il rifiuto lo azzera. Il trattino è la verità. */}
      <Td>{payment.paid_at ? formatDateTime(payment.paid_at) : t.common.none}</Td>
      <Td>
        <PaymentStatusBadge status={payment.status} />
        {payment.status === 'pending' ? (
          <span className="mt-0.5 block text-xs text-ink-muted">
            {t.payment.pendingDoesNotMove}
          </span>
        ) : null}
      </Td>
      <Td>{t.payment.methods[payment.method]}</Td>
      <Td className="text-ink-muted">{payment.reference ?? t.common.none}</Td>
      <TdNumeric>{formatAmount(payment.amount, { currency: payment.currency })}</TdNumeric>
    </Tr>
  )
}
