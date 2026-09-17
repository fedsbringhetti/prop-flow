import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Ban } from 'lucide-react'
import { Link, useSearchParams } from 'react-router'

import { listCharges, voidCharge, writeOffCharge } from '@/api/endpoints'
import type { ChargeFilters } from '@/api/endpoints'
import type { Charge } from '@/api/types'
import { Async, Empty } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { ConfirmAction } from '@/components/ui/confirm'
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
  type Selection,
} from '@/components/ui/table'
import { keys } from '@/app/query'
import { useApi, useIsStaff } from '@/app/session'
import { t } from '@/i18n'
import { isNegative } from '@/lib/decimal'
import { formatAmount, formatDate } from '@/lib/format'
import { BulkBar, useBulkAction, VerbButton, type Verb } from '../shared/bulk'
import { LoadMore, usePagedQuery } from '../shared/pagination'
import { ChargeStatusBadge } from '../shared/status'

/**
 * Gli stati **memorizzati**, e sono questi cinque.
 *
 * `overdue` non c'è, ed è la trappola: è derivato dalla scadenza, e
 * `?status=overdue` risponde `422` con `charge.derived_status_filter`. `draft`
 * nemmeno: esiste nell'enum del database e l'applicazione non lo scrive mai, quindi
 * offrirlo darebbe un elenco sempre vuoto.
 *
 * L'elenco è scritto a mano e **non derivato dall'enum dei tipi**: costruire un
 * menù da `ChargeStatus` ci farebbe finire dentro entrambi.
 */
const FILTERABLE = ['issued', 'partially_paid', 'paid', 'void', 'written_off'] as const

type FilterableStatus = (typeof FILTERABLE)[number]

function filterableStatus(raw: string | null): FilterableStatus | undefined {
  return FILTERABLE.find((status) => status === raw)
}

/**
 * Il verbo di questa tabella è **Annulla**, e non «Elimina»: un addebito
 * annullato resta visibile a chi doveva pagarlo — la `0023` nega `deleted_at`
 * apposta — e la conferma della scheda lo dice. Lo storno non entra nel blocco:
 * è un giudizio su una perdita, e resta per riga.
 */
const VOID: Verb = {
  label: t.charge.void,
  confirmOne: t.charge.voidConfirm,
  confirmMany: t.charge.voidConfirmMany,
  explain: t.charge.voidExplain,
}

/**
 * `void` è ammesso dal solo `issued`: su un addebito con del denaro dentro
 * direbbe che la richiesta non è mai esistita, e l'API risponde `409`. Le altre
 * righe non si offrono alla selezione — un bottone che produce sempre un errore
 * è peggio di un bottone assente — e la via per quelle è lo storno.
 */
function voidable(charge: Charge): boolean {
  return charge.status === 'issued'
}

/**
 * Lo scadenzario, e ogni altro elenco di addebiti.
 *
 * **Non c'è un filtro «scaduto»**, ed è deliberato: `overdue` è derivato dalla
 * scadenza e `?status=overdue` risponde `422` con `charge.derived_status_filter`.
 * Il filtro che serve è `?outstanding=true`, che enumera i tre stati dell'insoluto
 * e usa l'indice parziale che esiste per quello.
 *
 * Il filtro sullo stato memorizzato esiste invece per una ragione concreta: è la
 * destinazione del riquadro **Perdite** del riepilogo. Un numero che non porta da
 * nessuna parte è una decorazione, e senza questo filtro `written_off_total`
 * sarebbe uno di quelli.
 */
export function ChargesPage() {
  const client = useApi()
  const [params, setParams] = useSearchParams()
  const outstanding = params.get('insoluti') === '1'
  const status = filterableStatus(params.get('stato'))

  const filters: ChargeFilters = {
    ...(outstanding ? { outstanding: true } : {}),
    ...(status ? { status } : {}),
  }

  const isStaff = useIsStaff()
  const queryClient = useQueryClient()

  const query = usePagedQuery({
    queryKey: keys.charges({ outstanding, status: status ?? null }),
    fetchPage: (after) => listCharges(client, { ...filters, after }),
  })

  // Annullare è dello staff: chi la subisce non ha la colonna.
  const selection = useSelection(
    isStaff ? (query.data ?? []).filter(voidable).map((charge) => charge.id) : [],
  )
  const discard = useBulkAction({
    selection,
    action: (id) => voidCharge(client, id),
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: ['charges'] })
      await queryClient.invalidateQueries({ queryKey: keys.dashboard })
    },
  })

  return (
    <>
      {/* Per chi abita questa è la **pagina iniziale**, ed è la domanda che si
          fa davvero: quanto devo. La nota sugli stati derivati riguarda il
          filtro, che serve a entrambi. */}
      <PageHeader
        title={t.charge.title}
        description={isStaff ? t.charge.derivedStatusNote : t.charge.tenantSubtitle}
      />

      <Card>
        <div className="flex flex-wrap items-center gap-4 border-b border-border-subtle px-4 py-3">
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              className="size-4"
              checked={outstanding}
              onChange={(event) => {
                const next = new URLSearchParams(params)
                if (event.target.checked) next.set('insoluti', '1')
                else next.delete('insoluti')
                setParams(next, { replace: true })
              }}
            />
            {t.charge.outstandingOnly}
          </label>

          <label className="flex items-center gap-2 text-sm">
            <span className="text-ink-muted">{t.charge.status}</span>
            <select
              className="h-8 rounded-lg border border-border-subtle bg-surface px-2 pr-7 text-sm"
              value={status ?? ''}
              onChange={(event) => {
                const next = new URLSearchParams(params)
                if (event.target.value) next.set('stato', event.target.value)
                else next.delete('stato')
                setParams(next, { replace: true })
              }}
            >
              <option value="">{t.common.all}</option>
              {FILTERABLE.map((value) => (
                <option key={value} value={value}>
                  {t.charge.statuses[value]}
                </option>
              ))}
            </select>
          </label>
        </div>

        {isStaff ? (
          <BulkBar selection={selection} verb={VOID} bulk={discard} icon={<Ban aria-hidden />} />
        ) : null}

        <Async
          query={query}
          isEmpty={(items) => items.length === 0}
          empty={
            <Empty
              title={
                outstanding
                  ? t.charge.emptyOutstanding
                  : status
                    ? t.state.filteredTitle
                    : t.charge.empty
              }
              // Con un filtro attivo il vuoto è del filtro: dice come toglierlo,
              // non che cosa sia un addebito a chi ne ha già. Senza filtri è la
              // prima schermata di ogni inquilino, e spiega.
              hint={
                outstanding || status
                  ? t.state.filteredHint
                  : isStaff
                    ? t.charge.emptyHintStaff
                    : t.charge.emptyHintTenant
              }
              {...(isStaff && !outstanding && !status
                ? {
                    action: (
                      <Button asChild variant="primary">
                        <Link to="/contratti">{t.dashboard.goToLeases}</Link>
                      </Button>
                    ),
                  }
                : {})}
            />
          }
        >
          {(items) => (
            <TableFrame>
              <TableCaption>{t.charge.title}</TableCaption>
              <thead>
                <tr>
                  {isStaff ? <ThSelect selection={selection} /> : null}
                  <Th>{t.charge.type}</Th>
                  <Th>{t.charge.status}</Th>
                  <Th>{t.charge.dueDate}</Th>
                  <ThNumeric>{t.charge.amount}</ThNumeric>
                  <ThNumeric>{t.charge.paidAmount}</ThNumeric>
                  <ThNumeric>{t.charge.outstandingAmount}</ThNumeric>
                  <Th>
                    <span className="sr-only">{t.common.actions}</span>
                  </Th>
                </tr>
              </thead>
              <tbody>
                {items.map((charge) => (
                  <ChargeRow
                    key={charge.id}
                    charge={charge}
                    selection={selection}
                    failure={discard.failureFor(charge.id)}
                    runVoid={discard.run}
                  />
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

function ChargeRow({
  charge,
  selection,
  failure,
  runVoid,
}: {
  charge: Charge
  selection: Selection
  failure: string | undefined
  runVoid: (ids: readonly string[]) => Promise<unknown>
}) {
  const client = useApi()
  const isStaff = useIsStaff()
  const queryClient = useQueryClient()

  async function refresh() {
    await queryClient.invalidateQueries({ queryKey: ['charges'] })
    await queryClient.invalidateQueries({ queryKey: keys.dashboard })
  }

  const writeOff = useMutation({
    mutationFn: () => writeOffCharge(client, charge.id),
    onSuccess: refresh,
  })

  // Annullare e stornare sono dello staff: sono le due uscite della **pretesa**,
  // e chi la subisce non le ha. La sua uscita è dichiarare di aver pagato, ed è
  // sulla scheda dell'addebito — non in questo elenco, dove una riga per riga
  // moltiplicherebbe un gesto che si fa una volta e con l'importo davanti.
  const settled =
    charge.status === 'void' || charge.status === 'written_off' || charge.status === 'paid'
  const noActions = !isStaff || settled
  // Un residuo **negativo** è un credito verso l'inquilino: non compensa gli
  // altri insoluti e non compare nella dashboard, quindi almeno qui va detto.
  const credit = isNegative(charge.outstanding_amount)

  return (
    <Tr>
      {isStaff ? (
        <TdSelect
          selection={selection}
          id={charge.id}
          label={`${t.charge.types[charge.charge_type]} ${formatDate(charge.due_date)}`}
          failure={failure}
          disabled={!voidable(charge)}
        />
      ) : null}
      <Td>
        {/* Al **dettaglio dell'addebito**, e non più al contratto: è lì che si
            registra un incasso, ed è il gesto che questa fetta ha aggiunto. Il
            contratto resta raggiungibile da quella scheda. */}
        <Link
          to={`/addebiti/${charge.id}`}
          className="font-medium text-accent underline-offset-4 hover:underline"
        >
          {t.charge.types[charge.charge_type]}
        </Link>
        {charge.utility_bill_id ? (
          <Link
            to={`/bollette/${charge.utility_bill_id}`}
            className="block text-xs text-ink-muted underline-offset-4 hover:underline"
          >
            {t.charge.fromBill}
          </Link>
        ) : null}
      </Td>
      <Td>
        <ChargeStatusBadge status={charge.status} effectiveStatus={charge.effective_status} />
      </Td>
      <Td>{formatDate(charge.due_date)}</Td>
      <TdNumeric>{formatAmount(charge.amount, { currency: charge.currency })}</TdNumeric>
      <TdNumeric>{formatAmount(charge.paid_amount, { currency: charge.currency })}</TdNumeric>
      <TdNumeric className={credit ? 'text-success' : undefined}>
        {formatAmount(charge.outstanding_amount, { currency: charge.currency })}
        {/* Il colore da solo non basta: chi non lo distingue vedrebbe un numero
            negativo senza sapere che è un credito. La parola sta accanto. */}
        {credit ? (
          <span className="block text-xs font-medium">{t.charge.credit}</span>
        ) : null}
      </TdNumeric>
      <Td className="text-right whitespace-nowrap">
        {noActions ? null : (
          <span className="inline-flex gap-1">
            {/* La riga singola passa dallo stesso giro del blocco: stesso verbo,
                stessa conferma, stesso esito accanto alla riga. */}
            {voidable(charge) ? (
              <VerbButton
                verb={VOID}
                ids={[charge.id]}
                run={runVoid}
                rowLabel={`${t.charge.types[charge.charge_type]} ${formatDate(charge.due_date)}`}
              />
            ) : null}
            <ConfirmAction
              trigger={
                <Button size="sm" variant="ghost">
                  {t.charge.writeOff}
                </Button>
              }
              title={t.charge.writeOffConfirm}
              explanation={t.charge.writeOffExplain}
              confirmLabel={t.charge.writeOff}
              onConfirm={() => writeOff.mutateAsync()}
            />
          </span>
        )}
      </Td>
    </Tr>
  )
}
