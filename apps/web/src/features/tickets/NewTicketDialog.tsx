import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus } from 'lucide-react'
import { useState } from 'react'
import { useNavigate } from 'react-router'

import { listLeases, listProperties, listRooms, openTicket } from '@/api/endpoints'
import type { TicketCategory, TicketPriority } from '@/api/types'
import { InlineError, Loading } from '@/components/state'
import { Button } from '@/components/ui/button'
import { FormDialog } from '@/components/ui/confirm'
import { Field, Input, Select, Textarea } from '@/components/ui/field'
import { keys } from '@/app/query'
import { useApi } from '@/app/session'
import { t } from '@/i18n'

/** L'ordine del menù, dal guasto più comune in avanti. `other` in fondo. */
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

/** Il valore della `<option>` «senza contratto»: una stringa vuota sarebbe
 *  indistinguibile da «non ho ancora scelto». */
const NO_LEASE = 'senza-contratto'

interface Values {
  property_id: string
  lease_id: string
  room_id: string
  title: string
  description: string
  category: TicketCategory
  priority: TicketPriority
  visible_to_residents: boolean
}

function emptyValues(): Values {
  return {
    property_id: '',
    lease_id: '',
    room_id: '',
    title: '',
    description: '',
    category: 'other',
    priority: 'medium',
    // Il default è **privata**, ed è il comportamento che il modulo ha sempre
    // avuto: un guasto dentro casa propria non riguarda i vicini di pianerottolo.
    visible_to_residents: false,
  }
}

/**
 * Aprire una segnalazione.
 *
 * # Il contratto non è un campo che si dimentica
 *
 * `app.can_access_ticket` concede l'accesso a chi è **parte del contratto** solo
 * se `lease_id` non è nullo. Ne segue che una segnalazione aperta senza indicarlo
 * non la vede nessun inquilino — nemmeno quello che ci abita, se non è stato lui
 * ad aprirla. È la stessa forma di `payer_user_id` nella fetta A: il difetto non
 * era che il campo fosse facoltativo, era che **ometterlo fosse il default**.
 *
 * Qui il campo non ha un vuoto di comodo: o si sceglie un contratto, o si dichiara
 * «senza contratto», e quell'opzione porta scritta la sua conseguenza. Quando i
 * candidati sono uno solo — il caso dell'inquilino, che ha un contratto — è già
 * scelto: non è un default nascosto, è l'unica risposta possibile.
 *
 * # Categoria e priorità partono dai default dell'API
 *
 * `other` e `medium` sono ciò che l'API mette quando il corpo non li porta.
 * Chiederli a chi segnala un guasto produrrebbe classificazioni a caso, ed è il
 * lavoro che il triage fa dopo: mostrarli con quel valore dice la verità su che
 * cosa succede se non si tocca nulla.
 */
export function NewTicketDialog() {
  const client = useApi()
  const queryClient = useQueryClient()
  const navigate = useNavigate()

  const [open, setOpen] = useState(false)
  const [values, setValues] = useState<Values>(emptyValues)
  const [submitted, setSubmitted] = useState(false)

  // Si legge solo a modale aperta: sono tre elenchi che servono qui e in nessun
  // altro punto della schermata.
  const properties = useQuery({
    queryKey: keys.properties({ forTicket: true }),
    queryFn: () => listProperties(client, {}),
    enabled: open,
  })

  const propertyId = values.property_id
  const leases = useQuery({
    queryKey: keys.leases({ property_id: propertyId }),
    queryFn: () => listLeases(client, { property_id: propertyId }),
    enabled: open && propertyId !== '',
  })
  const rooms = useQuery({
    queryKey: keys.rooms(propertyId),
    queryFn: () => listRooms(client, propertyId),
    enabled: open && propertyId !== '',
  })

  const propertyItems = properties.data?.items ?? []
  const leaseItems = leases.data?.items ?? []
  const roomItems = rooms.data?.items ?? []

  // Un immobile solo, o un contratto solo, non sono una scelta: sono la risposta.
  // Aggiustare durante il render e non in un effetto evita il fotogramma in cui il
  // campo è vuoto mentre lo stato è già deciso.
  if (open && values.property_id === '' && propertyItems.length === 1 && propertyItems[0]) {
    setValues((v) => ({ ...v, property_id: propertyItems[0]?.id ?? '' }))
  }
  if (open && values.lease_id === '' && leaseItems.length === 1 && leaseItems[0]) {
    setValues((v) => ({ ...v, lease_id: leaseItems[0]?.id ?? '' }))
  }

  const titleError = submitted && values.title.trim() === '' ? t.validation.required : undefined
  const descriptionError =
    submitted && values.description.trim() === '' ? t.validation.required : undefined
  const propertyError = submitted && values.property_id === '' ? t.validation.required : undefined
  const leaseError = submitted && values.lease_id === '' ? t.validation.required : undefined

  const mutation = useMutation({
    mutationFn: () =>
      openTicket(client, values.property_id, {
        title: values.title.trim(),
        description: values.description.trim(),
        ...(values.room_id ? { room_id: values.room_id } : {}),
        ...(values.lease_id === NO_LEASE ? {} : { lease_id: values.lease_id }),
        category: values.category,
        priority: values.priority,
        ...(values.visible_to_residents ? { visible_to_residents: true } : {}),
      }),
    onSuccess: async (ticket) => {
      setOpen(false)
      setValues(emptyValues())
      setSubmitted(false)
      await queryClient.invalidateQueries({ queryKey: ['tickets'] })
      await queryClient.invalidateQueries({ queryKey: keys.dashboard })
      // Al dettaglio: chi ha appena segnalato un guasto vuole vedere che cosa ne è
      // stato, e il thread è lì.
      await navigate(`/segnalazioni/${ticket.id}`)
    },
  })

  const nothingToOpenOn = properties.isSuccess && propertyItems.length === 0

  return (
    <FormDialog
      trigger={
        <Button variant="primary">
          <Plus aria-hidden />
          {t.ticket.create}
        </Button>
      }
      title={t.ticket.newTitle}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) {
          // Il form non perde ciò che è stato digitato **su un errore**: la modale
          // resta aperta con i campi pieni. Si azzera solo chiudendola, che è il
          // gesto con cui si dichiara di aver rinunciato.
          setValues(emptyValues())
          setSubmitted(false)
          mutation.reset()
        }
      }}
    >
      {properties.isPending ? <Loading /> : null}

      {nothingToOpenOn ? (
        <p className="rounded-lg bg-surface-muted px-3 py-2 text-sm text-ink-muted">
          {t.ticket.noProperty}
        </p>
      ) : null}

      {properties.isSuccess && !nothingToOpenOn ? (
        <form
          noValidate
          className="flex flex-col gap-4"
          onSubmit={(event) => {
            event.preventDefault()
            setSubmitted(true)
            if (
              values.title.trim() === '' ||
              values.description.trim() === '' ||
              values.property_id === '' ||
              values.lease_id === ''
            ) {
              return
            }
            mutation.mutate()
          }}
        >
          <InlineError error={mutation.error} />

          <Field label={t.ticket.ticketTitle} hint={t.ticket.ticketTitleHint} error={titleError} required>
            {(props) => (
              <Input
                {...props}
                autoFocus
                value={values.title}
                onChange={(event) => setValues((v) => ({ ...v, title: event.target.value }))}
              />
            )}
          </Field>

          <Field
            label={t.ticket.description}
            hint={t.ticket.descriptionHint}
            error={descriptionError}
            required
          >
            {(props) => (
              <Textarea
                {...props}
                rows={4}
                value={values.description}
                onChange={(event) => setValues((v) => ({ ...v, description: event.target.value }))}
              />
            )}
          </Field>

          <Field label={t.ticket.property} error={propertyError} required>
            {(props) => (
              <Select
                {...props}
                value={values.property_id}
                onChange={(event) =>
                  // Cambiare immobile invalida contratto e stanza: appartengono a
                  // quello di prima, e lasciarli produrrebbe un `422` per
                  // riferimenti di un altro immobile.
                  setValues((v) => ({
                    ...v,
                    property_id: event.target.value,
                    lease_id: '',
                    room_id: '',
                  }))
                }
              >
                <option value="">{t.common.none}</option>
                {propertyItems.map((property) => (
                  <option key={property.id} value={property.id}>
                    {property.label}
                  </option>
                ))}
              </Select>
            )}
          </Field>

          <Field
            label={t.ticket.lease}
            hint={t.ticket.leaseHint}
            error={leaseError}
            required
          >
            {(props) => (
              <Select
                {...props}
                disabled={values.property_id === '' || leases.isPending}
                value={values.lease_id}
                onChange={(event) => setValues((v) => ({ ...v, lease_id: event.target.value }))}
              >
                {/* Un vuoto che **non si può inviare**: è ciò che rende il
                    contratto una scelta invece di un default. */}
                <option value="">{t.common.none}</option>
                {leaseItems.map((lease) => (
                  <option key={lease.id} value={lease.id}>
                    {t.lease.types[lease.lease_type]} · {t.lease.statuses[lease.effective_status]}
                  </option>
                ))}
                <option value={NO_LEASE}>{t.ticket.leaseNone}</option>
              </Select>
            )}
          </Field>

          {values.lease_id === NO_LEASE ? (
            <p className="rounded-lg border border-warning/30 bg-warning-soft px-3 py-2 text-xs text-warning">
              {t.ticket.leaseNoneHint}
            </p>
          ) : null}

          {/* **Sta subito sotto il contratto, e non fra le opzioni in fondo.**
              Le due domande sono la stessa vista da due lati — chi la vedrà — e
              l'avviso qui sopra dice il caso peggiore: senza contratto non la vede
              nessun inquilino. Questa casella è la risposta quando il guasto è di
              tutti, ed è l'unico posto in cui la conseguenza si legge prima di
              averla scelta. */}
          <label className="flex items-start gap-2 text-sm">
            <input
              type="checkbox"
              className="mt-0.5 size-4"
              checked={values.visible_to_residents}
              onChange={(event) =>
                setValues((v) => ({ ...v, visible_to_residents: event.target.checked }))
              }
            />
            <span>
              {t.ticket.visibleToResidents}
              <span className="block text-xs text-ink-muted">{t.ticket.visibleToResidentsHint}</span>
            </span>
          </label>

          {roomItems.length > 0 ? (
            <Field label={t.ticket.room} hint={t.common.optional}>
              {(props) => (
                <Select
                  {...props}
                  value={values.room_id}
                  onChange={(event) => setValues((v) => ({ ...v, room_id: event.target.value }))}
                >
                  <option value="">{t.ticket.wholeProperty}</option>
                  {roomItems.map((room) => (
                    <option key={room.id} value={room.id}>
                      {room.name}
                    </option>
                  ))}
                </Select>
              )}
            </Field>
          ) : null}

          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <Field label={t.ticket.category}>
              {(props) => (
                <Select
                  {...props}
                  value={values.category}
                  onChange={(event) =>
                    setValues((v) => ({ ...v, category: event.target.value as TicketCategory }))
                  }
                >
                  {CATEGORIES.map((category) => (
                    <option key={category} value={category}>
                      {t.ticket.categories[category]}
                    </option>
                  ))}
                </Select>
              )}
            </Field>

            <Field label={t.ticket.priority}>
              {(props) => (
                <Select
                  {...props}
                  value={values.priority}
                  onChange={(event) =>
                    setValues((v) => ({ ...v, priority: event.target.value as TicketPriority }))
                  }
                >
                  {PRIORITIES.map((priority) => (
                    <option key={priority} value={priority}>
                      {t.ticket.priorities[priority]}
                    </option>
                  ))}
                </Select>
              )}
            </Field>
          </div>

          <div className="flex justify-end gap-2">
            <Button type="button" onClick={() => setOpen(false)} disabled={mutation.isPending}>
              {t.common.cancel}
            </Button>
            <Button type="submit" variant="primary" disabled={mutation.isPending}>
              {mutation.isPending ? t.common.saving : t.ticket.create}
            </Button>
          </div>
        </form>
      ) : null}

      {properties.error ? <InlineError error={properties.error} /> : null}
    </FormDialog>
  )
}
