import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Archive, Ban, Check, LogOut, Undo2, X } from 'lucide-react'
import { Link, useNavigate, useParams } from 'react-router'

import {
  archiveExpenseGroup,
  cancelExpense,
  cancelSettlement,
  confirmSettlement,
  getGroupBalances,
  getExpenseGroup,
  leaveGroup,
  listExpenses,
  listSettlements,
  rejectSettlement,
} from '@/api/endpoints'
import type {
  Expense,
  ExpenseGroup,
  ExpenseGroupMember,
  GroupBalance,
  Settlement,
  SettlementStatus,
} from '@/api/types'
import { Async, Empty } from '@/components/state'
import { Badge, type BadgeTone } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardBody, CardHeader, CardTitle } from '@/components/ui/card'
import { ConfirmAction } from '@/components/ui/confirm'
import { PageHeader, PageSection } from '@/components/ui/page'
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
import { useApi, useIsStaff, useSession } from '@/app/session'
import { t } from '@/i18n'
import { compareDecimals } from '@/lib/decimal'
import { formatAmount, formatDate, formatDateTime, formatDecimal } from '@/lib/format'
import { BulkBar, useBulkAction, VerbButton, type Verb } from '../shared/bulk'
import { LoadMore, usePagedQuery } from '../shared/pagination'
import { AddMemberDialog } from './AddMemberDialog'
import { balancesSumToZero, NetAmount } from './balance'
import { DeclareSettlementDialog } from './DeclareSettlementDialog'
import { ExpenseDetailDialog } from './ExpenseDetailDialog'
import { useMemberNames, type NameOf } from './names'
import { RecordExpenseDialog } from './RecordExpenseDialog'

/**
 * La scheda di un gruppo: i saldi, le spese, i rimborsi, chi c'è dentro.
 *
 * # I saldi per primi
 *
 * È la domanda con cui si apre questa pagina — *a che punto siamo* — e le spese
 * sono il come ci si è arrivati. L'ordine sullo schermo stretto è quello, e non
 * l'ordine in cui i dati sono stati costruiti.
 *
 * # Un gruppo archiviato resta leggibile, e non muto
 *
 * L'archiviazione chiude le scritture, non la lettura: spese e saldi restano, e i
 * conti aperti restano aperti. Spariscono i due bottoni che otterrebbero `409
 * expense_group.archived`; **resta quello che aggiunge un membro**, perché
 * riportare dentro chi è uscito con un debito è il modo in cui quella persona può
 * tornare a vederlo — e su un conto chiuso è esattamente il caso che capita.
 *
 * # Il ruolo qui decide una cosa sola
 *
 * Quasi tutto il modulo è `ensure_member`, e a distinguere chi può fare che cosa
 * sono l'appartenenza al gruppo e — per i rimborsi — l'essere il destinatario o
 * l'autore della dichiarazione. Si instrada sugli identificatori, non sul ruolo.
 * L'unica eccezione è l'annullamento di una spesa, che lo staff può fare su
 * qualunque riga: è `CancelExpense`, e `ExpensesCard` lo sa.
 */
export function ExpenseGroupDetailPage() {
  const { id = '' } = useParams()
  const client = useApi()
  const { session } = useSession()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const userId = session?.userId ?? null

  const group = useQuery({
    queryKey: keys.expenseGroup(id),
    queryFn: () => getExpenseGroup(client, id),
  })

  const balances = useQuery({
    queryKey: keys.groupBalances(id),
    queryFn: () => getGroupBalances(client, id),
  })

  const nameOf = useMemberNames(group.data?.members, group.data?.property_id)

  const leave = useMutation({
    mutationFn: () => leaveGroup(client, id, userId ?? ''),
    onSuccess: async () => {
      // Da qui il gruppo non è più leggibile: `can_access_expense_group` esige una
      // membership attiva. Restare sulla scheda mostrerebbe un `404` al primo
      // aggiornamento, quindi si torna all'elenco.
      await queryClient.invalidateQueries({ queryKey: ['expense-groups'] })
      queryClient.removeQueries({ queryKey: keys.expenseGroup(id) })
      queryClient.removeQueries({ queryKey: keys.groupBalances(id) })
      await navigate('/spese')
    },
  })

  const archive = useMutation({
    mutationFn: () => archiveExpenseGroup(client, id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.expenseGroup(id) })
      await queryClient.invalidateQueries({ queryKey: ['expense-groups'] })
    },
  })

  return (
    <Async query={group}>
      {(data) => (
        <>
          <PageHeader
            title={data.name}
            description={data.is_archived ? t.expense.archivedNote : t.expense.kinds[data.kind]}
            breadcrumb={
              <Link to="/spese" className="hover:underline">
                {t.expense.backToGroups}
              </Link>
            }
            actions={
              <>
                {data.is_archived ? null : (
                  <>
                    <RecordExpenseDialog group={data} />
                    <DeclareSettlementDialog
                      group={data}
                      balances={balances.data?.items ?? []}
                      nameOf={nameOf}
                      userId={userId}
                    />
                  </>
                )}
                {data.is_archived ? null : (
                  <ConfirmAction
                    trigger={
                      <Button variant="dangerGhost">
                        <Archive aria-hidden />
                        {t.expense.archive}
                      </Button>
                    }
                    title={t.expense.archiveConfirm}
                    explanation={t.expense.archiveExplain}
                    confirmLabel={t.expense.archive}
                    onConfirm={() => archive.mutateAsync()}
                  />
                )}
              </>
            }
          />

          <PageSection>
            <BalancesCard group={data} balances={balances} nameOf={nameOf} userId={userId} />
          </PageSection>

          <PageSection>
            <ExpensesCard group={data} nameOf={nameOf} />
          </PageSection>

          <PageSection>
            <SettlementsCard group={data} nameOf={nameOf} userId={userId} />
          </PageSection>

          <PageSection>
            <MembersCard
              group={data}
              nameOf={nameOf}
              userId={userId}
              onLeave={() => leave.mutateAsync()}
            />
          </PageSection>
        </>
      )}
    </Async>
  )
}

/**
 * I saldi, letti dalla vista e **non ricalcolati**.
 *
 * Arrivano ordinati — i creditori per primi — e il client non li riordina: due
 * ordinamenti dello stesso elenco si contraddicono al primo pareggio.
 *
 * **Chi è uscito con un conto aperto compare qui**, ed è ciò che fa sommare i
 * netti a zero. Un nome che non è fra i membri attivi è la prima domanda che chi
 * guarda si fa, quindi la riga lo dice: senza quella nota sembrerebbe un errore.
 */
function BalancesCard({
  group,
  balances,
  nameOf,
  userId,
}: {
  group: ExpenseGroup
  balances: {
    data: { items: GroupBalance[] } | undefined
    isPending: boolean
    error: unknown
    refetch: () => void
  }
  nameOf: NameOf
  userId: string | null
}) {
  const active = new Set(
    (group.members ?? []).filter((member) => !member.left_at).map((member) => member.user_id),
  )

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t.expense.balances}</CardTitle>
        <p className="text-xs text-ink-muted">{t.expense.balancesHint}</p>
      </CardHeader>

      <Async
        query={balances}
        isEmpty={(data) => data.items.length === 0}
        empty={<Empty title={t.expense.balancesEmpty} hint={t.expense.whoPaysWhom} />}
      >
        {(data) => (
          <>
            <TableFrame>
              <TableCaption>{t.expense.balances}</TableCaption>
              <thead>
                <tr>
                  <Th>{t.expense.person}</Th>
                  <ThNumeric>{t.expense.totalPaid}</ThNumeric>
                  <ThNumeric>{t.expense.totalOwed}</ThNumeric>
                  <ThNumeric>{t.expense.settledOut}</ThNumeric>
                  <ThNumeric>{t.expense.settledIn}</ThNumeric>
                  <ThNumeric>{t.expense.netBalance}</ThNumeric>
                </tr>
              </thead>
              <tbody>
                {data.items.map((balance) => (
                  <Tr key={balance.user_id}>
                    <Td>
                      <span className="font-medium">{nameOf(balance.user_id)}</span>
                      {balance.user_id === userId ? (
                        <span className="ml-1 text-xs text-ink-muted">({t.expense.you})</span>
                      ) : null}
                      {active.has(balance.user_id) ? null : (
                        <span className="mt-0.5 block text-xs text-warning">
                          {t.expense.balanceLeft}
                        </span>
                      )}
                    </Td>
                    <TdNumeric>
                      {formatAmount(balance.total_paid, { currency: group.currency })}
                    </TdNumeric>
                    <TdNumeric>
                      {formatAmount(balance.total_owed, { currency: group.currency })}
                    </TdNumeric>
                    <TdNumeric>
                      {formatAmount(balance.total_settled_out, { currency: group.currency })}
                    </TdNumeric>
                    <TdNumeric>
                      {formatAmount(balance.total_settled_in, { currency: group.currency })}
                    </TdNumeric>
                    <TdNumeric className="font-medium">
                      <NetAmount value={balance.net_balance} currency={group.currency} />
                    </TdNumeric>
                  </Tr>
                ))}
              </tbody>
            </TableFrame>

            <CardBody className="border-t border-border-subtle text-xs text-ink-muted">
              {/* La verifica passa da `@/lib/decimal`: l'aritmetica nativa
                  produrrebbe uno zero che a volte non è zero, cioè un avviso di
                  guasto su un gruppo che sta benissimo. */}
              <p className={balancesSumToZero(data.items) ? undefined : 'text-danger'}>
                {balancesSumToZero(data.items)
                  ? t.expense.balancesSumZero
                  : t.expense.balancesSumBroken}
              </p>
              <p className="mt-1">{t.expense.whoPaysWhom}</p>
            </CardBody>
          </>
        )}
      </Async>
    </Card>
  )
}

/**
 * Il verbo della spesa è **Annulla**, ed è il soft delete: la spesa non ha uno
 * stato. La conferma dice ciò a cui chi annulla non pensa — i rimborsi già
 * confermati restano, e i saldi si spostano.
 */
const CANCEL_EXPENSE: Verb = {
  label: t.expense.cancel,
  confirmOne: t.expense.cancelConfirm,
  confirmMany: t.expense.cancelConfirmMany,
  explain: t.expense.cancelExplain,
}

/**
 * Le spese del gruppo, paginate a cursore.
 *
 * L'elenco **non porta le quote**: si leggono dal dettaglio, che è la modale. È la
 * stessa divisione del thread di una segnalazione — ciò che non ha un tetto sta
 * dietro la sua richiesta.
 *
 * # Chi può annullare, come lo dice l'API
 *
 * Chi l'ha registrata, chi l'ha anticipata, o lo staff: è il confine di
 * `CancelExpense`, e a chi non ci rientra l'API risponde `403 expense.not_yours`.
 * Qui si evita di offrire ciò che verrebbe rifiutato, e su un conto chiuso non si
 * offre a nessuno: disfare una spesa è una scrittura sul conto.
 */
function ExpensesCard({ group, nameOf }: { group: ExpenseGroup; nameOf: NameOf }) {
  const client = useApi()
  const isStaff = useIsStaff()
  const { session } = useSession()
  const queryClient = useQueryClient()
  const query = usePagedQuery({
    queryKey: keys.expenses(group.id),
    fetchPage: (after) => listExpenses(client, group.id, { after }),
  })

  const userId = session?.userId
  const cancellable = (expense: Expense) =>
    !group.is_archived &&
    (isStaff || expense.paid_by_user_id === userId || expense.created_by === userId)

  const selection = useSelection(
    (query.data ?? []).filter(cancellable).map((expense) => expense.id),
  )
  const discard = useBulkAction({
    selection,
    action: (id) => cancelExpense(client, id),
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.expenses(group.id) })
      // I saldi si leggono dalla vista, e la spesa ne è appena uscita.
      await queryClient.invalidateQueries({ queryKey: keys.groupBalances(group.id) })
    },
  })
  const column = selection.total > 0

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t.expense.expenses}</CardTitle>
      </CardHeader>
      <BulkBar selection={selection} verb={CANCEL_EXPENSE} bulk={discard} icon={<Ban aria-hidden />} />

      <Async
        query={query}
        isEmpty={(items) => items.length === 0}
        empty={
          <Empty
            title={t.expense.expensesEmpty}
            {...(group.is_archived ? {} : { hint: t.expense.expensesEmptyHint })}
          />
        }
      >
        {(items) => (
          <TableFrame>
            <TableCaption>{t.expense.expenses}</TableCaption>
            <thead>
              <tr>
                {column ? <ThSelect selection={selection} /> : null}
                <Th>{t.expense.expenseDate}</Th>
                <Th>{t.expense.description}</Th>
                <Th>{t.expense.category}</Th>
                <Th>{t.expense.paidBy}</Th>
                <ThNumeric>{t.expense.amount}</ThNumeric>
                <Th>
                  <span className="sr-only">{t.common.actions}</span>
                </Th>
              </tr>
            </thead>
            <tbody>
              {items.map((expense: Expense) => (
                <Tr key={expense.id}>
                  {column ? (
                    <TdSelect
                      selection={selection}
                      id={expense.id}
                      label={expense.description}
                      failure={discard.failureFor(expense.id)}
                      disabled={!cancellable(expense)}
                    />
                  ) : null}
                  <Td className="whitespace-nowrap">{formatDate(expense.expense_date)}</Td>
                  <Td>{expense.description}</Td>
                  <Td>{t.expense.categories[expense.category]}</Td>
                  <Td>{nameOf(expense.paid_by_user_id)}</Td>
                  <TdNumeric>
                    {formatAmount(expense.amount, { currency: expense.currency })}
                  </TdNumeric>
                  <Td className="text-right whitespace-nowrap">
                    <span className="inline-flex gap-1">
                      <ExpenseDetailDialog expense={expense} nameOf={nameOf} />
                      {cancellable(expense) ? (
                        <VerbButton
                          verb={CANCEL_EXPENSE}
                          ids={[expense.id]}
                          run={discard.run}
                          rowLabel={expense.description}
                        />
                      ) : null}
                    </span>
                  </Td>
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
  )
}

const settlementTone: Record<SettlementStatus, BadgeTone> = {
  pending: 'warning',
  confirmed: 'success',
  rejected: 'danger',
  cancelled: 'muted',
}

/**
 * I rimborsi, e le **tre** uscite da `pending`.
 *
 * È la differenza con i pagamenti al proprietario, dove le uscite sono due:
 * `settlements_update` nomina anche `from_user_id`, quindi **chi ha dichiarato può
 * ritirare**. Chi arriva dall'altra schermata si aspetta il contrario, ed è la
 * ragione per cui il bottone c'è e la nota lo dice.
 *
 * Le tre azioni si instradano sugli identificatori e non sul ruolo: confermare e
 * rifiutare sono di **chi riceve** — è l'unico che sa se il denaro è arrivato —
 * ritirare di **chi ha dichiarato**. Offrirle a chiunque altro produrrebbe
 * `403 settlement.not_addressed_to_you` o `403 settlement.not_declared_by_you`.
 */
function SettlementsCard({
  group,
  nameOf,
  userId,
}: {
  group: ExpenseGroup
  nameOf: NameOf
  userId: string | null
}) {
  const client = useApi()
  const query = usePagedQuery({
    queryKey: keys.settlements(group.id),
    fetchPage: (after) => listSettlements(client, group.id, { after }),
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t.expense.settlements}</CardTitle>
        <p className="text-xs text-ink-muted">{t.expense.onlyReceiverDecides}</p>
      </CardHeader>

      <Async
        query={query}
        isEmpty={(items) => items.length === 0}
        empty={
          <Empty
            title={t.expense.settlementsEmpty}
            {...(group.is_archived ? {} : { hint: t.expense.settlementsEmptyHint })}
          />
        }
      >
        {(items) => (
          <TableFrame>
            <TableCaption>{t.expense.settlements}</TableCaption>
            <thead>
              <tr>
                <Th>{t.expense.declaredAt}</Th>
                <Th>{t.expense.settledAtColumn}</Th>
                <Th>{t.expense.from}</Th>
                <Th>{t.expense.to}</Th>
                <ThNumeric>{t.expense.amount}</ThNumeric>
                <Th>{t.payment.status}</Th>
                <Th>
                  <span className="sr-only">{t.common.actions}</span>
                </Th>
              </tr>
            </thead>
            <tbody>
              {items.map((settlement: Settlement) => (
                <SettlementRow
                  key={settlement.id}
                  settlement={settlement}
                  groupId={group.id}
                  nameOf={nameOf}
                  userId={userId}
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
  )
}

function SettlementRow({
  settlement,
  groupId,
  nameOf,
  userId,
}: {
  settlement: Settlement
  groupId: string
  nameOf: NameOf
  userId: string | null
}) {
  const client = useApi()
  const queryClient = useQueryClient()

  /**
   * Confermare **non ricalcola niente**, a differenza della conferma di un
   * pagamento: il saldo di un gruppo non è memorizzato da nessuna parte, si legge
   * dalla vista. Si invalida la chiave dei saldi e basta — chi arriva dal modulo
   * dei pagamenti cerca un `recalculate_*` che qui non serve e non deve esistere.
   */
  async function refresh() {
    await queryClient.invalidateQueries({ queryKey: keys.settlements(groupId) })
    await queryClient.invalidateQueries({ queryKey: keys.groupBalances(groupId) })
  }

  const confirm = useMutation({
    mutationFn: () => confirmSettlement(client, settlement.id),
    onSuccess: refresh,
  })
  const reject = useMutation({
    mutationFn: () => rejectSettlement(client, settlement.id),
    onSuccess: refresh,
  })
  const withdraw = useMutation({
    mutationFn: () => cancelSettlement(client, settlement.id),
    onSuccess: refresh,
  })

  const pending = settlement.status === 'pending'
  const mineToConfirm = pending && settlement.to_user_id === userId
  const mineToWithdraw = pending && settlement.from_user_id === userId

  return (
    <Tr>
      {/* Quando la riga è arrivata: `created_at`, che c'è **sempre**. */}
      <Td className="whitespace-nowrap">{formatDateTime(settlement.created_at)}</Td>
      {/* Quando il denaro si è mosso: `settled_at`, che finché il rimborso è in
          attesa **non c'è** — resta nullo fino alla conferma, che lo fa ricadere su
          quel momento — e che le due uscite negative azzerano. Due colonne e non
          una, per la ragione già trovata sui pagamenti: una sola sarebbe vuota
          proprio nel caso comune. */}
      <Td className="whitespace-nowrap">
        {settlement.settled_at ? formatDateTime(settlement.settled_at) : t.common.none}
      </Td>
      <Td>{nameOf(settlement.from_user_id)}</Td>
      <Td>{nameOf(settlement.to_user_id)}</Td>
      <TdNumeric>{formatAmount(settlement.amount, { currency: settlement.currency })}</TdNumeric>
      <Td>
        <Badge tone={settlementTone[settlement.status]}>
          {t.expense.settlementStatuses[settlement.status]}
        </Badge>
        {mineToConfirm ? (
          <span className="mt-0.5 block text-xs text-warning">{t.expense.waitingYou}</span>
        ) : pending ? (
          <span className="mt-0.5 block text-xs text-ink-muted">
            {t.expense.pendingDoesNotMove}
          </span>
        ) : null}
      </Td>
      <Td className="text-right whitespace-nowrap">
        {mineToConfirm ? (
          <span className="inline-flex gap-1">
            <ConfirmAction
              trigger={
                <Button size="sm" variant="ghost">
                  <Check aria-hidden />
                  {t.expense.confirm}
                </Button>
              }
              title={t.expense.confirmTitle}
              explanation={t.expense.confirmExplain}
              confirmLabel={t.expense.confirm}
              variant="primary"
              onConfirm={() => confirm.mutateAsync()}
            />
            <ConfirmAction
              trigger={
                <Button size="sm" variant="ghost">
                  <X aria-hidden />
                  {t.expense.reject}
                </Button>
              }
              title={t.expense.rejectTitle}
              explanation={t.expense.rejectExplain}
              confirmLabel={t.expense.reject}
              onConfirm={() => reject.mutateAsync()}
            />
          </span>
        ) : null}
        {mineToWithdraw ? (
          <ConfirmAction
            trigger={
              <Button size="sm" variant="ghost">
                <Undo2 aria-hidden />
                {t.expense.withdraw}
              </Button>
            }
            title={t.expense.withdrawTitle}
            explanation={t.expense.withdrawExplain}
            confirmLabel={t.expense.withdraw}
            onConfirm={() => withdraw.mutateAsync()}
          />
        ) : null}
      </Td>
    </Tr>
  )
}

/**
 * Chi c'è dentro, **compreso chi è uscito**.
 *
 * La riga di chi esce non sparisce: le quote a suo nome restano, e i saldi partono
 * da lì. Mostrarla è anche l'unico modo di spiegare un nome che compare nei saldi
 * e non fra i membri attivi.
 *
 * L'uscita si offre **solo per sé stessi**: a togliere qualcun altro serve lo
 * staff, e a un membro l'API risponde `403 expense_group.membership_not_yours` —
 * dalla migration `0029` è la Row Level Security a rifiutare, non solo l'API.
 */
function MembersCard({
  group,
  nameOf,
  userId,
  onLeave,
}: {
  group: ExpenseGroup
  nameOf: NameOf
  userId: string | null
  onLeave: () => Promise<unknown>
}) {
  const members = group.members ?? []
  const mine = members.find((member) => member.user_id === userId && !member.left_at)

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t.expense.members}</CardTitle>
        <div className="flex flex-wrap gap-2">
          <AddMemberDialog group={group} />
          {mine ? (
            <ConfirmAction
              trigger={
                <Button size="sm" variant="dangerGhost">
                  <LogOut aria-hidden />
                  {t.expense.leave}
                </Button>
              }
              title={t.expense.leaveConfirm}
              explanation={t.expense.leaveExplain}
              confirmLabel={t.expense.leave}
              onConfirm={onLeave}
            />
          ) : null}
        </div>
      </CardHeader>

      {members.length === 0 ? (
        <Empty />
      ) : (
        <ul className="divide-y divide-border-subtle">
          {members.map((member) => (
            <MemberRow key={member.user_id} member={member} nameOf={nameOf} userId={userId} />
          ))}
        </ul>
      )}

      <CardBody className="border-t border-border-subtle text-xs text-ink-muted">
        <p>{t.expense.leaveOnlyYourself}</p>
      </CardBody>
    </Card>
  )
}

function MemberRow({
  member,
  nameOf,
  userId,
}: {
  member: ExpenseGroupMember
  nameOf: NameOf
  userId: string | null
}) {
  return (
    <li className="flex flex-wrap items-center justify-between gap-2 px-4 py-2.5 text-sm">
      <span className="min-w-0">
        <span className="font-medium">{nameOf(member.user_id)}</span>
        {member.user_id === userId ? (
          <span className="ml-1 text-xs text-ink-muted">({t.expense.you})</span>
        ) : null}
      </span>
      <span className="flex flex-wrap items-center gap-3 text-xs text-ink-muted">
        {/* Il peso si mostra solo quando dice qualcosa: `1` su una divisione in
            parti uguali è il default di tutti, e ripeterlo su ogni riga toglie
            attenzione a chi ha davvero la doppia. */}
        {compareDecimals(member.weight, '1') === 0 ? null : (
          <span className="tabular">
            {t.expense.memberWeight}: {formatDecimal(member.weight, { maximumFractionDigits: 3 })}
          </span>
        )}
        {member.left_at ? (
          <Badge tone="muted">
            {t.expense.memberLeftOn} {formatDateTime(member.left_at)}
          </Badge>
        ) : (
          <span>
            {t.expense.memberSince} {formatDateTime(member.joined_at)}
          </span>
        )}
      </span>
    </li>
  )
}
