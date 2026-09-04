import { useQuery } from '@tanstack/react-query'
import { ArrowUpRight } from 'lucide-react'
import { Link } from 'react-router'

import { getDashboard } from '@/api/endpoints'
import type { Dashboard } from '@/api/types'
import { Async, Empty } from '@/components/state'
import { Button } from '@/components/ui/button'
import { PageHeader } from '@/components/ui/page'
import { keys } from '@/app/query'
import { useApi } from '@/app/session'
import { t } from '@/i18n'
import { formatAmount, formatCount } from '@/lib/format'
import { cn } from '@/lib/utils'

/**
 * I nove numeri, ognuno **collegato al posto dove si agisce**.
 *
 * Un numero che non porta da nessuna parte è una decorazione: dice che c'è un
 * problema e lascia a chi guarda il compito di cercarlo. Qui `overdue_charges`
 * porta allo scadenzario già filtrato e `bills_pending_review` alle bollette da
 * rivedere, così la dashboard è un punto di partenza e non un cruscotto.
 */
export function DashboardPage() {
  const client = useApi()
  const query = useQuery({
    queryKey: keys.dashboard,
    queryFn: () => getDashboard(client),
  })

  return (
    <>
      <PageHeader title={t.dashboard.title} description={t.dashboard.subtitle} />
      <Async
        query={query}
        isEmpty={(data) => data.properties_count === 0 && data.active_leases === 0}
        empty={
          <Empty
            title={t.dashboard.empty}
            hint={t.property.emptyHint}
            action={
              <Button asChild variant="primary">
                <Link to="/immobili/nuovo">{t.property.create}</Link>
              </Button>
            }
          />
        }
      >
        {(data) => <Tiles data={data} />}
      </Async>
    </>
  )
}

interface Tile {
  label: string
  value: string
  note?: string
  to: string
  action: string
  /** Il tono dice se il numero **richiede attenzione**, non se è grande. */
  tone?: 'neutral' | 'warning' | 'danger'
  /** Un tono di allerta su uno zero sarebbe un allarme per niente. */
  alertWhen?: boolean
}

function Tiles({ data }: { data: Dashboard }) {
  const tiles: Tile[] = [
    {
      label: t.dashboard.totalOutstanding,
      value: formatAmount(data.total_outstanding),
      note: t.dashboard.totalOutstandingNote,
      to: '/addebiti?insoluti=1',
      action: t.dashboard.goToOverdue,
      tone: 'warning',
      alertWhen: data.total_outstanding !== '0.00',
    },
    {
      // Il nono numero. **È il residuo degli addebiti stornati, non il loro
      // importo**: 800 con 300 incassati e poi stornato vale 500, perché gli altri
      // trecento sono entrati.
      //
      // Sta accanto all'insoluto e **non lo compensa**: quello dice quanto è
      // ancora dovuto, questo quanto non lo sarà mai. Un addebito *annullato* non
      // conta — non doveva essere chiesto, quindi non è una perdita — ed è la
      // distinzione per cui questo riquadro esiste.
      //
      // Il tono è `danger` e non `warning`: una perdita non aspetta nessuno, è già
      // avvenuta. E l'allerta scatta solo se c'è qualcosa: un rosso su uno zero
      // sarebbe un allarme per niente.
      label: t.dashboard.writtenOffTotal,
      value: formatAmount(data.written_off_total),
      note: t.dashboard.writtenOffTotalNote,
      to: '/addebiti?stato=written_off',
      action: t.dashboard.goToWrittenOff,
      tone: 'danger',
      alertWhen: data.written_off_total !== '0.00',
    },
    {
      label: t.dashboard.overdueCharges,
      value: formatCount(data.overdue_charges),
      to: '/addebiti?insoluti=1',
      action: t.dashboard.goToOverdue,
      tone: 'danger',
      alertWhen: data.overdue_charges > 0,
    },
    {
      label: t.dashboard.billsPendingReview,
      value: formatCount(data.bills_pending_review),
      note: t.dashboard.billsPendingReviewNote,
      to: '/bollette',
      action: t.dashboard.goToBills,
      tone: 'warning',
      alertWhen: data.bills_pending_review > 0,
    },
    {
      label: t.dashboard.leasesExpiring,
      value: formatCount(data.leases_expiring_90d),
      note: t.dashboard.leasesExpiringNote,
      to: '/contratti',
      action: t.dashboard.goToLeases,
      tone: 'warning',
      alertWhen: data.leases_expiring_90d > 0,
    },
    {
      label: t.dashboard.monthlyRentExpected,
      value: formatAmount(data.monthly_rent_expected),
      note: t.dashboard.monthlyRentNote,
      to: '/contratti?inEssere=1',
      action: t.dashboard.goToLeases,
    },
    {
      label: t.dashboard.activeLeases,
      value: formatCount(data.active_leases),
      to: '/contratti?inEssere=1',
      action: t.dashboard.goToLeases,
    },
    {
      label: t.dashboard.propertiesCount,
      value: formatCount(data.properties_count),
      to: '/immobili',
      action: t.dashboard.goToProperties,
    },
    {
      label: t.dashboard.openTickets,
      value: formatCount(data.open_tickets),
      // Le segnalazioni sono della fetta 2: il numero si mostra perché è vero, e
      // porta dove si potrà agire quando quella schermata esisterà. Fingere che
      // non esista sarebbe peggio che mostrarlo senza destinazione.
      to: '/contratti',
      action: t.dashboard.goToLeases,
      tone: 'warning',
      alertWhen: data.open_tickets > 0,
    },
  ]

  return (
    <ul className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-4">
      {tiles.map((tile) => (
        <li key={tile.label}>
          <Link
            to={tile.to}
            className={cn(
              'group flex h-full flex-col rounded-card border bg-surface p-4 transition-colors',
              tile.alertWhen && tile.tone === 'danger'
                ? 'border-danger/40 hover:border-danger'
                : tile.alertWhen && tile.tone === 'warning'
                  ? 'border-warning/40 hover:border-warning'
                  : 'border-border-subtle hover:border-accent/50',
            )}
          >
            <span className="text-xs font-medium tracking-wide text-ink-muted uppercase">
              {tile.label}
            </span>
            <span
              className={cn(
                'tabular mt-2 text-2xl font-semibold',
                tile.alertWhen && tile.tone === 'danger' && 'text-danger',
                tile.alertWhen && tile.tone === 'warning' && 'text-warning',
              )}
            >
              {tile.value}
            </span>
            {tile.note ? <span className="mt-1 text-xs text-ink-muted">{tile.note}</span> : null}
            <span className="mt-auto flex items-center gap-1 pt-3 text-xs font-medium text-accent">
              {tile.action}
              <ArrowUpRight
                className="size-3 transition-transform group-hover:translate-x-0.5"
                aria-hidden
              />
            </span>
          </Link>
        </li>
      ))}
    </ul>
  )
}
