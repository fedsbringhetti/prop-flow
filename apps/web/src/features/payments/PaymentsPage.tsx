import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Check, X } from 'lucide-react'
import { Link, useSearchParams } from 'react-router'

import { confirmPayment, listPayments, rejectPayment } from '@/api/endpoints'
import type { Payment, PaymentStatus } from '@/api/types'
import { Async, Empty } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { ConfirmAction } from '@/components/ui/confirm'
import { PageHeader } from '@/components/ui/page'
import { TableCaption, TableFrame, Td, TdNumeric, Th, ThNumeric, Tr } from '@/components/ui/table'
import { keys } from '@/app/query'
import { useApi } from '@/app/session'
import { t } from '@/i18n'
import { formatAmount, formatDateTime } from '@/lib/format'
import { LoadMore, usePagedQuery } from '../shared/pagination'
import { PaymentStatusBadge } from '../shared/status'

/**
 * La coda delle dichiarazioni.
 *
 * # Perché è una schermata e non una riga in fondo all'addebito
 *
 * La domanda che pone — *chi dice di aver pagato, in tutta l'organizzazione* — non
 * si può fare dall'addebito, perché per farla bisognerebbe già sapere **quale**
 * addebito guardare. Senza questa schermata una dichiarazione si scoprirebbe
 * aprendo gli addebiti a uno a uno, ed è la ragione per cui
 * `GET /api/v1/payments` esiste.
 *
 * # Il filtro è nell'URL
 *
 * Come per lo scadenzario: la coda è un indirizzo che si può salvare e mandare a
 * qualcuno, non uno stato che vive solo dentro il componente. **In attesa è il
 * default**, perché è la ragione per cui si apre questa pagina; togliere il filtro
 * mostra ogni movimento.
 *
 * # Che cosa muove il saldo, detto dove si agisce
 *
 * Una dichiarazione `pending` **non** ha mosso il saldo: renderla visibile senza
 * renderla vera è precisamente il suo scopo. Ne segue che confermarla lo fa
 * scendere, e che **rifiutarla non lo fa risalire** — non era mai sceso. Un
 * messaggio che dicesse «saldo aggiornato» dopo un rifiuto direbbe il falso, ed è
 * il motivo per cui le due conferme spiegano cose diverse.
 *
 * # Chi ha dichiarato non può ritirare, e qui non glielo si offre
 *
 * `payments_update` è staff soltanto: un bottone «ritira» otterrebbe zero righe
 * aggiornate, cioè `auth.denied_by_policy` — il codice riservato ai nostri
 * difetti. L'uscita da `pending` è il rifiuto, ed è dello staff.
 */
export function PaymentsPage() {
  const client = useApi()
  const [params, setParams] = useSearchParams()
  // Assente significa la coda: è il default della schermata, non l'elenco intero.
  const showAll = params.get('tutti') === '1'
  const status: PaymentStatus | undefined = showAll ? undefined : 'pending'

  const query = usePagedQuery({
    queryKey: keys.payments({ status: status ?? null }),
    fetchPage: (after) => listPayments(client, { status, after }),
  })

  return (
    <>
      <PageHeader
        title={showAll ? t.payment.title : t.payment.queueTitle}
        description={t.payment.queueSubtitle}
      />

      <Card>
        <div className="flex flex-wrap items-center gap-4 border-b border-border-subtle px-4 py-3">
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              className="size-4"
              checked={showAll}
              onChange={(event) => {
                const next = new URLSearchParams(params)
                if (event.target.checked) next.set('tutti', '1')
                else next.delete('tutti')
                setParams(next, { replace: true })
              }}
            />
            {t.common.all}
          </label>
          <p className="text-xs text-ink-muted">{t.payment.onlyStaffDecides}</p>
        </div>

        <Async
          query={query}
          isEmpty={(items) => items.length === 0}
          empty={
            <Empty
              title={showAll ? t.payment.empty : t.payment.queueEmpty}
              {...(showAll ? {} : { hint: t.payment.queueEmptyHint })}
            />
          }
        >
          {(items) => (
            <TableFrame>
              <TableCaption>{showAll ? t.payment.title : t.payment.queueTitle}</TableCaption>
              <thead>
                <tr>
                  <Th>{t.payment.declaredAt}</Th>
                  <Th>{t.payment.movedAt}</Th>
                  <Th>{t.payment.status}</Th>
                  <Th>{t.payment.method}</Th>
                  <Th>{t.payment.reference}</Th>
                  <ThNumeric>{t.payment.amount}</ThNumeric>
                  <Th>
                    <span className="sr-only">{t.common.actions}</span>
                  </Th>
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
    </>
  )
}

function PaymentRow({ payment }: { payment: Payment }) {
  const client = useApi()
  const queryClient = useQueryClient()

  async function refresh() {
    await queryClient.invalidateQueries({ queryKey: ['payments'] })
    await queryClient.invalidateQueries({ queryKey: keys.chargePayments(payment.charge_id) })
    await queryClient.invalidateQueries({ queryKey: keys.charge(payment.charge_id) })
    await queryClient.invalidateQueries({ queryKey: ['charges'] })
    await queryClient.invalidateQueries({ queryKey: keys.dashboard })
  }

  const confirm = useMutation({
    mutationFn: () => confirmPayment(client, payment.id),
    onSuccess: refresh,
  })
  const reject = useMutation({
    mutationFn: () => rejectPayment(client, payment.id),
    onSuccess: refresh,
  })

  const waiting = payment.status === 'pending'

  return (
    <Tr>
      <Td>{formatDateTime(payment.created_at)}</Td>
      {/* `paid_at` manca finché nessuno ha detto quando: una dichiarazione può
          non portarlo, e il rifiuto lo azzera. Il trattino è la verità. */}
      <Td>{payment.paid_at ? formatDateTime(payment.paid_at) : t.common.none}</Td>
      <Td>
        <PaymentStatusBadge status={payment.status} />
      </Td>
      <Td>{t.payment.methods[payment.method]}</Td>
      <Td>
        <span className="block text-ink-muted">{payment.reference ?? t.common.none}</span>
        <Link
          to={`/addebiti/${payment.charge_id}`}
          className="text-xs text-accent underline-offset-4 hover:underline"
        >
          {t.payment.goToCharge}
        </Link>
      </Td>
      <TdNumeric>{formatAmount(payment.amount, { currency: payment.currency })}</TdNumeric>
      <Td className="text-right whitespace-nowrap">
        {waiting ? (
          <span className="inline-flex gap-1">
            <ConfirmAction
              trigger={
                <Button size="sm" variant="ghost">
                  <Check aria-hidden />
                  {t.payment.confirm}
                </Button>
              }
              title={t.payment.confirmTitle}
              explanation={t.payment.confirmExplain}
              confirmLabel={t.payment.confirm}
              variant="primary"
              onConfirm={() => confirm.mutateAsync()}
            />
            <ConfirmAction
              trigger={
                <Button size="sm" variant="ghost">
                  <X aria-hidden />
                  {t.payment.reject}
                </Button>
              }
              title={t.payment.rejectTitle}
              explanation={t.payment.rejectExplain}
              confirmLabel={t.payment.reject}
              onConfirm={() => reject.mutateAsync()}
            />
          </span>
        ) : null}
      </Td>
    </Tr>
  )
}
