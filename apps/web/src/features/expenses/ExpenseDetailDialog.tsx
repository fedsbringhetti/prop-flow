import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'

import { getExpense } from '@/api/endpoints'
import type { Expense } from '@/api/types'
import { Async } from '@/components/state'
import { Button } from '@/components/ui/button'
import { FormDialog } from '@/components/ui/confirm'
import { DescriptionItem, DescriptionList } from '@/components/ui/page'
import { keys } from '@/app/query'
import { useApi } from '@/app/session'
import { t } from '@/i18n'
import { formatAmount, formatDate, formatDecimal } from '@/lib/format'
import type { NameOf } from './names'

/**
 * Il dettaglio di una spesa, che è **dove stanno le quote**.
 *
 * L'elenco non le porta — `ExpenseResponse.shares` è assente lì e presente qui —
 * quindi la modale fa la sua richiesta, e solo quando si apre.
 *
 * # Le quote si mostrano, non si ricalcolano
 *
 * La loro somma è **esattamente** l'importo: lo garantisce il trigger
 * `expense_shares_balanced`, e il centesimo di resto va ai primi in ordine di
 * identificatore. Un client che le ridividesse otterrebbe numeri leggermente
 * diversi da quelli su cui il gruppo sta facendo i conti.
 *
 * # `share_weight` è **nullo** su una divisione in parti uguali
 *
 * Il peso non ha partecipato al calcolo, e mostrarlo come `1` sarebbe inventare un
 * dato: la colonna compare solo quando la ripartizione è a peso.
 */
export function ExpenseDetailDialog({ expense, nameOf }: { expense: Expense; nameOf: NameOf }) {
  const client = useApi()
  const [open, setOpen] = useState(false)

  const detail = useQuery({
    queryKey: keys.expense(expense.id),
    queryFn: () => getExpense(client, expense.id),
    enabled: open,
  })

  return (
    <FormDialog
      trigger={
        <Button size="sm" variant="link">
          {t.expense.shares}
        </Button>
      }
      title={t.expense.expenseDetail}
      description={expense.description}
      open={open}
      onOpenChange={setOpen}
    >
      <Async query={detail}>
        {(data) => (
          <div className="flex flex-col gap-4">
            <DescriptionList>
              <DescriptionItem label={t.expense.amount} numeric>
                {formatAmount(data.amount, { currency: data.currency })}
              </DescriptionItem>
              <DescriptionItem label={t.expense.expenseDate}>
                {formatDate(data.expense_date)}
              </DescriptionItem>
              <DescriptionItem label={t.expense.paidBy}>
                {nameOf(data.paid_by_user_id)}
              </DescriptionItem>
              <DescriptionItem label={t.expense.splitMethod}>
                {data.split_method === 'equal' || data.split_method === 'weighted'
                  ? t.expense.splitMethods[data.split_method]
                  : data.split_method}
              </DescriptionItem>
              {data.notes ? (
                <DescriptionItem label={t.expense.notes}>{data.notes}</DescriptionItem>
              ) : null}
            </DescriptionList>

            {/* Un elenco e non una tabella, e non è una scelta di gusto: una
                tabella con la larghezza minima che regge il resto dell'interfaccia
                scorrerebbe in orizzontale dentro una modale larga la metà, e a
                finire fuori sarebbe **la colonna degli importi** — cioè la sola
                ragione per cui si apre questo dettaglio. */}
            <ul className="divide-y divide-border-subtle rounded-card border border-border-subtle">
              <li className="px-3 py-1.5 text-xs font-medium tracking-wide text-ink-muted uppercase">
                {t.expense.shares}
              </li>
              {(data.shares ?? []).map((share) => (
                <li
                  key={share.user_id}
                  className="flex flex-wrap items-center justify-between gap-2 px-3 py-2 text-sm"
                >
                  <span className="min-w-0">
                    {nameOf(share.user_id)}
                    {/* `share_weight` è **nullo** su una divisione in parti uguali:
                        il peso non ha partecipato al calcolo, e mostrarlo come `1`
                        sarebbe inventare un dato. */}
                    {share.share_weight ? (
                      <span className="ml-2 text-xs text-ink-muted">
                        {t.expense.shareWeight}:{' '}
                        {formatDecimal(share.share_weight, { maximumFractionDigits: 3 })}
                      </span>
                    ) : null}
                  </span>
                  <span className="tabular font-medium">
                    {formatAmount(share.share_amount, { currency: data.currency })}
                  </span>
                </li>
              ))}
            </ul>

            <p className="text-xs text-ink-muted">{t.expense.sharesSum}</p>
          </div>
        )}
      </Async>
    </FormDialog>
  )
}
