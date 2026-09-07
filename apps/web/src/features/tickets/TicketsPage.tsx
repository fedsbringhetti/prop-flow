import { Link, useSearchParams } from 'react-router'

import { listTickets } from '@/api/endpoints'
import type { Ticket } from '@/api/types'
import { Async, Empty } from '@/components/state'
import { Card } from '@/components/ui/card'
import { PageHeader } from '@/components/ui/page'
import { TableCaption, TableFrame, Td, Th, Tr } from '@/components/ui/table'
import { keys } from '@/app/query'
import { useApi, useIsStaff } from '@/app/session'
import { t } from '@/i18n'
import { formatDate } from '@/lib/format'
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
export function TicketsPage() {
  const client = useApi()
  const isStaff = useIsStaff()
  const [params, setParams] = useSearchParams()

  const showAll = params.get('tutte') === '1'
  const open = showAll ? undefined : true

  const query = usePagedQuery({
    queryKey: keys.tickets({ open: open ?? null }),
    fetchPage: (after) => listTickets(client, { open, after }),
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
                  <Th>{t.ticket.reference}</Th>
                  <Th>{t.ticket.ticketTitle}</Th>
                  <Th>{t.ticket.status}</Th>
                  <Th>{t.ticket.priority}</Th>
                  <Th>{t.ticket.category}</Th>
                  <Th>{t.ticket.openedAt}</Th>
                </tr>
              </thead>
              <tbody>
                {items.map((ticket) => (
                  <TicketRow key={ticket.id} ticket={ticket} />
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

function TicketRow({ ticket }: { ticket: Ticket }) {
  return (
    <Tr>
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
    </Tr>
  )
}
