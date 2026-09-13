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
import type { EnergyClass, Property, PropertyStatus, Room, RoomStatus } from '@/api/types'
import { Async, Empty, InlineError } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card, CardBody, CardHeader, CardTitle } from '@/components/ui/card'
import { ConfirmAction, FormDialog } from '@/components/ui/confirm'
import { Field, Input, Select } from '@/components/ui/field'
import { DescriptionItem, DescriptionList, PageHeader, PageSection } from '@/components/ui/page'
import {
  TableCaption,
  TableFrame,
  Td,
  TdNumeric,
  TdSelect,
  Th,
  ThNumeric,
  ThSelect,
  Tr,
  useSelection,
} from '@/components/ui/table'
import { keys } from '@/app/query'
import { useApi, useIsStaff } from '@/app/session'
import { t } from '@/i18n'
import { compareDecimals, parseDecimal } from '@/lib/decimal'
import { formatAmount, formatDecimal } from '@/lib/format'
import { AttachmentsCard } from '../shared/attachments'
import { BulkBar, useBulkAction, VerbButton, type Verb } from '../shared/bulk'
import { RoomStatusBadge } from '../shared/status'
import { PropertyStatusBadge } from '../shared/status'

const ENERGY_CLASSES: EnergyClass[] = ['A4', 'A3', 'A2', 'A1', 'A', 'B', 'C', 'D', 'E', 'F', 'G']

/**
 * Gli stati che chi amministra **decide**.
 *
 * `rented` e `partially_rented` non ci sono perché non si scelgono: discendono dai
 * contratti in essere, e offrirli in una tendina chiederebbe a una persona di
 * ricordarsi di aggiornare a mano qualcosa che il database sa già (ADR-019).
 * `archived` non c'è perché ha la sua operazione, e il dominio rifiuta di
 * riceverlo da un `PATCH`.
 */
const EDITABLE_STATUSES = ['draft', 'available', 'maintenance'] as const

/** Il verbo delle stanze: **Archivia**, con la frase della scheda e il numero. */
const ARCHIVE_ROOM: Verb = {
  label: t.room.archive,
  confirmOne: t.room.archiveConfirm,
  confirmMany: t.room.archiveConfirmMany,
  explain: t.room.archiveExplain,
}

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
                <PropertyStatusBadge status={data.status} effectiveStatus={data.effective_status} />
              </CardHeader>
              <CardBody>
                <DescriptionList>
                  {/* Come per i contratti: il badge dice ciò che i contratti in
                      essere producono, e lo stato *scelto* resta leggibile qui —
                      accanto a quello effettivo, e non ripetuto su ogni riga
                      dell'elenco. */}
                  <DescriptionItem label={t.property.statusLabel}>
                    {t.property.statuses[data.status]}
                  </DescriptionItem>
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
    status: property.status,
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
        status: values.status,
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
        {/* Lo stato dichiarato, e solo quello: gli altri due li calcolano i
            contratti, e la spiegazione lo dice invece di lasciare che qualcuno
            cerchi «Affittato» nella tendina. */}
        <Field label={t.property.statusLabel} hint={t.property.statusHint}>
          {(props) => (
            <Select
              {...props}
              value={
                EDITABLE_STATUSES.includes(values.status as (typeof EDITABLE_STATUSES)[number])
                  ? values.status
                  : ''
              }
              onChange={(event) =>
                setValues((v) => ({ ...v, status: event.target.value as PropertyStatus }))
              }
            >
              {/* Un immobile già affittato mostra lo stato derivato, che non è
                  fra le scelte: l'opzione vuota evita che la tendina dica una
                  cosa falsa finché non si sceglie. */}
              {EDITABLE_STATUSES.includes(values.status as (typeof EDITABLE_STATUSES)[number]) ? null : (
                <option value="">{t.property.statuses[values.status]}</option>
              )}
              {EDITABLE_STATUSES.map((value) => (
                <option key={value} value={value}>
                  {t.property.statuses[value]}
                </option>
              ))}
            </Select>
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
  const queryClient = useQueryClient()
  const rooms = useQuery({
    queryKey: keys.rooms(propertyId),
    queryFn: () => listRooms(client, propertyId),
  })

  // L'archiviazione in blocco vive qui e non nella modale della stanza: è lo
  // stesso giro della riga singola, e la modale resta il posto dove si corregge.
  const selection = useSelection(canManage ? (rooms.data?.items ?? []).map((room) => room.id) : [])
  const archive = useBulkAction({
    selection,
    action: (id) => archiveRoom(client, id),
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.rooms(propertyId) })
      // Il conteggio dei locali sta sull'immobile: una stanza archiviata lo cambia.
      await queryClient.invalidateQueries({ queryKey: keys.property(propertyId) })
    },
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t.room.title}</CardTitle>
        {archived ? null : <NewRoomDialog propertyId={propertyId} />}
      </CardHeader>
      {canManage ? (
        <BulkBar selection={selection} verb={ARCHIVE_ROOM} bulk={archive} icon={<Archive aria-hidden />} />
      ) : null}
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
                {canManage ? <ThSelect selection={selection} /> : null}
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
                  {canManage ? (
                    <TdSelect
                      selection={selection}
                      id={room.id}
                      label={room.name}
                      failure={archive.failureFor(room.id)}
                    />
                  ) : null}
                  <Td>{room.name}</Td>
                  <Td>
                    <RoomStatusBadge status={room.status} effectiveStatus={room.effective_status} />
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
                    <Td className="whitespace-nowrap">
                      <span className="inline-flex gap-1">
                        <RoomDialog room={room} propertyId={propertyId} />
                        <VerbButton
                          verb={ARCHIVE_ROOM}
                          ids={[room.id]}
                          run={archive.run}
                          rowLabel={room.name}
                        />
                      </span>
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

/** I campi di una stanza come li digita chi scrive: **stringhe**, anche i numeri. */
interface RoomValues {
  name: string
  monthly_rent: string
  size_sqm: string
  floor: string
  bathroom: boolean
  furnished: boolean
  utility_share: string
}

const NEW_ROOM: RoomValues = {
  name: '',
  monthly_rent: '',
  size_sqm: '',
  floor: '',
  bathroom: false,
  furnished: false,
  utility_share: '',
}

function roomValuesOf(room: Room): RoomValues {
  return {
    name: room.name,
    monthly_rent: room.monthly_rent ?? '',
    size_sqm: room.size_sqm ?? '',
    floor: room.floor?.toString() ?? '',
    bathroom: room.has_private_bathroom,
    furnished: room.is_furnished ?? false,
    utility_share: room.utility_share_percent ?? '',
  }
}

/**
 * Un decimale come lo vuole l'API, da come lo scrive una persona italiana.
 *
 * `450,00` digitato in un'interfaccia italiana è un numero, non un errore: senza
 * questa riga l'API risponde `422` e la schermata dice «controlla i campi
 * segnalati» senza segnalarne nessuno.
 */
function decimalOf(raw: string): string {
  return raw.trim().replace(',', '.')
}

/**
 * La quota è `app.percentage`: 0–100, al più due decimali.
 *
 * Si verifica qui perché il rifiuto del server è un `422 validation.failed`, che
 * nel catalogo dice «controlla i campi segnalati» — vero, ma senza dire quale.
 * Solo **dopo un tentativo di invio**: un errore che compare a ogni tasto mentre
 * si scrive `33,33` è rumore, non aiuto.
 */
function shareProblem(raw: string, submitted: boolean): string | undefined {
  if (!submitted) return undefined
  const value = decimalOf(raw)
  if (!value) return undefined
  const parsed = parseDecimal(value)
  if (!parsed || parsed.scale > 2) return t.validation.percentage
  if (compareDecimals(value, '0') < 0 || compareDecimals(value, '100') > 0) {
    return t.validation.percentage
  }
  return undefined
}

/**
 * I campi di una stanza, scritti **una volta sola** per i due form.
 *
 * Erano due elenchi distinti e già divergenti: la creazione ne chiedeva quattro, la
 * modifica cinque, e il `POST` ne accetta otto. Chi creava una stanza non aveva
 * modo di sapere che *arredata* esistesse, né la quota delle utenze. Due elenchi
 * che chiedono le stesse cose divergono alla prima aggiunta, e la seconda finisce
 * in uno solo dei due — che è esattamente com'è andata.
 *
 * **`status` resta fuori da questo elenco condiviso, e non è una dimenticanza.**
 * La domanda si è chiusa: `occupied` è **derivato** dai contratti in essere e non
 * lo scrive nessuno (ADR-019), quindi ciò che si dichiara è soltanto se la stanza
 * sia fuori uso. La tendina a due valori vive perciò nella sola modifica —
 * `RoomDialog` — e non alla creazione, dove una stanza appena inserita è
 * disponibile per definizione.
 *
 * Quella tendina è anche **il posto in cui lo stato dichiarato di una stanza si
 * legge**, ora che il badge mostra il fatto e non più la coppia: le stanze non
 * hanno una scheda propria dove metterlo, e chi lo cerca lo cerca per cambiarlo.
 */
function RoomFields({
  values,
  onChange,
  shareError,
}: {
  values: RoomValues
  onChange: (patch: Partial<RoomValues>) => void
  shareError: string | undefined
}) {
  return (
    <>
      <Field label={t.room.name} hint={t.room.nameHint} required>
        {(props) => (
          <Input
            {...props}
            value={values.name}
            onChange={(event) => onChange({ name: event.target.value })}
          />
        )}
      </Field>
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <Field label={t.room.monthlyRent}>
          {(props) => (
            <Input
              {...props}
              inputMode="decimal"
              placeholder="450.00"
              value={values.monthly_rent}
              onChange={(event) => onChange({ monthly_rent: event.target.value })}
            />
          )}
        </Field>
        <Field label={t.room.surface}>
          {(props) => (
            <Input
              {...props}
              inputMode="decimal"
              value={values.size_sqm}
              onChange={(event) => onChange({ size_sqm: event.target.value })}
            />
          )}
        </Field>
        <Field label={t.room.floor}>
          {(props) => (
            <Input
              {...props}
              inputMode="numeric"
              value={values.floor}
              onChange={(event) => onChange({ floor: event.target.value })}
            />
          )}
        </Field>
      </div>
      <Field label={t.room.utilityShare} hint={t.room.utilityShareHint} error={shareError}>
        {(props) => (
          <Input
            {...props}
            inputMode="decimal"
            value={values.utility_share}
            onChange={(event) => onChange({ utility_share: event.target.value })}
          />
        )}
      </Field>
      <label className="flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          className="size-4"
          checked={values.bathroom}
          onChange={(event) => onChange({ bathroom: event.target.checked })}
        />
        {t.room.privateBathroom}
      </label>
      <label className="flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          className="size-4"
          checked={values.furnished}
          onChange={(event) => onChange({ furnished: event.target.checked })}
        />
        {t.room.furnished}
      </label>
    </>
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

  const [values, setValues] = useState<RoomValues>(roomValuesOf(room))
  // Lo stato **dichiarato**: `occupied` non è fra le scelte, perché lo dice il
  // contratto. Sta qui e non nel form di creazione — nessuno crea una stanza già
  // fuori uso — ed è la ragione per cui la sezione 1 lo aveva lasciato fuori.
  const [status, setStatus] = useState<RoomStatus>(
    room.status === 'occupied' ? 'available' : room.status,
  )
  const [submitted, setSubmitted] = useState(false)
  const shareError = shareProblem(values.utility_share, submitted)

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
        status,
        monthly_rent: decimalOf(values.monthly_rent) || null,
        size_sqm: decimalOf(values.size_sqm) || null,
        floor: values.floor.trim() ? Number(values.floor) : null,
        has_private_bathroom: values.bathroom,
        is_furnished: values.furnished,
        utility_share_percent: decimalOf(values.utility_share) || null,
      }),
    onSuccess: async () => {
      await reread()
      setOpen(false)
      setError(null)
      setSubmitted(false)
    },
    onError: setError,
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
            setSubmitted(true)
            if (shareProblem(values.utility_share, true)) return
            save.mutate()
          }}
        >
          <InlineError error={error} />
          <RoomFields
            values={values}
            onChange={(patch) => setValues((v) => ({ ...v, ...patch }))}
            shareError={shareError}
          />
          <Field label={t.room.statusLabel} hint={t.room.statusHint}>
            {(props) => (
              <Select
                {...props}
                value={status}
                onChange={(event) => setStatus(event.target.value as RoomStatus)}
              >
                <option value="available">{t.room.statuses.available}</option>
                <option value="unavailable">{t.room.statuses.unavailable}</option>
              </Select>
            )}
          </Field>
          <div className="flex flex-wrap justify-end gap-2">
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
  const [values, setValues] = useState<RoomValues>(NEW_ROOM)
  const [submitted, setSubmitted] = useState(false)
  const shareError = shareProblem(values.utility_share, submitted)

  const mutation = useMutation({
    mutationFn: () =>
      createRoom(client, propertyId, {
        name: values.name,
        // Sulla **creazione** un campo vuoto si omette e non si manda `null`: il
        // `POST` non ha la semantica del `PATCH`, e omettere è dire «non lo so»,
        // che è ciò che un campo lasciato in bianco significa qui.
        ...(decimalOf(values.monthly_rent) ? { monthly_rent: decimalOf(values.monthly_rent) } : {}),
        ...(decimalOf(values.size_sqm) ? { size_sqm: decimalOf(values.size_sqm) } : {}),
        ...(values.floor.trim() ? { floor: Number(values.floor) } : {}),
        has_private_bathroom: values.bathroom,
        is_furnished: values.furnished,
        ...(decimalOf(values.utility_share)
          ? { utility_share_percent: decimalOf(values.utility_share) }
          : {}),
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.rooms(propertyId) })
      // Il conteggio dei locali sta sull'immobile, e una stanza in più lo cambia.
      await queryClient.invalidateQueries({ queryKey: keys.property(propertyId) })
      setValues(NEW_ROOM)
      setSubmitted(false)
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
      // **Che si possa correggere dopo va detto qui**, non scoperto dopo: il nome
      // è l'unico dato che occupa qualcosa — è unico nell'immobile — e sapere che
      // tutto si sistema poi è ciò che toglie a questo form la solennità che non
      // deve avere.
      description={t.room.newExplain}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) {
          setError(null)
          setSubmitted(false)
        }
      }}
    >
      <form
        noValidate
        className="flex flex-col gap-4"
        onSubmit={(event) => {
          event.preventDefault()
          setSubmitted(true)
          if (shareProblem(values.utility_share, true)) return
          mutation.mutate()
        }}
      >
        <InlineError error={error} />
        <RoomFields
          values={values}
          onChange={(patch) => setValues((v) => ({ ...v, ...patch }))}
          shareError={shareError}
        />
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
