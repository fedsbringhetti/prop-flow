import { zodResolver } from '@hookform/resolvers/zod'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useForm } from 'react-hook-form'
import { Link, useNavigate } from 'react-router'
import { z } from 'zod'

import { createProperty } from '@/api/endpoints'
import type { PropertyType } from '@/api/types'
import { fieldErrorFor } from '@/api/problem'
import { InlineError } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card, CardBody } from '@/components/ui/card'
import { Field, Input, Select } from '@/components/ui/field'
import { PageHeader } from '@/components/ui/page'
import { keys } from '@/app/query'
import { useApi } from '@/app/session'
import { t } from '@/i18n'

const TYPES: PropertyType[] = [
  'apartment',
  'house',
  'studio',
  'villa',
  'office',
  'retail',
  'garage',
  'land',
  'other',
]

const schema = z.object({
  label: z.string().min(1, t.validation.required),
  property_type: z.enum([
    'apartment',
    'house',
    'studio',
    'villa',
    'office',
    'retail',
    'garage',
    'land',
    'other',
  ]),
  street: z.string().min(1, t.validation.required),
  city: z.string().min(1, t.validation.required),
  province: z.string().optional(),
  postal_code: z.string().optional(),
})

type Values = z.infer<typeof schema>

/**
 * La creazione chiede **solo l'indispensabile**.
 *
 * Superficie, piano, classe energetica e note si aggiungono dal dettaglio con un
 * `PATCH`: un form di venti campi al primo immobile è il modo più rapido di far
 * abbandonare la registrazione, e nessuno di quei campi è obbligatorio per il
 * resto del percorso.
 */
export function NewPropertyPage() {
  const client = useApi()
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const [error, setError] = useState<unknown>(null)

  const form = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: {
      label: '',
      property_type: 'apartment',
      street: '',
      city: '',
      province: '',
      postal_code: '',
    },
  })

  const mutation = useMutation({
    mutationFn: (values: Values) =>
      createProperty(client, {
        label: values.label,
        property_type: values.property_type,
        address: {
          street: values.street,
          city: values.city,
          ...(values.province ? { province: values.province } : {}),
          ...(values.postal_code ? { postal_code: values.postal_code } : {}),
        },
      }),
    onSuccess: async (property) => {
      // Il conteggio della dashboard cambia con questa creazione: invalidarlo
      // qui è ciò che rende vero il numero senza un ricaricamento.
      await queryClient.invalidateQueries({ queryKey: keys.dashboard })
      await queryClient.invalidateQueries({ queryKey: ['properties'] })
      await navigate(`/immobili/${property.id}`)
    },
    onError: (failure) => {
      // Gli errori per campo tornano nel campo. Un `422` mostrato solo in cima
      // costringe a cercare quale dei sei campi non andava.
      for (const field of ['label', 'street', 'city', 'province', 'postal_code'] as const) {
        const message = fieldErrorFor(failure, field) ?? fieldErrorFor(failure, `address.${field}`)
        if (message) form.setError(field, { message })
      }
      setError(failure)
    },
  })

  return (
    <>
      <PageHeader
        title={t.property.newTitle}
        breadcrumb={
          <Link to="/immobili" className="hover:underline">
            {t.property.title}
          </Link>
        }
      />
      <Card className="max-w-2xl">
        <CardBody>
          <form
            noValidate
            onSubmit={form.handleSubmit((values) => {
              setError(null)
              mutation.mutate(values)
            })}
            className="flex flex-col gap-4"
          >
            <InlineError error={error} />
            <Field label={t.property.label} hint={t.property.labelHint} error={form.formState.errors.label?.message} required>
              {(props) => <Input {...props} {...form.register('label')} autoFocus />}
            </Field>
            <Field label={t.property.type} error={form.formState.errors.property_type?.message} required>
              {(props) => (
                <Select {...props} {...form.register('property_type')}>
                  {TYPES.map((value) => (
                    <option key={value} value={value}>
                      {t.property.types[value]}
                    </option>
                  ))}
                </Select>
              )}
            </Field>
            <Field label={t.property.street} error={form.formState.errors.street?.message} required>
              {(props) => <Input {...props} {...form.register('street')} autoComplete="street-address" />}
            </Field>
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
              <Field label={t.property.city} error={form.formState.errors.city?.message} required className="sm:col-span-2">
                {(props) => <Input {...props} {...form.register('city')} autoComplete="address-level2" />}
              </Field>
              <Field label={t.property.province} error={form.formState.errors.province?.message}>
                {(props) => <Input {...props} {...form.register('province')} maxLength={2} />}
              </Field>
            </div>
            <Field label={t.property.postalCode} error={form.formState.errors.postal_code?.message}>
              {(props) => <Input {...props} {...form.register('postal_code')} inputMode="numeric" autoComplete="postal-code" />}
            </Field>

            <div className="flex flex-wrap justify-end gap-2 pt-2">
              <Button asChild>
                <Link to="/immobili">{t.common.cancel}</Link>
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
