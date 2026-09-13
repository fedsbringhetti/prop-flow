import { useQueryClient } from '@tanstack/react-query'
import { X } from 'lucide-react'
import { Link, useSearchParams } from 'react-router'

import { cancelTicket, listTickets } from '@/api/endpoints'
import type { Ticket } from '@/api/types'
import { Async, Empty } from '@/components/state'
import { Card } from '@/components/ui/card'
import { PageHeader } from '@/components/ui/page'
import {
  TableCaption,
  TableFrame,
  Td,
  TdSelect,
  Th,
  ThSelect,
  Tr,
  useSelection,
  type Selection,
} from '@/components/ui/table'
import { keys } from '@/app/query'
import { useApi, useIsStaff, useSession } from '@/app/session'
import { t } from '@/i18n'
import { formatDate } from '@/lib/format'
import { BulkBar, useBulkAction, VerbButton, type Verb } from '../shared/bulk'
import { LoadMore, usePagedQuery } from '../shared/pagination'
import { TicketPriorityBadge, TicketStatusBadge } from '../shared/status'
import { NewTicketDialog } from './NewTicketDialog'

/**
 * Le segnalazioni, **una schermata sola per due ruoli**.
 *
 * Il perimetro di questa fetta è la differenza fra le due metà del prodotto, non
 * il modulo: una segnalazione è dell'inquilino *e* del proprietario, e farne due
 * schermate sarebbe farne due da tenere allineate. A scegliere le righe è
 * `app.can_access_ticket` — chi ha aperto, chi è assegnato, e chi è parte del
 * contratto indicato — non un ramo applicativo sul ruolo.
 *
 * # `?open=true` non è `status <> 'closed'`
 *
 * Il filtro chiede i **sei stati non terminali** e usa l'indice parziale
 * `tickets_open_ix`, che è definito su esattamente quei sei valori: una
 * disuguaglianza non lo userebbe. La fetta A ha aggiunto una ragione in più per
 * crederci — perfino un filtro parametrico su un enum non usa un indice parziale,
 * perché il cast `text → enum` passa da `enum_in`, che è `STABLE` e non si riduce
 * a costante (ADR-040).
 *
 * # Il filtro sta nell'URL
 *
 * Come lo scadenzario e la coda dei pagamenti: la coda di lavorazione è un
 * indirizzo che si può salvare e mandare a qualcuno, non uno stato che vive solo
 * dentro il componente. **Aperte è il default**, perché è la ragione per cui si
 * apre questa pagina.
 */
/**
 * Il verbo di questa tabella è **Annulla**: una segnalazione annullata resta
 * visibile con la sua storia, e la conferma della scheda lo dice.
 */
const CANCEL: Verb = {
  label: t.ticket.cancel,
  confirmOne: t.ticket.cancelTitle,
  confirmMany: t.ticket.cancelTitleMany,
  explain: t.ticket.cancelExplain,
}

const WORKING = new Set<Ticket['status']>([
  'open',
  'triaged',
  'assigned',
  'in_progress',
  'waiting_tenant',
  'waiting_owner',
])

/**
 * Chi può annullare, come sulla scheda: lo staff finché la segnalazione è in
 * lavorazione, chi ha segnalato **finché è `open`** — oltre, l'API gli
 * risponde `org.staff_role_required`, e la riga non si offre. È lo stesso
 * criterio di `TicketActions`, e deve restarlo.
 */
function cancellable(ticket: Ticket, isStaff: boolean, userId: string | undefined): boolean {
  if (!WORKING.has(ticket.status)) return false
  if (isStaff) return true
  const isReporter = ticket.reported_by !== null && ticket.reported_by === userId
  return isReporter && ticket.status === 'open'
}

export function TicketsPage() {
  const client = useApi()
  const isStaff = useIsStaff()
  const { session } = useSession()
  const queryClient = useQueryClient()
  const [params, setParams] = useSearchParams()

  const showAll = params.get('tutte') === '1'
  const open = showAll ? undefined : true

  const query = usePagedQuery({
    queryKey: keys.tickets({ open: open ?? null }),
    fetchPage: (after) => listTickets(client, { open, after }),
  })

  const selection = useSelection(
    (query.data ?? [])
      .filter((ticket) => cancellable(ticket, isStaff, session?.userId))
      .map((ticket) => ticket.id),
  )
  const discard = useBulkAction({
    selection,
    action: (id) => cancelTicket(client, id),
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: ['tickets'] })
      // `open_tickets` conta i sei stati non terminali: annullare lo muove.
      await queryClient.invalidateQueries({ queryKey: keys.dashboard })
    },
  })

  return (
    <>
      <PageHeader
        title={t.ticket.title}
        description={isStaff ? t.ticket.staffSubtitle : t.ticket.tenantSubtitle}
        actions={<NewTicketDialog />}
      />

      <Card>
        <div className="flex flex-wrap items-center gap-4 border-b border-border-subtle px-4 py-3">
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              className="size-4"
              checked={!showAll}
              onChange={(event) => {
                const next = new URLSearchParams(params)
                if (event.target.checked) next.delete('tutte')
                else next.set('tutte', '1')
                setParams(next, { replace: true })
              }}
            />
            {t.ticket.onlyOpen}
          </label>
          <p className="text-xs text-ink-muted">{t.ticket.onlyOpenHint}</p>
        </div>

        <BulkBar selection={selection} verb={CANCEL} bulk={discard} icon={<X aria-hidden />} />

        <Async
          query={query}
          isEmpty={(items) => items.length === 0}
          empty={
            <Empty
              title={showAll ? t.ticket.empty : t.ticket.emptyOpen}
              hint={isStaff ? t.ticket.emptyHintStaff : t.ticket.emptyHintTenant}
            />
          }
        >
          {(items) => (
            <TableFrame>
              <TableCaption>{t.ticket.title}</TableCaption>
              <thead>
                <tr>
                  {/* La colonna c'è solo se almeno una riga si può annullare da
                      chi guarda: a un inquilino con sole segnalazioni in
                      lavorazione una colonna di caselle spente non dice niente. */}
                  {selection.total > 0 ? <ThSelect selection={selection} /> : null}
                  <Th>{t.ticket.reference}</Th>
                  <Th>{t.ticket.ticketTitle}</Th>
                  <Th>{t.ticket.status}</Th>
                  <Th>{t.ticket.priority}</Th>
                  <Th>{t.ticket.category}</Th>
                  <Th>{t.ticket.openedAt}</Th>
                  <Th>
                    <span className="sr-only">{t.common.actions}</span>
                  </Th>
                </tr>
              </thead>
              <tbody>
                {items.map((ticket) => (
                  <TicketRow
                    key={ticket.id}
                    ticket={ticket}
                    selection={selection}
                    column={selection.total > 0}
                    selectable={cancellable(ticket, isStaff, session?.userId)}
                    failure={discard.failureFor(ticket.id)}
                    run={discard.run}
                  />
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

function TicketRow({
  ticket,
  selection,
  column,
  selectable,
  failure,
  run,
}: {
  ticket: Ticket
  selection: Selection
  /** Se la colonna di selezione è montata su questa tabella. */
  column: boolean
  selectable: boolean
  failure: string | undefined
  run: (ids: readonly string[]) => Promise<unknown>
}) {
  const label = `${ticket.reference ?? ''} ${ticket.title}`.trim()
  return (
    <Tr>
      {column ? (
        <TdSelect
          selection={selection}
          id={ticket.id}
          label={label}
          failure={failure}
          disabled={!selectable}
        />
      ) : null}
      <Td className="font-numeric text-ink-muted whitespace-nowrap">
        {/* `reference` lo assegna il database e in teoria può mancare: su una riga
            appena inserita e riletta senza il trigger non ci sarebbe. Il trattino
            è più onesto di una stringa vuota. */}
        {ticket.reference ?? t.common.none}
      </Td>
      <Td>
        <Link
          to={`/segnalazioni/${ticket.id}`}
          className="font-medium text-accent underline-offset-4 hover:underline"
        >
          {ticket.title}
        </Link>
      </Td>
      <Td>
        <TicketStatusBadge status={ticket.status} />
      </Td>
      <Td>
        <TicketPriorityBadge priority={ticket.priority} />
      </Td>
      <Td className="text-ink-muted">{t.ticket.categories[ticket.category]}</Td>
      <Td className="whitespace-nowrap">{formatDate(ticket.created_at)}</Td>
      <Td className="text-right whitespace-nowrap">
        {selectable ? (
          <VerbButton verb={CANCEL} ids={[ticket.id]} run={run} rowLabel={label} />
        ) : null}
      </Td>
    </Tr>
  )
}
