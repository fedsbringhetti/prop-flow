import { Plus } from 'lucide-react'
import { Link, useSearchParams } from 'react-router'

import { listProperties } from '@/api/endpoints'
import type { PropertyStatus } from '@/api/types'
import { Async, Empty } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { Select } from '@/components/ui/field'
import { PageHeader } from '@/components/ui/page'
import {
  TableCaption,
  TableFrame,
  Td,
  TdNumeric,
  Th,
  ThNumeric,
  Tr,
} from '@/components/ui/table'
import { keys } from '@/app/query'
import { useApi } from '@/app/session'
import { t } from '@/i18n'
import { formatDecimal } from '@/lib/format'
import { LoadMore, usePagedQuery } from '../shared/pagination'
import { PropertyStatusBadge } from '../shared/status'

const STATUSES: PropertyStatus[] = [
  'draft',
  'available',
  'partially_rented',
  'rented',
  'maintenance',
  'archived',
]

export function PropertiesPage() {
  const client = useApi()
  // Il filtro sta nell'URL e non in uno stato locale: un elenco filtrato è un
  // indirizzo condivisibile, e il tasto «indietro» torna al filtro precedente
  // invece di uscire dalla schermata.
  const [params, setParams] = useSearchParams()
  const status = (params.get('stato') ?? '') as PropertyStatus | ''

  const query = usePagedQuery({
    queryKey: keys.properties({ status }),
    fetchPage: (after) =>
      listProperties(client, { status: status || undefined, after }),
  })

  return (
    <>
      <PageHeader
        title={t.property.title}
        actions={
          <Button asChild variant="primary">
            <Link to="/immobili/nuovo">
              <Plus aria-hidden />
              {t.property.create}
            </Link>
          </Button>
        }
      />

      <Card>
        <div className="flex flex-wrap items-center gap-2 border-b border-border-subtle px-4 py-3">
          <label htmlFor="filtro-stato" className="text-xs font-medium text-ink-muted">
            {t.property.status}
          </label>
          <Select
            id="filtro-stato"
            className="w-auto min-w-40"
            value={status}
            onChange={(event) => {
              const next = new URLSearchParams(params)
              if (event.target.value) next.set('stato', event.target.value)
              else next.delete('stato')
              setParams(next, { replace: true })
            }}
          >
            <option value="">{t.common.all}</option>
            {STATUSES.map((value) => (
              <option key={value} value={value}>
                {t.property.statuses[value]}
              </option>
            ))}
          </Select>
        </div>

        <Async
          query={query}
          isEmpty={(items) => items.length === 0}
          empty={
            <Empty
              title={t.property.empty}
              hint={t.property.emptyHint}
              action={
                <Button asChild variant="primary">
                  <Link to="/immobili/nuovo">{t.property.create}</Link>
                </Button>
              }
            />
          }
        >
          {(items) => (
            <TableFrame>
              <TableCaption>{t.property.title}</TableCaption>
              <thead>
                <tr>
                  <Th>{t.property.label}</Th>
                  <Th>{t.property.type}</Th>
                  <Th>{t.property.city}</Th>
                  <Th>{t.property.status}</Th>
                  <ThNumeric>{t.property.surface}</ThNumeric>
                </tr>
              </thead>
              <tbody>
                {items.map((property) => (
                  <Tr key={property.id}>
                    <Td>
                      <Link
                        to={`/immobili/${property.id}`}
                        className="font-medium text-accent underline-offset-4 hover:underline"
                      >
                        {property.label}
                      </Link>
                      <span className="block text-xs text-ink-muted">
                        {property.address.street}
                      </span>
                    </Td>
                    <Td>{t.property.types[property.property_type]}</Td>
                    <Td>{property.address.city}</Td>
                    <Td>
                      <PropertyStatusBadge status={property.status} />
                    </Td>
                    <TdNumeric>
                      {property.size_sqm ? `${formatDecimal(property.size_sqm)} m²` : t.common.none}
                    </TdNumeric>
                  </Tr>
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
