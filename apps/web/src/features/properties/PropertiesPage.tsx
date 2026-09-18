import { useQueryClient } from '@tanstack/react-query'
import { Archive, Plus } from 'lucide-react'
import { Link, useSearchParams } from 'react-router'

import { archiveProperty, listProperties } from '@/api/endpoints'
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
import { formatDecimal } from '@/lib/format'
import { BulkBar, useBulkAction, VerbButton, type Verb } from '../shared/bulk'
import { LoadMore, usePagedQuery } from '../shared/pagination'
import { PropertyStatusBadge } from '../shared/status'

/**
 * Gli stati **memorizzati**, che sono gli unici filtrabili.
 *
 * `rented` e `partially_rented` discendono dai contratti in essere e non sono in
 * database: filtrarli risponde `422` con `property.derived_status_filter`, e per
 * quella domanda c'è il filtro dell'occupazione qui accanto. `archived` resta
 * un'opzione che non può corrispondere a niente — archiviare mette `deleted_at` e
 * la riga esce da ogni elenco — ed è il terzo di `R26` che questa sezione non
 * chiude.
 */
const STATUSES = ['draft', 'available', 'maintenance', 'archived'] as const

/**
 * Il verbo di questa tabella è **Archivia**, e la conferma è quella della scheda
 * con il numero al posto di «questo». Nessun «Elimina»: un immobile archiviato
 * resta leggibile con la sua storia, e il bottone deve dirlo.
 */
const ARCHIVE: Verb = {
  label: t.property.archive,
  confirmOne: t.property.archiveConfirm,
  confirmMany: t.property.archiveConfirmMany,
  explain: t.property.archiveExplain,
}

export function PropertiesPage() {
  const client = useApi()
  const isStaff = useIsStaff()
  const queryClient = useQueryClient()
  // Il filtro sta nell'URL e non in uno stato locale: un elenco filtrato è un
  // indirizzo condivisibile, e il tasto «indietro» torna al filtro precedente
  // invece di uscire dalla schermata.
  const [params, setParams] = useSearchParams()
  const status = (params.get('stato') ?? '') as (typeof STATUSES)[number] | ''
  // «Affittato» non è un filtro sullo stato ma sull'occupazione: lo stato
  // memorizzato non lo porta, e chiederlo all'API risponderebbe `422`.
  const occupancy = params.get('occupazione') ?? ''

  const query = usePagedQuery({
    queryKey: keys.properties({ status, occupancy }),
    fetchPage: (after) =>
      listProperties(client, {
        status: status || undefined,
        ...(occupancy ? { occupied: occupancy === 'occupati' } : {}),
        after,
      }),
  })

  // Archiviare è dello staff: a chi abita la colonna non compare. Le righe
  // caricate sono tutte archiviabili, perché un immobile archiviato non è in
  // nessun elenco.
  const selection = useSelection(isStaff ? (query.data ?? []).map((property) => property.id) : [])
  const archive = useBulkAction({
    selection,
    action: (id) => archiveProperty(client, id),
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: ['properties'] })
      await queryClient.invalidateQueries({ queryKey: keys.dashboard })
    },
  })

  return (
    <>
      <PageHeader
        title={t.property.title}
        actions={
          /* Registrare un immobile è dello staff: `CreateProperty` è
             `ensure_staff`, e offrirlo a chi abita sarebbe un bottone che
             risponde sempre `403`. */
          isStaff ? (
            <Button asChild variant="primary">
              <Link to="/immobili/nuovo">
                <Plus aria-hidden />
                {t.property.create}
              </Link>
            </Button>
          ) : null
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

          {/* L'occupazione è un filtro a sé perché **non è uno stato**: discende
              dai contratti in essere, e il valore corrispondente non esiste in
              database. È la stessa forma di `in_force` sui contratti. */}
          <label htmlFor="filtro-occupazione" className="text-xs font-medium text-ink-muted">
            {t.property.occupancy}
          </label>
          <Select
            id="filtro-occupazione"
            className="w-auto min-w-40"
            value={occupancy}
            onChange={(event) => {
              const next = new URLSearchParams(params)
              if (event.target.value) next.set('occupazione', event.target.value)
              else next.delete('occupazione')
              setParams(next, { replace: true })
            }}
          >
            <option value="">{t.common.all}</option>
            <option value="occupati">{t.property.filterOccupied}</option>
            <option value="liberi">{t.property.filterFree}</option>
          </Select>

          {/* Come sui contratti: il badge dice ciò che i contratti in essere
              producono, il filtro chiede lo stato dichiarato, e la spiegazione sta
              dove nasce la domanda invece che fra parentesi su ogni riga. */}
          {status ? (
            <p className="w-full text-xs text-ink-muted">{t.property.filteredByDeclaredStatus}</p>
          ) : null}
        </div>

        {isStaff ? (
          <BulkBar selection={selection} verb={ARCHIVE} bulk={archive} icon={<Archive aria-hidden />} />
        ) : null}

        <Async
          query={query}
          isEmpty={(items) => items.length === 0}
          empty={
            <Empty
              title={status || occupancy ? t.state.filteredTitle : t.property.empty}
              hint={
                status || occupancy
                  ? t.state.filteredHint
                  : isStaff
                    ? t.property.emptyHint
                    : t.property.emptyForTenant
              }
              {...(isStaff && !status && !occupancy
                ? {
                    action: (
                      <Button asChild variant="primary">
                        <Link to="/immobili/nuovo">{t.property.create}</Link>
                      </Button>
                    ),
                  }
                : {})}
            />
          }
        >
          {(items) => (
            <TableFrame>
              <TableCaption>{t.property.title}</TableCaption>
              <thead>
                <tr>
                  {isStaff ? <ThSelect selection={selection} /> : null}
                  <Th>{t.property.label}</Th>
                  <Th>{t.property.type}</Th>
                  <Th>{t.property.city}</Th>
                  <Th>{t.property.status}</Th>
                  <ThNumeric>{t.property.surface}</ThNumeric>
                  {isStaff ? (
                    <Th>
                      <span className="sr-only">{t.common.actions}</span>
                    </Th>
                  ) : null}
                </tr>
              </thead>
              <tbody>
                {items.map((property) => (
                  <Tr key={property.id}>
                    {isStaff ? (
                      <TdSelect
                        selection={selection}
                        id={property.id}
                        label={property.label}
                        failure={archive.failureFor(property.id)}
                      />
                    ) : null}
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
                      <PropertyStatusBadge
                        status={property.status}
                        effectiveStatus={property.effective_status}
                      />
                    </Td>
                    <TdNumeric>
                      {property.size_sqm ? `${formatDecimal(property.size_sqm)} m²` : t.common.none}
                    </TdNumeric>
                    {isStaff ? (
                      <Td className="text-right whitespace-nowrap">
                        {/* La riga singola è un giro di lunghezza uno: stesso
                            verbo, stessa conferma, stesso esito accanto alla riga. */}
                        <VerbButton
                          verb={ARCHIVE}
                          ids={[property.id]}
                          run={archive.run}
                          rowLabel={property.label}
                        />
                      </Td>
                    ) : null}
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
