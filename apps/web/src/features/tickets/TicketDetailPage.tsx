import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Send } from 'lucide-react'
import { useState } from 'react'
import { Link, useParams } from 'react-router'

import { getTicket, listTicketMessages, postTicketMessage } from '@/api/endpoints'
import type { TicketDetail, TicketMessage, TicketStatusChange } from '@/api/types'
import { Async, Empty, InlineError } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card, CardBody, CardHeader, CardTitle } from '@/components/ui/card'
import { Field, Textarea } from '@/components/ui/field'
import { DescriptionItem, DescriptionList, PageHeader, PageSection } from '@/components/ui/page'
import { keys } from '@/app/query'
import { useApi, useIsStaff } from '@/app/session'
import { t } from '@/i18n'
import { formatAmount, formatDateTime } from '@/lib/format'
import { TicketActions } from './TicketActions'
import { LoadMore, usePagedQuery } from '../shared/pagination'
import { TicketPriorityBadge, TicketStatusBadge } from '../shared/status'

/**
 * La scheda di una segnalazione: che cosa è successo, che cosa è stato fatto, e
 * che cosa ci si è detti.
 *
 * # Tre letture e non una
 *
 * Il dettaglio porta l'aggregato **più lo storico degli stati**, che è corto e
 * limitato per costruzione — una riga per transizione. I **messaggi** stanno su
 * una rotta propria con il proprio cursore: una conversazione non ha un tetto, e
 * incorporarla significherebbe che il giorno in cui qualcuno scrive duecento
 * messaggi il dettaglio smette di rispondere.
 *
 * # `estimated_cost` assente non è zero
 *
 * A chi non è staff l'API lo manda `null` **anche quando un valore c'è**, ed è una
 * regola del contratto e non un confine di sicurezza: la Row Level Security lascia
 * comunque leggere la riga. Mostrare `0,00 €` sarebbe una bugia; il trattino è la
 * verità. `actual_cost` invece lo vede chiunque veda la segnalazione.
 */
export function TicketDetailPage() {
  const { id = '' } = useParams()
  const client = useApi()

  const ticket = useQuery({ queryKey: keys.ticket(id), queryFn: () => getTicket(client, id) })

  return (
    <Async query={ticket}>
      {(data) => (
        <>
          <PageHeader
            title={data.title}
            description={data.reference ?? undefined}
            breadcrumb={
              <Link to="/segnalazioni" className="hover:underline">
                {t.ticket.backToList}
              </Link>
            }
            actions={<TicketActions ticket={data} />}
          />

          <PageSection>
            <Card>
              <CardHeader>
                <CardTitle>{t.ticket.detailTitle}</CardTitle>
                <span className="flex flex-wrap items-center gap-2">
                  <TicketStatusBadge status={data.status} />
                  <TicketPriorityBadge priority={data.priority} />
                </span>
              </CardHeader>
              <CardBody className="flex flex-col gap-4">
                {/* Il testo con cui una persona ha descritto un guasto in un
                    momento preciso. Non si modifica, e il thread esiste apposta
                    per correggere il tiro: un titolo riscritto dopo tre messaggi
                    rende incomprensibili i tre messaggi. */}
                <p className="text-sm whitespace-pre-wrap text-ink">{data.description}</p>

                <DescriptionList>
                  <DescriptionItem label={t.ticket.category}>
                    {t.ticket.categories[data.category]}
                  </DescriptionItem>
                  <DescriptionItem label={t.ticket.openedAt}>
                    {formatDateTime(data.created_at)}
                  </DescriptionItem>
                  <DescriptionItem label={t.ticket.openedBy}>
                    {t.ticket.actors[data.reported_by_kind]}
                  </DescriptionItem>
                  <DescriptionItem label={t.ticket.property}>
                    <Link
                      to={`/immobili/${data.property_id}`}
                      className="text-accent underline-offset-4 hover:underline"
                    >
                      {t.property.title}
                    </Link>
                  </DescriptionItem>
                  <DescriptionItem label={t.ticket.assignedTo}>
                    {data.assigned_to ? formatDateTime(data.assigned_at) : t.ticket.assignedToNobody}
                  </DescriptionItem>
                  <DescriptionItem label={t.ticket.closedAt}>
                    {data.closed_at ? formatDateTime(data.closed_at) : t.common.none}
                  </DescriptionItem>
                  <DescriptionItem label={t.ticket.estimatedCost} numeric>
                    {data.estimated_cost
                      ? formatAmount(data.estimated_cost, { currency: data.currency })
                      : t.common.none}
                  </DescriptionItem>
                  <DescriptionItem label={t.ticket.actualCost} numeric>
                    {data.actual_cost
                      ? formatAmount(data.actual_cost, { currency: data.currency })
                      : t.common.none}
                  </DescriptionItem>
                </DescriptionList>

                {data.resolution_notes ? (
                  <div className="rounded-lg bg-surface-muted px-3 py-2">
                    <p className="text-xs font-medium tracking-wide text-ink-muted uppercase">
                      {t.ticket.resolutionNotes}
                    </p>
                    <p className="mt-1 text-sm whitespace-pre-wrap text-ink">
                      {data.resolution_notes}
                    </p>
                  </div>
                ) : null}
              </CardBody>
            </Card>
          </PageSection>

          <PageSection>
            <HistoryCard ticket={data} />
          </PageSection>

          <PageSection>
            <ThreadCard ticketId={id} />
          </PageSection>
        </>
      )}
    </Async>
  )
}

/**
 * Lo storico degli stati, che lo scrive un **trigger** del database e non l'API.
 *
 * La prima riga è sempre `null → open` e la produce l'inserimento: per questo
 * `changed_by` può essere nullo — è `app.current_user_id()`, che fuori da un
 * contesto di richiesta non c'è. Un «da: sconosciuto» sarebbe più preciso di un
 * nome inventato, ed è ciò che si mostra.
 */
function HistoryCard({ ticket }: { ticket: TicketDetail }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t.ticket.history}</CardTitle>
      </CardHeader>
      {ticket.history.length === 0 ? (
        <Empty title={t.state.emptyTitle} />
      ) : (
        <CardBody>
          <ol className="flex flex-col gap-3">
            {ticket.history.map((change) => (
              <HistoryRow key={`${change.changed_at}-${change.to_status}`} change={change} />
            ))}
          </ol>
        </CardBody>
      )}
    </Card>
  )
}

function HistoryRow({ change }: { change: TicketStatusChange }) {
  return (
    <li className="flex flex-wrap items-baseline gap-x-2 gap-y-1 text-sm">
      <span className="font-numeric text-xs text-ink-muted">
        {formatDateTime(change.changed_at)}
      </span>
      {change.from_status ? (
        <span className="text-ink-muted">
          {t.ticket.statuses[change.from_status]} →{' '}
          <span className="font-medium text-ink">{t.ticket.statuses[change.to_status]}</span>
        </span>
      ) : (
        <span className="font-medium text-ink">{t.ticket.historyOpened}</span>
      )}
      {change.reason ? <span className="text-ink-muted">· {change.reason}</span> : null}
    </li>
  )
}

/**
 * Il thread.
 *
 * # Un messaggio non si disfa
 *
 * `app.ticket_messages` ha **due sole policy**, `_select` e `_insert`: non esiste
 * modifica né cancellazione, nemmeno soft. Non è un limite da aggirare, è una
 * proprietà del thread — ed è scritto **prima** di inviare, dove serve, invece che
 * scoperto dopo cercando un bottone che non c'è.
 *
 * # Le note interne non si nascondono: non arrivano
 *
 * A un inquilino le filtra `ticket_messages_select`, non un campo della risposta.
 * Il client non ha nulla da nascondere, e la casella per scriverne compare al solo
 * staff perché a un inquilino che chiedesse `is_internal` l'API risponderebbe
 * `403`.
 */
function ThreadCard({ ticketId }: { ticketId: string }) {
  const client = useApi()
  const isStaff = useIsStaff()
  const queryClient = useQueryClient()

  const [body, setBody] = useState('')
  const [internal, setInternal] = useState(false)
  const [submitted, setSubmitted] = useState(false)

  const query = usePagedQuery({
    queryKey: keys.ticketMessages(ticketId),
    fetchPage: (after) => listTicketMessages(client, ticketId, after),
  })

  const send = useMutation({
    mutationFn: () =>
      postTicketMessage(client, ticketId, {
        body: body.trim(),
        ...(isStaff && internal ? { is_internal: true } : {}),
      }),
    onSuccess: async () => {
      // Solo dopo l'invio riuscito: un errore di rete non deve far perdere ciò che
      // è stato scritto, che su un thread può essere un paragrafo.
      setBody('')
      setInternal(false)
      setSubmitted(false)
      await queryClient.invalidateQueries({ queryKey: keys.ticketMessages(ticketId) })
    },
  })

  const bodyError = submitted && body.trim() === '' ? t.validation.required : undefined

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t.ticket.thread}</CardTitle>
      </CardHeader>

      <Async
        query={query}
        isEmpty={(items) => items.length === 0}
        empty={<Empty title={t.ticket.threadEmpty} hint={t.ticket.threadEmptyHint} />}
      >
        {(items) => (
          <ul className="divide-y divide-border-subtle">
            {items.map((message) => (
              <MessageRow key={message.id} message={message} />
            ))}
          </ul>
        )}
      </Async>

      <LoadMore
        hasNextPage={query.hasNextPage}
        isFetchingNextPage={query.isFetchingNextPage}
        fetchNextPage={() => void query.fetchNextPage()}
      />

      <CardBody className="border-t border-border-subtle">
        <form
          noValidate
          className="flex flex-col gap-3"
          onSubmit={(event) => {
            event.preventDefault()
            setSubmitted(true)
            if (body.trim() === '') return
            send.mutate()
          }}
        >
          <InlineError error={send.error} />

          <Field label={t.ticket.message} hint={t.ticket.messageIsFinal} error={bodyError} required>
            {(props) => (
              <Textarea
                {...props}
                rows={3}
                placeholder={t.ticket.messagePlaceholder}
                value={body}
                onChange={(event) => setBody(event.target.value)}
              />
            )}
          </Field>

          <div className="flex flex-wrap items-center justify-between gap-3">
            {isStaff ? (
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  className="size-4"
                  checked={internal}
                  onChange={(event) => setInternal(event.target.checked)}
                />
                <span>
                  {t.ticket.internalNote}
                  <span className="block text-xs text-ink-muted">{t.ticket.internalNoteHint}</span>
                </span>
              </label>
            ) : (
              <span />
            )}
            <Button type="submit" variant="primary" disabled={send.isPending}>
              <Send aria-hidden />
              {send.isPending ? t.common.saving : t.ticket.send}
            </Button>
          </div>
        </form>
      </CardBody>
    </Card>
  )
}

function MessageRow({ message }: { message: TicketMessage }) {
  return (
    <li
      className={
        message.is_internal
          ? 'border-l-2 border-warning bg-warning-soft/40 px-4 py-3'
          : 'px-4 py-3'
      }
    >
      <p className="flex flex-wrap items-baseline gap-x-2 text-xs text-ink-muted">
        <span className="font-medium text-ink">{t.ticket.actors[message.author_kind]}</span>
        <span className="font-numeric">{formatDateTime(message.created_at)}</span>
        {message.is_internal ? (
          <span className="font-medium text-warning">{t.ticket.internalNote}</span>
        ) : null}
      </p>
      <p className="mt-1 text-sm whitespace-pre-wrap text-ink">{message.body}</p>
    </li>
  )
}
