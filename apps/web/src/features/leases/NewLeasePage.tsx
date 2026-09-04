import { zodResolver } from '@hookform/resolvers/zod'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useForm, useWatch } from 'react-hook-form'
import { Link, useNavigate, useSearchParams } from 'react-router'
import { z } from 'zod'

import { createLease, listProperties, listRooms } from '@/api/endpoints'
import { fieldErrorFor } from '@/api/problem'
import { InlineError, Loading } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card, CardBody } from '@/components/ui/card'
import { Field, Input, Select } from '@/components/ui/field'
import { PageHeader } from '@/components/ui/page'
import { keys } from '@/app/query'
import { useApi } from '@/app/session'
import { t } from '@/i18n'
import { parseDecimal } from '@/lib/decimal'

const schema = z
  .object({
    property_id: z.string().min(1, t.validation.required),
    room_id: z.string().optional(),
    start_date: z.string().min(1, t.validation.date),
    end_date: z.string().optional(),
    // La validazione passa da `parseDecimal` e non da `Number`: è la stessa
    // ragione per cui gli importi restano stringhe fino al server.
    rent_amount: z
      .string()
      .refine((value) => parseDecimal(value) !== null, t.validation.amount)
      .refine((value) => {
        const parsed = parseDecimal(value)
        return parsed !== null && parsed.units > 0n
      }, t.validation.positiveAmount),
    deposit_amount: z.string().optional(),
  })
  .refine(
    (values) => !values.end_date || values.end_date > values.start_date,
    { path: ['end_date'], message: t.validation.endBeforeStart },
  )

type Values = z.infer<typeof schema>

export function NewLeasePage() {
  const client = useApi()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [params] = useSearchParams()
  const [error, setError] = useState<unknown>(null)

  const form = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: {
      property_id: params.get('immobile') ?? '',
      room_id: '',
      start_date: '',
      end_date: '',
      rent_amount: '',
      deposit_amount: '',
    },
  })

  // `useWatch` e non `form.watch()`: il secondo restituisce una funzione che
  // non si può memoizzare, e il compilatore di React rinuncia a ottimizzare
  // l'intero componente quando la incontra.
  const propertyId = useWatch({ control: form.control, name: 'property_id' })

  const properties = useQuery({
    queryKey: keys.properties({ perContratto: true }),
    queryFn: () => listProperties(client, {}),
  })

  // Le stanze si caricano solo quando un immobile è scelto: il tipo di contratto
  // **si deduce dalla presenza di `room_id`** e non si dichiara, quindi questo
  // campo *è* la scelta fra contratto sull'immobile intero e per stanza.
  const rooms = useQuery({
    queryKey: keys.rooms(propertyId),
    queryFn: () => listRooms(client, propertyId),
    enabled: Boolean(propertyId),
  })

  const mutation = useMutation({
    mutationFn: (values: Values) =>
      createLease(client, {
        property_id: values.property_id,
        ...(values.room_id ? { room_id: values.room_id } : {}),
        start_date: values.start_date,
        ...(values.end_date ? { end_date: values.end_date } : {}),
        rent_amount: values.rent_amount,
        ...(values.deposit_amount ? { deposit_amount: values.deposit_amount } : {}),
      }),
    onSuccess: async (lease) => {
      await queryClient.invalidateQueries({ queryKey: ['leases'] })
      await queryClient.invalidateQueries({ queryKey: keys.dashboard })
      await navigate(`/contratti/${lease.id}`)
    },
    onError: (failure) => {
      for (const field of ['start_date', 'end_date', 'rent_amount', 'deposit_amount'] as const) {
        const message = fieldErrorFor(failure, field)
        if (message) form.setError(field, { message })
      }
      setError(failure)
    },
  })

  return (
    <>
      <PageHeader
        title={t.lease.newTitle}
        breadcrumb={
          <Link to="/contratti" className="hover:underline">
            {t.lease.title}
          </Link>
        }
      />
      <Card className="max-w-2xl">
        <CardBody>
          <form
            noValidate
            className="flex flex-col gap-4"
            onSubmit={form.handleSubmit((values) => {
              setError(null)
              mutation.mutate(values)
            })}
          >
            <InlineError error={error} />

            <Field label={t.lease.property} error={form.formState.errors.property_id?.message} required>
              {(props) =>
                properties.isPending ? (
                  <Loading className="py-2" />
                ) : (
                  <Select {...props} {...form.register('property_id')}>
                    <option value="">—</option>
                    {(properties.data?.items ?? []).map((property) => (
                      <option key={property.id} value={property.id}>
                        {property.label}
                      </option>
                    ))}
                  </Select>
                )
              }
            </Field>

            <Field label={t.lease.room} hint={t.lease.wholeProperty}>
              {(props) => (
                <Select {...props} {...form.register('room_id')} disabled={!propertyId}>
                  <option value="">{t.lease.wholeProperty}</option>
                  {(rooms.data?.items ?? []).map((room) => (
                    <option key={room.id} value={room.id}>
                      {room.name}
                    </option>
                  ))}
                </Select>
              )}
            </Field>

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <Field label={t.lease.startDate} error={form.formState.errors.start_date?.message} required>
                {(props) => <Input {...props} {...form.register('start_date')} type="date" />}
              </Field>
              <Field
                label={t.lease.endDate}
                hint={t.lease.endDateHint}
                error={form.formState.errors.end_date?.message}
              >
                {(props) => <Input {...props} {...form.register('end_date')} type="date" />}
              </Field>
            </div>

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <Field label={t.lease.rentAmount} error={form.formState.errors.rent_amount?.message} required>
                {(props) => (
                  <Input {...props} {...form.register('rent_amount')} inputMode="decimal" placeholder="800.00" />
                )}
              </Field>
              <Field label={t.lease.depositAmount} error={form.formState.errors.deposit_amount?.message}>
                {(props) => (
                  <Input {...props} {...form.register('deposit_amount')} inputMode="decimal" placeholder="1600.00" />
                )}
              </Field>
            </div>

            <div className="flex flex-wrap justify-end gap-2 pt-2">
              <Button asChild>
                <Link to="/contratti">{t.common.cancel}</Link>
              </Button>
              <Button type="submit" variant="primary" disabled={mutation.isPending}>
                {mutation.isPending ? t.common.saving : t.common.create}
              </Button>
            </div>
          </form>
        </CardBody>
      </Card>
    </>
  )
}
