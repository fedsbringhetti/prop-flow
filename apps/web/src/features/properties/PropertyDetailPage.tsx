import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Archive, Plus } from 'lucide-react'
import { useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router'

import {
  archiveProperty,
  archiveRoom,
  createRoom,
  getProperty,
  listResidents,
  listRooms,
  updateProperty,
  updateRoom,
} from '@/api/endpoints'
import type { EnergyClass, Property, Room } from '@/api/types'
import { Async, Empty, InlineError } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card, CardBody, CardHeader, CardTitle } from '@/components/ui/card'
import { ConfirmAction, FormDialog } from '@/components/ui/confirm'
import { Field, Input, Select } from '@/components/ui/field'
import { DescriptionItem, DescriptionList, PageHeader, PageSection } from '@/components/ui/page'
import { TableCaption, TableFrame, Td, TdNumeric, Th, ThNumeric, Tr } from '@/components/ui/table'
import { keys } from '@/app/query'
import { useApi, useIsStaff } from '@/app/session'
import { t } from '@/i18n'
import { formatAmount, formatDecimal } from '@/lib/format'
import { AttachmentsCard } from '../shared/attachments'
import { RoomStatusBadge } from '../shared/status'
import { PropertyStatusBadge } from '../shared/status'

const ENERGY_CLASSES: EnergyClass[] = ['A4', 'A3', 'A2', 'A1', 'A', 'B', 'C', 'D', 'E', 'F', 'G']

export function PropertyDetailPage() {
  const { id = '' } = useParams()
  const client = useApi()
  const isStaff = useIsStaff()
  const navigate = useNavigate()
  const queryClient = useQueryClient()

  const property = useQuery({
    queryKey: keys.property(id),
    queryFn: () => getProperty(client, id),
  })

  const archive = useMutation({
    mutationFn: () => archiveProperty(client, id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['properties'] })
      await queryClient.invalidateQueries({ queryKey: keys.dashboard })
      await navigate('/immobili')
    },
  })

  return (
    <Async query={property}>
      {(data) => (
        <>
          <PageHeader
            title={data.label}
            description={[data.address.street, data.address.city].filter(Boolean).join(', ')}
            breadcrumb={
              <Link to="/immobili" className="hover:underline">
                {t.property.title}
              </Link>
            }
            actions={
              /* Modificare e archiviare sono dello staff. L'inquilino arriva qui
                 dal proprio contratto e legge: `GetProperty` è `ensure_member`,
                 `UpdateProperty` e `ArchiveProperty` no. */
              !isStaff ? null : (
              <>
                <EditPropertyDialog property={data} />
                {data.status === 'archived' ? null : (
                  <ConfirmAction
                    trigger={
                      <Button variant="dangerGhost">
                        <Archive aria-hidden />
                        {t.property.archive}
                      </Button>
                    }
                    title={t.property.archiveConfirm}
                    explanation={t.property.archiveExplain}
                    confirmLabel={t.property.archive}
                    onConfirm={() => archive.mutateAsync()}
                  />
                )}
              </>
              )
            }
          />

          <PageSection>
            <Card>
              <CardHeader>
                <CardTitle>{t.property.title}</CardTitle>
                <PropertyStatusBadge status={data.status} />
              </CardHeader>
              <CardBody>
                <DescriptionList>
                  <DescriptionItem label={t.property.type}>
                    {t.property.types[data.property_type]}
                  </DescriptionItem>
                  <DescriptionItem label={t.property.address}>
                    {[data.address.street, data.address.postal_code, data.address.city, data.address.province]
                      .filter(Boolean)
                      .join(' · ')}
                  </DescriptionItem>
                  <DescriptionItem label={t.property.surface} numeric>
                    {data.size_sqm ? `${formatDecimal(data.size_sqm)} m²` : t.common.none}
                  </DescriptionItem>
                  <DescriptionItem label={t.property.roomsCount} numeric>
                    {data.rooms_count ?? t.common.none}
                  </DescriptionItem>
                  <DescriptionItem label={t.property.floor} numeric>
                    {data.floor ?? t.common.none}
                  </DescriptionItem>
                  <DescriptionItem label={t.property.energyClass}>
                    {data.energy_class ?? t.common.none}
                  </DescriptionItem>
                  {data.notes ? (
                    <DescriptionItem label={t.property.notes}>{data.notes}</DescriptionItem>
                  ) : null}
                </DescriptionList>
              </CardBody>
            </Card>
          </PageSection>

          <PageSection>
            <RoomsCard
              propertyId={id}
              archived={data.status === 'archived' || !isStaff}
            />
          </PageSection>

          <PageSection>
            <ResidentsCard propertyId={id} />
          </PageSection>

          <PageSection>
            {/* Caricare è di chi è **membro**, non dello staff: l'inquilino
                allega al perimetro che gli compete, e a decidere che cosa gli
                competa è `app.can_access_attachment` attraverso le policy, non un
                ramo sul ruolo scritto qui. Su un immobile archiviato invece non
                si allega più, come non ci si aggiungono stanze. */}
            <AttachmentsCard
              owner={{ type: 'properties', id }}
              title={t.document.propertyTitle}
              hint={t.document.propertyHint}
              canUpload={data.status !== 'archived'}
            />
          </PageSection>
        </>
      )}
    </Async>
  )
}

/**
 * La modifica è un `PATCH` **parziale**, e la distinzione conta.
 *
 * Un campo lasciato vuoto va mandato come `null` esplicito, non omesso: omesso
 * significa «invariato», e senza la distinzione non si potrebbe mai cancellare una
 * nota o una superficie inserita per sbaglio.
 */
function EditPropertyDialog({ property }: { property: Property }) {
  const client = useApi()
  const queryClient = useQueryClient()
  const [open, setOpen] = useState(false)
  const [error, setError] = useState<unknown>(null)

  const [values, setValues] = useState({
    label: property.label,
    size_sqm: property.size_sqm ?? '',
    rooms_count: property.rooms_count?.toString() ?? '',
    floor: property.floor?.toString() ?? '',
    energy_class: property.energy_class ?? '',
    notes: property.notes ?? '',
  })

  const mutation = useMutation({
    mutationFn: () =>
      updateProperty(client, property.id, {
        label: values.label,
        // `|| null` e non `|| undefined`: svuotare il campo deve azzerare il
        // valore, non lasciarlo com'era.
        size_sqm: values.size_sqm.trim() || null,
        rooms_count: values.rooms_count.trim() ? Number(values.rooms_count) : null,
        floor: values.floor.trim() ? Number(values.floor) : null,
        energy_class: values.energy_class || null,
        notes: values.notes.trim() || null,
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.property(property.id) })
      await queryClient.invalidateQueries({ queryKey: ['properties'] })
      setOpen(false)
      setError(null)
    },
    onError: setError,
  })

  return (
    <FormDialog
      trigger={<Button>{t.common.edit}</Button>}
      title={t.common.edit}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) setError(null)
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
        <InlineError error={error} />
        <Field label={t.property.label} required>
          {(props) => (
            <Input
              {...props}
              value={values.label}
              onChange={(event) => setValues((v) => ({ ...v, label: event.target.value }))}
            />
          )}
        </Field>
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
          <Field label={t.property.surface}>
            {(props) => (
              <Input
                {...props}
                inputMode="decimal"
                value={values.size_sqm}
                onChange={(event) => setValues((v) => ({ ...v, size_sqm: event.target.value }))}
              />
            )}
          </Field>
          <Field label={t.property.roomsCount}>
            {(props) => (
              <Input
                {...props}
                inputMode="numeric"
                value={values.rooms_count}
                onChange={(event) => setValues((v) => ({ ...v, rooms_count: event.target.value }))}
              />
            )}
          </Field>
          <Field label={t.property.floor}>
            {(props) => (
              <Input
                {...props}
                inputMode="numeric"
                value={values.floor}
                onChange={(event) => setValues((v) => ({ ...v, floor: event.target.value }))}
              />
            )}
          </Field>
        </div>
        <Field label={t.property.energyClass}>
          {(props) => (
            <Select
              {...props}
              value={values.energy_class}
              onChange={(event) => setValues((v) => ({ ...v, energy_class: event.target.value }))}
            >
              <option value="">{t.common.none}</option>
              {ENERGY_CLASSES.map((value) => (
                <option key={value} value={value}>
                  {value}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t.property.notes}>
          {(props) => (
            <Input
              {...props}
              value={values.notes}
              onChange={(event) => setValues((v) => ({ ...v, notes: event.target.value }))}
            />
          )}
        </Field>
        <div className="flex justify-end gap-2">
          <Button onClick={() => setOpen(false)}>{t.common.cancel}</Button>
          <Button type="submit" variant="primary" disabled={mutation.isPending}>
            {mutation.isPending ? t.common.saving : t.common.save}
          </Button>
        </div>
      </form>
    </FormDialog>
  )
}

/**
 * Le stanze dell'immobile.
 *
 * `archived` copre due condizioni che qui hanno lo **stesso** effetto — un
 * immobile archiviato non accetta stanze nuove, e chi non amministra non ne
 * aggiunge — e nessun'altra: l'elenco si legge in entrambi i casi.
 *
 * Correggerle è la stessa condizione, e per questo la colonna delle azioni segue
 * lo stesso flag: `PATCH /rooms/{id}` e `DELETE /rooms/{id}` sono `staff`, e una
 * stanza di un immobile archiviato risponde `409`.
 */
function RoomsCard({ propertyId, archived }: { propertyId: string; archived: boolean }) {
  const canManage = !archived
  const client = useApi()
  const rooms = useQuery({
    queryKey: keys.rooms(propertyId),
    queryFn: () => listRooms(client, propertyId),
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t.room.title}</CardTitle>
        {archived ? null : <NewRoomDialog propertyId={propertyId} />}
      </CardHeader>
      <Async
        query={rooms}
        isEmpty={(data) => data.items.length === 0}
        empty={<Empty title={t.room.empty} />}
      >
        {(data) => (
          <TableFrame>
            <TableCaption>{t.room.title}</TableCaption>
            <thead>
              <tr>
                <Th>{t.room.name}</Th>
                <Th>{t.room.status}</Th>
                <Th>{t.room.privateBathroom}</Th>
                <ThNumeric>{t.room.surface}</ThNumeric>
                <ThNumeric>{t.room.monthlyRent}</ThNumeric>
                {canManage ? <Th>{t.common.actions}</Th> : null}
              </tr>
            </thead>
            <tbody>
              {data.items.map((room) => (
                <Tr key={room.id}>
                  <Td>{room.name}</Td>
                  <Td>
                    <RoomStatusBadge status={room.status} />
                  </Td>
                  <Td>{room.has_private_bathroom ? t.common.yes : t.common.no}</Td>
                  <TdNumeric>
                    {room.size_sqm ? `${formatDecimal(room.size_sqm)} m²` : t.common.none}
                  </TdNumeric>
                  <TdNumeric>
                    {room.monthly_rent
                      ? formatAmount(room.monthly_rent, { currency: room.currency })
                      : t.common.none}
                  </TdNumeric>
                  {canManage ? (
                    <Td>
                      <RoomDialog room={room} propertyId={propertyId} />
                    </Td>
                  ) : null}
                </Tr>
              ))}
            </tbody>
          </TableFrame>
        )}
      </Async>
    </Card>
  )
}

/**
 * La scheda di una stanza: si corregge, si archivia, e porta i suoi allegati.
 *
 * # Perché una modale e non una pagina
 *
 * Una stanza non ha una rotta propria — `GET /rooms/{id}` non esiste, l'elenco
 * passa dall'immobile — e inventarne una vorrebbe dire leggere il dettaglio
 * filtrando una lista. Questa modale è la **sola** superficie che una stanza
 * abbia, e per questo raccoglie tutto ciò che la riguarda invece di lasciare gli
 * allegati senza posto.
 *
 * # Il nome è unico per immobile, quindi un refuso non è cosmetico
 *
 * Occupa un nome che poi non si può riusare, e finché la stanza esiste nessun'altra
 * può chiamarsi così. Correggerlo è il caso d'uso principale di questa modale;
 * l'archiviazione **libera il nome**, e la conferma lo dice — la stringa era già
 * nel catalogo, scritta per questo momento.
 *
 * # Il `PATCH` è parziale con la semantica di sempre
 *
 * Assente = invariato, `null` = azzerato. Un campo svuotato manda `null`
 * esplicito: senza quella distinzione un canone inserito per sbaglio non si
 * potrebbe più togliere.
 */
function RoomDialog({ room, propertyId }: { room: Room; propertyId: string }) {
  const client = useApi()
  const queryClient = useQueryClient()
  const [open, setOpen] = useState(false)
  const [error, setError] = useState<unknown>(null)

  const [values, setValues] = useState({
    name: room.name,
    monthly_rent: room.monthly_rent ?? '',
    size_sqm: room.size_sqm ?? '',
    bathroom: room.has_private_bathroom,
    furnished: room.is_furnished ?? false,
  })

  async function reread() {
    await queryClient.invalidateQueries({ queryKey: keys.rooms(propertyId) })
    // Il conteggio dei locali sta sull'immobile: una stanza archiviata lo cambia.
    await queryClient.invalidateQueries({ queryKey: keys.property(propertyId) })
  }

  const save = useMutation({
    mutationFn: () =>
      updateRoom(client, room.id, {
        name: values.name,
        // `|| null` e non `|| undefined`: svuotare deve azzerare, non lasciare
        // com'era. È la stessa scelta del `PATCH` dell'immobile.
        monthly_rent: values.monthly_rent.trim() || null,
        size_sqm: values.size_sqm.trim() || null,
        has_private_bathroom: values.bathroom,
        is_furnished: values.furnished,
      }),
    onSuccess: async () => {
      await reread()
      setOpen(false)
      setError(null)
    },
    onError: setError,
  })

  const archive = useMutation({
    mutationFn: () => archiveRoom(client, room.id),
    onSuccess: async () => {
      await reread()
      setOpen(false)
    },
  })

  return (
    <FormDialog
      trigger={
        <Button size="sm" variant="ghost">
          {t.common.edit}
          {/* Il nome nell'etichetta accessibile: in una tabella di sei stanze
              «Modifica» ripetuto sei volte non dice quale, e chi naviga con un
              lettore di schermo sente solo quello. */}
          <span className="sr-only"> — {room.name}</span>
        </Button>
      }
      title={room.name}
      description={t.room.dialogExplain}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) setError(null)
      }}
    >
      <div className="flex flex-col gap-5">
        <form
          noValidate
          className="flex flex-col gap-4"
          onSubmit={(event) => {
            event.preventDefault()
            save.mutate()
          }}
        >
          <InlineError error={error} />
          <Field label={t.room.name} hint={t.room.nameHint} required>
            {(props) => (
              <Input
                {...props}
                value={values.name}
                onChange={(event) => setValues((v) => ({ ...v, name: event.target.value }))}
              />
            )}
          </Field>
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <Field label={t.room.monthlyRent}>
              {(props) => (
                <Input
                  {...props}
                  inputMode="decimal"
                  value={values.monthly_rent}
                  onChange={(event) =>
                    setValues((v) => ({ ...v, monthly_rent: event.target.value }))
                  }
                />
              )}
            </Field>
            <Field label={t.room.surface}>
              {(props) => (
                <Input
                  {...props}
                  inputMode="decimal"
                  value={values.size_sqm}
                  onChange={(event) => setValues((v) => ({ ...v, size_sqm: event.target.value }))}
                />
              )}
            </Field>
          </div>
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              className="size-4"
              checked={values.bathroom}
              onChange={(event) => setValues((v) => ({ ...v, bathroom: event.target.checked }))}
            />
            {t.room.privateBathroom}
          </label>
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              className="size-4"
              checked={values.furnished}
              onChange={(event) => setValues((v) => ({ ...v, furnished: event.target.checked }))}
            />
            {t.room.furnished}
          </label>
          <div className="flex flex-wrap justify-end gap-2">
            <ConfirmAction
              trigger={
                <Button variant="dangerGhost">
                  <Archive aria-hidden />
                  {t.room.archive}
                </Button>
              }
              title={t.room.archiveConfirm}
              explanation={t.room.archiveExplain}
              confirmLabel={t.room.archive}
              onConfirm={() => archive.mutateAsync()}
            />
            <Button type="submit" variant="primary" disabled={save.isPending}>
              {save.isPending ? t.common.saving : t.common.save}
            </Button>
          </div>
        </form>

        {/* Gli allegati della stanza stanno qui perché **qui** è l'unica
            superficie che una stanza abbia: senza una rotta di dettaglio, la sola
            alternativa sarebbe stata lasciarli irraggiungibili. */}
        <AttachmentsCard
          owner={{ type: 'rooms', id: room.id }}
          title={t.document.roomTitle}
          kind="photo"
        />
      </div>
    </FormDialog>
  )
}

function NewRoomDialog({ propertyId }: { propertyId: string }) {
  const client = useApi()
  const queryClient = useQueryClient()
  const [open, setOpen] = useState(false)
  const [error, setError] = useState<unknown>(null)
  const [values, setValues] = useState({ name: '', monthly_rent: '', size_sqm: '', bathroom: false })

  const mutation = useMutation({
    mutationFn: () =>
      createRoom(client, propertyId, {
        name: values.name,
        ...(values.monthly_rent.trim() ? { monthly_rent: values.monthly_rent.trim() } : {}),
        ...(values.size_sqm.trim() ? { size_sqm: values.size_sqm.trim() } : {}),
        has_private_bathroom: values.bathroom,
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.rooms(propertyId) })
      setValues({ name: '', monthly_rent: '', size_sqm: '', bathroom: false })
      setOpen(false)
      setError(null)
    },
    onError: setError,
  })

  return (
    <FormDialog
      trigger={
        <Button size="sm">
          <Plus aria-hidden />
          {t.room.create}
        </Button>
      }
      title={t.room.newTitle}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) setError(null)
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
        <InlineError error={error} />
        <Field label={t.room.name} hint={t.room.nameHint} required>
          {(props) => (
            <Input
              {...props}
              value={values.name}
              onChange={(event) => setValues((v) => ({ ...v, name: event.target.value }))}
            />
          )}
        </Field>
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <Field label={t.room.monthlyRent}>
            {(props) => (
              <Input
                {...props}
                inputMode="decimal"
                placeholder="450.00"
                value={values.monthly_rent}
                onChange={(event) => setValues((v) => ({ ...v, monthly_rent: event.target.value }))}
              />
            )}
          </Field>
          <Field label={t.room.surface}>
            {(props) => (
              <Input
                {...props}
                inputMode="decimal"
                value={values.size_sqm}
                onChange={(event) => setValues((v) => ({ ...v, size_sqm: event.target.value }))}
              />
            )}
          </Field>
        </div>
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            className="size-4"
            checked={values.bathroom}
            onChange={(event) => setValues((v) => ({ ...v, bathroom: event.target.checked }))}
          />
          {t.room.privateBathroom}
        </label>
        <div className="flex justify-end gap-2">
          <Button onClick={() => setOpen(false)}>{t.common.cancel}</Button>
          <Button type="submit" variant="primary" disabled={mutation.isPending}>
            {mutation.isPending ? t.common.saving : t.common.create}
          </Button>
        </div>
      </form>
    </FormDialog>
  )
}

/**
 * Chi abita l'immobile.
 *
 * L'elenco **non porta l'email**, e non è una scelta di presentazione: la risposta
 * viene da una funzione che proietta tre colonne, quindi l'email non c'è da
 * omettere. Chi è stato invitato e non ha ancora accettato compare senza
 * identificatore, e va detto — altrimenti sembra un errore.
 */
function ResidentsCard({ propertyId }: { propertyId: string }) {
  const client = useApi()
  const residents = useQuery({
    queryKey: keys.residents(propertyId),
    queryFn: () => listResidents(client, propertyId),
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t.property.residents}</CardTitle>
      </CardHeader>
      <Async
        query={residents}
        isEmpty={(data) => data.items.length === 0}
        empty={<Empty title={t.property.residentsEmpty} />}
      >
        {(data) => (
          <ul className="divide-y divide-border-subtle">
            {data.items.map((resident, index) => (
              <li
                key={`${resident.user_id ?? 'invitato'}-${resident.full_name}-${index}`}
                className="flex flex-wrap items-center justify-between gap-2 px-4 py-2.5 text-sm"
              >
                <span className="font-medium">{resident.full_name}</span>
                <span className="flex items-center gap-3 text-xs text-ink-muted">
                  {resident.room_name ? <span>{resident.room_name}</span> : null}
                  {resident.user_id ? null : <span>{t.property.residentPending}</span>}
                </span>
              </li>
            ))}
          </ul>
        )}
      </Async>
    </Card>
  )
}
