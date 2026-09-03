import { zodResolver } from '@hookform/resolvers/zod'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useForm, useWatch } from 'react-hook-form'
import { Link, useNavigate } from 'react-router'
import { z } from 'zod'

import { listLeases, listProperties, registerBill } from '@/api/endpoints'
import { fieldErrorFor } from '@/api/problem'
import type { UtilityType } from '@/api/types'
import { InlineError, Loading } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card, CardBody } from '@/components/ui/card'
import { Field, Input, Select } from '@/components/ui/field'
import { PageHeader } from '@/components/ui/page'
import { keys } from '@/app/query'
import { useApi } from '@/app/session'
import { t } from '@/i18n'
import { parseDecimal } from '@/lib/decimal'
import { formatDate } from '@/lib/format'

const TYPES: UtilityType[] = [
  'electricity',
  'gas',
  'water',
  'heating',
  'waste',
  'internet',
  'condo_fees',
  'other',
]

const schema = z.object({
  property_id: z.string().min(1, t.validation.required),
  lease_id: z.string().optional(),
  utility_type: z.enum([
    'electricity',
    'gas',
    'water',
    'heating',
    'waste',
    'internet',
    'condo_fees',
    'other',
  ]),
  supplier_name: z.string().optional(),
  supplier_vat: z.string().optional(),
  invoice_number: z.string().optional(),
  period_start: z.string().optional(),
  period_end: z.string().optional(),
  total_amount: z
    .string()
    .refine((value) => parseDecimal(value) !== null, t.validation.amount),
  consumption_value: z.string().optional(),
  consumption_unit: z.string().optional(),
})

type Values = z.infer<typeof schema>

export function NewBillPage() {
  const client = useApi()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [error, setError] = useState<unknown>(null)

  const form = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: {
      property_id: '',
      lease_id: '',
      utility_type: 'electricity',
      supplier_name: '',
      supplier_vat: '',
      invoice_number: '',
      period_start: '',
      period_end: '',
      total_amount: '',
      consumption_value: '',
      consumption_unit: '',
    },
  })

  // `useWatch` e non `form.watch()`: il secondo restituisce una funzione che
  // non si può memoizzare, e il compilatore di React rinuncia a ottimizzare
  // l'intero componente quando la incontra.
  const propertyId = useWatch({ control: form.control, name: 'property_id' })

  const properties = useQuery({
    queryKey: keys.properties({ perBolletta: true }),
    queryFn: () => listProperties(client, {}),
  })

  const leases = useQuery({
    queryKey: keys.leases({ property: propertyId }),
    queryFn: () => listLeases(client, { property_id: propertyId }),
    enabled: Boolean(propertyId),
  })

  const mutation = useMutation({
    mutationFn: (values: Values) =>
      registerBill(client, values.property_id, {
        utility_type: values.utility_type,
        ...(values.lease_id ? { lease_id: values.lease_id } : {}),
        ...(values.supplier_name ? { supplier_name: values.supplier_name } : {}),
        ...(values.supplier_vat ? { supplier_vat: values.supplier_vat } : {}),
        ...(values.invoice_number ? { invoice_number: values.invoice_number } : {}),
        ...(values.period_start ? { period_start: values.period_start } : {}),
        ...(values.period_end ? { period_end: values.period_end } : {}),
        total_amount: values.total_amount,
        ...(values.consumption_value && values.consumption_unit
          ? {
              consumption: { value: values.consumption_value, unit: values.consumption_unit },
            }
          : {}),
      }),
    onSuccess: async (bill) => {
      await queryClient.invalidateQueries({ queryKey: ['bills'] })
      await queryClient.invalidateQueries({ queryKey: keys.dashboard })
      await navigate(`/bollette/${bill.id}`)
    },
    onError: (failure) => {
      for (const field of [
        'total_amount',
        'supplier_vat',
        'invoice_number',
        'period_start',
        'period_end',
      ] as const) {
        const message = fieldErrorFor(failure, field)
        if (message) form.setError(field, { message })
      }
      setError(failure)
    },
  })

  return (
    <>
      <PageHeader
        title={t.bill.newTitle}
        breadcrumb={
          <Link to="/bollette" className="hover:underline">
            {t.bill.title}
          </Link>
        }
      />
      <Card className="max-w-3xl">
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

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <Field label={t.bill.property} error={form.formState.errors.property_id?.message} required>
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
              <Field label={t.bill.utilityType} required>
                {(props) => (
                  <Select {...props} {...form.register('utility_type')}>
                    {TYPES.map((value) => (
                      <option key={value} value={value}>
                        {t.bill.types[value]}
                      </option>
                    ))}
                  </Select>
                )}
              </Field>
            </div>

            {/* Senza `lease_id` la bolletta la vede il solo staff: la policy la
                mostra a chi è parte del contratto indicato. È il campo che la
                rende leggibile a chi la paga, e per questo l'avvertenza è qui e
                non in una nota a fondo pagina. */}
            <Field label={t.bill.lease} hint={t.bill.leaseHint}>
              {(props) => (
                <Select {...props} {...form.register('lease_id')} disabled={!propertyId}>
                  <option value="">—</option>
                  {(leases.data?.items ?? []).map((lease) => (
                    <option key={lease.id} value={lease.id}>
                      {t.lease.types[lease.lease_type]} · {formatDate(lease.start_date)}
                    </option>
                  ))}
                </Select>
              )}
            </Field>

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
              <Field label={t.bill.supplierName}>
                {(props) => <Input {...props} {...form.register('supplier_name')} />}
              </Field>
              <Field label={t.bill.supplierVat} error={form.formState.errors.supplier_vat?.message}>
                {(props) => <Input {...props} {...form.register('supplier_vat')} placeholder="IT12345678901" />}
              </Field>
              <Field label={t.bill.invoiceNumber} error={form.formState.errors.invoice_number?.message}>
                {(props) => <Input {...props} {...form.register('invoice_number')} />}
              </Field>
            </div>

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
              <Field label={t.bill.periodStart} error={form.formState.errors.period_start?.message}>
                {(props) => <Input {...props} {...form.register('period_start')} type="date" />}
              </Field>
              <Field label={t.bill.periodEnd} error={form.formState.errors.period_end?.message}>
                {(props) => <Input {...props} {...form.register('period_end')} type="date" />}
              </Field>
              <Field label={t.bill.totalAmount} error={form.formState.errors.total_amount?.message} required>
                {(props) => (
                  <Input {...props} {...form.register('total_amount')} inputMode="decimal" placeholder="184.20" />
                )}
              </Field>
            </div>

            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              {/* Il consumo ammette **quattro** decimali, gli importi due. */}
              <Field label={t.bill.consumptionValue}>
                {(props) => (
                  <Input {...props} {...form.register('consumption_value')} inputMode="decimal" placeholder="123.4567" />
                )}
              </Field>
              <Field label={t.bill.consumptionUnit}>
                {(props) => <Input {...props} {...form.register('consumption_unit')} placeholder="Smc" />}
              </Field>
            </div>

            <div className="flex flex-wrap justify-end gap-2 pt-2">
              <Button asChild>
                <Link to="/bollette">{t.common.cancel}</Link>
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
