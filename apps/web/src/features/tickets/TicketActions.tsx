import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Check, Hand, Pause, Pencil, Play, RotateCcw, Tag, X } from 'lucide-react'
import { useState } from 'react'

import {
  assignTicket,
  cancelTicket,
  closeTicket,
  correctTicketReport,
  reopenTicket,
  startTicket,
  triageTicket,
  updateTicketCosts,
  waitTicket,
} from '@/api/endpoints'
import type { CorrectTicketReportInput } from '@/api/endpoints'
import type { Ticket, TicketCategory, TicketPriority, WaitingOn } from '@/api/types'
import { Button } from '@/components/ui/button'
import { ConfirmAction, FormDialog } from '@/components/ui/confirm'
import { InlineError } from '@/components/state'
import { Field, Input, Select, Textarea } from '@/components/ui/field'
import { keys } from '@/app/query'
import { useApi, useIsStaff, useSession } from '@/app/session'
import { t } from '@/i18n'
import { isPositive, parseDecimal } from '@/lib/decimal'

const CATEGORIES: readonly TicketCategory[] = [
  'plumbing',
  'electrical',
  'heating_cooling',
  'appliance',
  'structural',
  'pest',
  'cleaning',
  'connectivity',
  'security',
  'other',
] as const

const PRIORITIES: readonly TicketPriority[] = ['low', 'medium', 'high', 'urgent'] as const

/**
 * Le azioni della segnalazione, **guidate dalla macchina a stati** e non dal
 * ruolo soltanto.
 *
 * # Che cosa compare da dove
 *
 * Le guardie qui replicano quelle del dominio, che sono l'autorità:
 * `triage` parte dal solo `open`; `assign` da `open`, `triaged` o `assigned`;
 * `start` da `assigned` e dai due `waiting_*`; `wait` dal solo `in_progress`;
 * `close` e `cancel` da tutti e sei gli stati di lavorazione; `reopen` dal solo
 * `closed`. Offrire un bottone fuori da questi confini darebbe `409
 * ticket.invalid_transition` ogni volta, ed è precisamente ciò che un'interfaccia
 * non deve fare.
 *
 * **Non è un confine di sicurezza**: a negare sono l'API e la policy di riga
 * dietro. Qui si evita di *offrire* ciò che verrebbe rifiutato.
 *
 * # Riapertura e annullamento: si instrada sul **codice**, non sul ruolo
 *
 * A un coinquilino che provi a riaprire l'API risponde `403 ticket.not_yours` e
 * **non** `org.staff_role_required`: non gli manca un ruolo — nessun ruolo gli
 * darebbe quell'operazione — gli manca l'essere stato lui a segnalare, e i due
 * rimedi sono diversi. Per l'annullamento vale il contrario in un caso preciso:
 * chi ha segnalato può annullare **finché nessuno ci ha messo mano**, e da lì in
 * poi `org.staff_role_required` è il codice vero.
 *
 * Ne segue che le due azioni si mostrano a chi ha segnalato — non solo allo staff
 * — e che i messaggi di rifiuto arrivano dal `code`, tradotto nel catalogo.
 */
export function TicketActions({ ticket }: { ticket: Ticket }) {
  const client = useApi()
  const isStaff = useIsStaff()
  const { session } = useSession()
  const queryClient = useQueryClient()

  async function refresh() {
    await queryClient.invalidateQueries({ queryKey: keys.ticket(ticket.id) })
    await queryClient.invalidateQueries({ queryKey: ['tickets'] })
    // `open_tickets` conta i sei stati non terminali: chiudere, annullare o
    // riaprire lo muove, e il riquadro del riepilogo deve dirlo.
    await queryClient.invalidateQueries({ queryKey: keys.dashboard })
  }

  // Chi chiama, sempre: l'azione è «prendi in carico». Senza un utente non c'è
  // nessuno da assegnare, e il bottone non compare — mandare una stringa vuota
  // produrrebbe un `422` per uno stato che dentro il guscio non esiste.
  const userId = session?.userId
  const take = useMutation({
    mutationFn: () => assignTicket(client, ticket.id, userId ?? ''),
    onSuccess: refresh,
  })
  const start = useMutation({ mutationFn: () => startTicket(client, ticket.id), onSuccess: refresh })
  const discard = useMutation({
    mutationFn: () => cancelTicket(client, ticket.id),
    onSuccess: refresh,
  })

  const working =
    ticket.status === 'open' ||
    ticket.status === 'triaged' ||
    ticket.status === 'assigned' ||
    ticket.status === 'in_progress' ||
    ticket.status === 'waiting_tenant' ||
    ticket.status === 'waiting_owner'

  const suspended = ticket.status === 'waiting_tenant' || ticket.status === 'waiting_owner'
  const startable = ticket.status === 'assigned' || suspended
  const assignable =
    ticket.status === 'open' || ticket.status === 'triaged' || ticket.status === 'assigned'

  // **`assigned` resta fra gli stati assegnabili**: il bottone serve anche a
  // *riassegnare* a qualcun altro, e toglierlo dall'elenco toglierebbe quella
  // funzione a tutti. Quello che si aggiunge è che chi guarda non l'abbia già in
  // carico — «prendi in carico» su qualcosa che si ha già in carico è l'unico caso
  // in cui il bottone non dice niente. Il confronto passa dal `null` esplicito: un
  // `undefined === undefined` renderebbe assegnatario chiunque su una segnalazione
  // che non ha nessuno.
  const alreadyMine = ticket.assigned_to !== null && ticket.assigned_to === userId

  // Chi ha segnalato: `reported_by` è nullo quando la riga è stata scritta fuori
  // da un contesto di richiesta, e un `undefined === undefined` non deve
  // trasformare chiunque nel segnalante.
  const isReporter = ticket.reported_by !== null && ticket.reported_by === session?.userId

  return (
    <>
      {isStaff && ticket.status === 'open' ? (
        <TriageDialog ticket={ticket} onDone={refresh} />
      ) : null}

      {isStaff && assignable && !alreadyMine && userId ? (
        <ConfirmAction
          trigger={
            <Button variant="primary">
              <Hand aria-hidden />
              {t.ticket.take}
            </Button>
          }
          title={t.ticket.takeTitle}
          explanation={t.ticket.takeExplain}
          confirmLabel={t.ticket.take}
          variant="primary"
          onConfirm={() => take.mutateAsync()}
        />
      ) : null}

      {isStaff && startable ? (
        <ConfirmAction
          trigger={
            <Button>
              <Play aria-hidden />
              {suspended ? t.ticket.resume : t.ticket.start}
            </Button>
          }
          title={suspended ? t.ticket.resume : t.ticket.start}
          explanation={t.ticket.waitExplain}
          confirmLabel={suspended ? t.ticket.resume : t.ticket.start}
          variant="primary"
          onConfirm={() => start.mutateAsync()}
        />
      ) : null}

      {isStaff && ticket.status === 'in_progress' ? (
        <WaitDialog ticket={ticket} onDone={refresh} />
      ) : null}

      {isStaff && working ? <CloseDialog ticket={ticket} onDone={refresh} /> : null}

      {/* I costi si modificano in **qualunque** stato tranne `cancelled`, dove
          l'API risponde `409 ticket.cancelled`: una segnalazione annullata non si
          tocca più. Una chiusa sì — la fattura del fornitore arriva dopo. */}
      {isStaff && ticket.status !== 'cancelled' ? (
        <CostsDialog ticket={ticket} onDone={refresh} />
      ) : null}

      {/* Riaprire è di **chi ha segnalato**, oltre che dello staff: è ciò che
          rende la segnalazione un rapporto invece di una casella postale. Senza,
          un guasto ricorrente diventa una segnalazione nuova ogni volta e lo
          storico si frantuma proprio dove servirebbe intero. */}
      {(isStaff || isReporter) && ticket.status === 'closed' ? (
        <ReopenDialog ticket={ticket} onDone={refresh} />
      ) : null}

      {/* Correggere ha la stessa condizione dell'annullamento, e non è un caso:
          è la stessa regola — lo staff sempre, chi ha segnalato finché nessuno ci
          ha messo mano — e nell'API è la stessa funzione. Su una annullata non
          corregge nessuno: l'API risponde `409 ticket.cancelled`. */}
      {(isStaff || (isReporter && ticket.status === 'open')) && ticket.status !== 'cancelled' ? (
        <CorrectDialog ticket={ticket} onDone={refresh} />
      ) : null}

      {/* Chi ha segnalato può annullare **finché è `open`**. Oltre, l'API gli
          risponde `org.staff_role_required`, e quello è il codice vero: la
          segnalazione è sua, ma annullarne una in lavorazione butterebbe via il
          lavoro di qualcun altro. */}
      {(isStaff || (isReporter && ticket.status === 'open')) && working ? (
        <ConfirmAction
          trigger={
            <Button variant="dangerGhost">
              <X aria-hidden />
              {t.ticket.cancel}
            </Button>
          }
          title={t.ticket.cancelTitle}
          explanation={t.ticket.cancelExplain}
          confirmLabel={t.ticket.cancel}
          onConfirm={() => discard.mutateAsync()}
        />
      ) : null}

      {isReporter && !isStaff && working && ticket.status !== 'open' ? (
        <p className="max-w-xs text-xs text-ink-muted">
          {t.ticket.onlyOpenCancelledByReporter} {t.ticket.onlyOpenCorrectedByReporter}
        </p>
      ) : null}
    </>
  )
}

/** Il triage: categoria e priorità insieme, perché è ciò che `triaged` significa. */
function TriageDialog({ ticket, onDone }: { ticket: Ticket; onDone: () => Promise<void> }) {
  const client = useApi()
  const [open, setOpen] = useState(false)
  const [category, setCategory] = useState<TicketCategory>(ticket.category)
  const [priority, setPriority] = useState<TicketPriority>(ticket.priority)

  const mutation = useMutation({
    mutationFn: () => triageTicket(client, ticket.id, { category, priority }),
    onSuccess: async () => {
      setOpen(false)
      await onDone()
    },
  })

  return (
    <FormDialog
      trigger={
        <Button variant="primary">
          <Tag aria-hidden />
          {t.ticket.triage}
        </Button>
      }
      title={t.ticket.triageTitle}
      description={t.ticket.triageExplain}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) mutation.reset()
      }}
    >
      <form
        noValidate
        className="flex flex-col gap-4"
        onSubmit={(event) => {
          event.preventDefault()
          mutation.mutate()
        }}
      >
        <InlineError error={mutation.error} />

        <Field label={t.ticket.category}>
          {(props) => (
            <Select
              {...props}
              value={category}
              onChange={(event) => setCategory(event.target.value as TicketCategory)}
            >
              {CATEGORIES.map((value) => (
                <option key={value} value={value}>
                  {t.ticket.categories[value]}
                </option>
              ))}
            </Select>
          )}
        </Field>

        <Field label={t.ticket.priority}>
          {(props) => (
            <Select
              {...props}
              value={priority}
              onChange={(event) => setPriority(event.target.value as TicketPriority)}
            >
              {PRIORITIES.map((value) => (
                <option key={value} value={value}>
                  {t.ticket.priorities[value]}
                </option>
              ))}
            </Select>
          )}
        </Field>

        <div className="flex justify-end gap-2">
          <Button type="button" onClick={() => setOpen(false)} disabled={mutation.isPending}>
            {t.common.cancel}
          </Button>
          <Button type="submit" variant="primary" disabled={mutation.isPending}>
            {mutation.isPending ? t.common.saving : t.ticket.triage}
          </Button>
        </div>
      </form>
    </FormDialog>
  )
}

/**
 * La sospensione, e il campo che le dà senso.
 *
 * `waiting_on` non è un dettaglio: è la differenza fra «fermo perché aspetto che
 * tu sia in casa» e una segnalazione che sembra abbandonata.
 */
function WaitDialog({ ticket, onDone }: { ticket: Ticket; onDone: () => Promise<void> }) {
  const client = useApi()
  const [open, setOpen] = useState(false)
  const [waitingOn, setWaitingOn] = useState<WaitingOn>('tenant')

  const mutation = useMutation({
    mutationFn: () => waitTicket(client, ticket.id, waitingOn),
    onSuccess: async () => {
      setOpen(false)
      await onDone()
    },
  })

  return (
    <FormDialog
      trigger={
        <Button>
          <Pause aria-hidden />
          {t.ticket.wait}
        </Button>
      }
      title={t.ticket.waitTitle}
      description={t.ticket.waitExplain}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) mutation.reset()
      }}
    >
      <form
        noValidate
        className="flex flex-col gap-4"
        onSubmit={(event) => {
          event.preventDefault()
          mutation.mutate()
        }}
      >
        <InlineError error={mutation.error} />

        <Field label={t.ticket.waitOn}>
          {(props) => (
            <Select
              {...props}
              value={waitingOn}
              onChange={(event) => setWaitingOn(event.target.value as WaitingOn)}
            >
              <option value="tenant">{t.ticket.waitOnTenant}</option>
              <option value="owner">{t.ticket.waitOnOwner}</option>
            </Select>
          )}
        </Field>

        <div className="flex justify-end gap-2">
          <Button type="button" onClick={() => setOpen(false)} disabled={mutation.isPending}>
            {t.common.cancel}
          </Button>
          <Button type="submit" variant="primary" disabled={mutation.isPending}>
            {mutation.isPending ? t.common.saving : t.ticket.wait}
          </Button>
        </div>
      </form>
    </FormDialog>
  )
}

/** La chiusura, con la nota facoltativa su com'è andata a finire. */
function CloseDialog({ ticket, onDone }: { ticket: Ticket; onDone: () => Promise<void> }) {
  const client = useApi()
  const [notes, setNotes] = useState('')

  const mutation = useMutation({
    mutationFn: () => closeTicket(client, ticket.id, notes.trim() || undefined),
    onSuccess: onDone,
  })

  return (
    <ConfirmAction
      trigger={
        <Button>
          <Check aria-hidden />
          {t.ticket.close}
        </Button>
      }
      title={t.ticket.closeTitle}
      explanation={t.ticket.closeExplain}
      confirmLabel={t.ticket.close}
      variant="primary"
      onConfirm={() => mutation.mutateAsync()}
    >
      <Field label={t.ticket.resolutionNotes} hint={t.ticket.resolutionNotesHint}>
        {(props) => (
          <Textarea
            {...props}
            rows={3}
            value={notes}
            onChange={(event) => setNotes(event.target.value)}
          />
        )}
      </Field>
    </ConfirmAction>
  )
}

/**
 * I costi: **l'unico `PATCH`** del modulo.
 *
 * Vale la semantica generale — assente = invariato, `null` = azzerato — e un campo
 * svuotato deve quindi arrivare come `null` esplicito, altrimenti cancellare una
 * stima sarebbe impossibile. Gli importi passano da `@/lib/decimal`: `parseFloat`
 * è vietato, e `"184.20"` letto come binario torna `184.19999999999999`.
 */
function CostsDialog({ ticket, onDone }: { ticket: Ticket; onDone: () => Promise<void> }) {
  const client = useApi()
  const [open, setOpen] = useState(false)
  const [estimated, setEstimated] = useState(ticket.estimated_cost ?? '')
  const [actual, setActual] = useState(ticket.actual_cost ?? '')
  const [submitted, setSubmitted] = useState(false)

  const estimatedError = amountProblem(estimated, submitted)
  const actualError = amountProblem(actual, submitted)

  const mutation = useMutation({
    mutationFn: () =>
      updateTicketCosts(client, ticket.id, {
        estimated_cost: patchValue(estimated),
        actual_cost: patchValue(actual),
      }),
    onSuccess: async () => {
      setOpen(false)
      setSubmitted(false)
      await onDone()
    },
  })

  return (
    <FormDialog
      trigger={<Button>{t.ticket.editCosts}</Button>}
      title={t.ticket.costs}
      description={t.ticket.onlyCostsArePatchable}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) {
          setEstimated(ticket.estimated_cost ?? '')
          setActual(ticket.actual_cost ?? '')
          setSubmitted(false)
          mutation.reset()
        }
      }}
    >
      <form
        noValidate
        className="flex flex-col gap-4"
        onSubmit={(event) => {
          event.preventDefault()
          setSubmitted(true)
          if (amountProblem(estimated, true) || amountProblem(actual, true)) return
          mutation.mutate()
        }}
      >
        <InlineError error={mutation.error} />

        <Field
          label={t.ticket.estimatedCost}
          hint={t.ticket.estimatedCostHint}
          error={estimatedError}
        >
          {(props) => (
            <Input
              {...props}
              inputMode="decimal"
              value={estimated}
              onChange={(event) => setEstimated(event.target.value)}
            />
          )}
        </Field>

        <Field label={t.ticket.actualCost} hint={t.ticket.actualCostHint} error={actualError}>
          {(props) => (
            <Input
              {...props}
              inputMode="decimal"
              value={actual}
              onChange={(event) => setActual(event.target.value)}
            />
          )}
        </Field>

        <div className="flex justify-end gap-2">
          <Button type="button" onClick={() => setOpen(false)} disabled={mutation.isPending}>
            {t.common.cancel}
          </Button>
          <Button type="submit" variant="primary" disabled={mutation.isPending}>
            {mutation.isPending ? t.common.saving : t.common.save}
          </Button>
        </div>
      </form>
    </FormDialog>
  )
}

/**
 * Un campo svuotato è un `null` **esplicito**, non un campo assente.
 *
 * Senza questa distinzione un `PATCH` non potrebbe cancellare un valore, e «non
 * inviato» finirebbe per significare «metti a null» — o, peggio, il contrario:
 * una stima cancellata nel form resterebbe in database senza che nulla protesti.
 */
function patchValue(raw: string): string | null {
  const value = raw.trim().replace(',', '.')
  return value === '' ? null : value
}

function amountProblem(raw: string, submitted: boolean): string | undefined {
  if (!submitted) return undefined
  const value = raw.trim().replace(',', '.')
  if (!value) return undefined
  if (!parseDecimal(value)) return t.validation.amount
  if (!isPositive(value)) return t.validation.positiveAmount
  return undefined
}

/**
 * La riapertura, che ora **chiede perché**.
 *
 * Non è una conferma con un campo di comodo: da quando un thread chiuso non
 * riceve più messaggi, riaprire è l'unica via per dire «non è risolto», e senza il
 * motivo non resterebbe da nessuna parte scritto perché la segnalazione è tornata
 * viva. L'API risponde `422` a un corpo senza `reason`, quindi il campo è
 * obbligatorio anche qui — e la verifica scatta **solo dopo un tentativo di
 * invio**, che è la regola del progetto sui form.
 *
 * Il testo finisce **nel thread**, e va detto prima di scriverlo: chi lo legge
 * deve sapere che sta parlando a qualcuno, non compilando un campo.
 */
function ReopenDialog({ ticket, onDone }: { ticket: Ticket; onDone: () => Promise<void> }) {
  const client = useApi()
  const [open, setOpen] = useState(false)
  const [reason, setReason] = useState('')
  const [submitted, setSubmitted] = useState(false)

  const mutation = useMutation({
    mutationFn: () => reopenTicket(client, ticket.id, reason.trim()),
    onSuccess: async () => {
      setOpen(false)
      setReason('')
      setSubmitted(false)
      await onDone()
    },
  })

  const error = submitted && reason.trim() === '' ? t.validation.required : undefined

  return (
    <FormDialog
      trigger={
        <Button variant="primary">
          <RotateCcw aria-hidden />
          {t.ticket.reopen}
        </Button>
      }
      title={t.ticket.reopenTitle}
      description={t.ticket.reopenExplain}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) mutation.reset()
      }}
    >
      <form
        noValidate
        className="flex flex-col gap-4"
        onSubmit={(event) => {
          event.preventDefault()
          setSubmitted(true)
          if (reason.trim() === '') return
          mutation.mutate()
        }}
      >
        <InlineError error={mutation.error} />

        <Field label={t.ticket.reopenReason} hint={t.ticket.reopenReasonHint} error={error} required>
          {(props) => (
            <Textarea
              {...props}
              rows={3}
              placeholder={t.ticket.reopenReasonPlaceholder}
              value={reason}
              onChange={(event) => setReason(event.target.value)}
            />
          )}
        </Field>

        <div className="flex justify-end gap-2">
          <Button type="button" onClick={() => setOpen(false)} disabled={mutation.isPending}>
            {t.common.cancel}
          </Button>
          <Button type="submit" variant="primary" disabled={mutation.isPending}>
            {mutation.isPending ? t.common.saving : t.ticket.reopen}
          </Button>
        </div>
      </form>
    </FormDialog>
  )
}

/**
 * La correzione del testo e della visibilità.
 *
 * **Tre campi e una sola autorizzazione**, che è la ragione per cui l'API li tiene
 * su una rotta a sé (`PATCH /tickets/{id}/report`) invece di allargare il `PATCH`
 * dei costi: sono tutti e tre ciò che chi ha segnalato ha dichiarato.
 *
 * Si manda **solo ciò che è cambiato**, perché assente = invariato: rimandare
 * indietro un testo non toccato lo sovrascriverebbe con la copia che questa
 * schermata aveva in mano, che può essere vecchia.
 */
function CorrectDialog({ ticket, onDone }: { ticket: Ticket; onDone: () => Promise<void> }) {
  const client = useApi()
  const [open, setOpen] = useState(false)
  const [title, setTitle] = useState(ticket.title)
  const [description, setDescription] = useState(ticket.description)
  const [shared, setShared] = useState(ticket.visible_to_residents)
  const [submitted, setSubmitted] = useState(false)

  const mutation = useMutation({
    mutationFn: () => {
      const input: CorrectTicketReportInput = {}
      if (title.trim() !== ticket.title) input.title = title.trim()
      if (description.trim() !== ticket.description) input.description = description.trim()
      if (shared !== ticket.visible_to_residents) input.visible_to_residents = shared
      return correctTicketReport(client, ticket.id, input)
    },
    onSuccess: async () => {
      setOpen(false)
      setSubmitted(false)
      await onDone()
    },
  })

  const titleError = submitted && title.trim() === '' ? t.validation.required : undefined
  const descriptionError = submitted && description.trim() === '' ? t.validation.required : undefined

  return (
    <FormDialog
      trigger={
        <Button>
          <Pencil aria-hidden />
          {t.ticket.correct}
        </Button>
      }
      title={t.ticket.correctTitle}
      description={t.ticket.correctExplain}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (next) {
          // Si riparte da ciò che la scheda ha in mano: la modale si riapre dopo
          // una correzione riuscita e non deve mostrare il testo di prima.
          setTitle(ticket.title)
          setDescription(ticket.description)
          setShared(ticket.visible_to_residents)
          setSubmitted(false)
        } else {
          mutation.reset()
        }
      }}
    >
      <form
        noValidate
        className="flex flex-col gap-4"
        onSubmit={(event) => {
          event.preventDefault()
          setSubmitted(true)
          if (title.trim() === '' || description.trim() === '') return
          mutation.mutate()
        }}
      >
        <InlineError error={mutation.error} />

        <Field label={t.ticket.ticketTitle} error={titleError} required>
          {(props) => (
            <Input {...props} value={title} onChange={(event) => setTitle(event.target.value)} />
          )}
        </Field>

        <Field label={t.ticket.description} error={descriptionError} required>
          {(props) => (
            <Textarea
              {...props}
              rows={4}
              value={description}
              onChange={(event) => setDescription(event.target.value)}
            />
          )}
        </Field>

        <label className="flex items-start gap-2 text-sm">
          <input
            type="checkbox"
            className="mt-0.5 size-4"
            checked={shared}
            onChange={(event) => setShared(event.target.checked)}
          />
          <span>
            {t.ticket.visibleToResidents}
            <span className="block text-xs text-ink-muted">{t.ticket.visibleToResidentsHint}</span>
          </span>
        </label>

        <div className="flex justify-end gap-2">
          <Button type="button" onClick={() => setOpen(false)} disabled={mutation.isPending}>
            {t.common.cancel}
          </Button>
          <Button type="submit" variant="primary" disabled={mutation.isPending}>
            {mutation.isPending ? t.common.saving : t.common.save}
          </Button>
        </div>
      </form>
    </FormDialog>
  )
}
