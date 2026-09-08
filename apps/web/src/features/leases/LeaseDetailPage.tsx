import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Copy, Plus, UserPlus } from 'lucide-react'
import { useState } from 'react'
import { Link, useParams } from 'react-router'

import {
  activateLease,
  getLease,
  addLeaseParty,
  inviteTenant,
  revokeInvitation,
  issueCharge,
  listCharges,
  terminateLease,
} from '@/api/endpoints'
import type { Invitation, LeasePartyRole } from '@/api/types'
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
import { formatAmount, formatDate, formatDecimal, todayIso } from '@/lib/format'
import { ChargeStatusBadge, LeaseStatusBadge } from '../shared/status'

const PARTY_ROLES: LeasePartyRole[] = ['tenant', 'cotenant', 'guarantor']

export function LeaseDetailPage() {
  const { id = '' } = useParams()
  const client = useApi()
  const isStaff = useIsStaff()
  const queryClient = useQueryClient()

  const lease = useQuery({
    queryKey: keys.lease(id),
    queryFn: () => getLease(client, id),
  })

  async function refresh() {
    await queryClient.invalidateQueries({ queryKey: keys.lease(id) })
    await queryClient.invalidateQueries({ queryKey: ['leases'] })
    await queryClient.invalidateQueries({ queryKey: keys.dashboard })
  }

  const activate = useMutation({
    mutationFn: () => activateLease(client, id),
    onSuccess: refresh,
  })
  const terminate = useMutation({
    mutationFn: (reason: string) => terminateLease(client, id, reason || undefined),
    onSuccess: refresh,
  })

  return (
    <Async query={lease}>
      {(data) => (
        <>
          <PageHeader
            title={t.lease.types[data.lease_type]}
            description={`${formatDate(data.start_date)} — ${formatDate(data.end_date)}`}
            breadcrumb={
              <Link to="/contratti" className="hover:underline">
                {t.lease.title}
              </Link>
            }
            actions={
              /* Invitare, attivare e disdire sono dello staff. Per chi abita
                 questa è la scheda del **proprio** contratto — `GetLease` è
                 `ensure_member` e la policy la restringe al suo — ed è la sola
                 pagina da cui si arriva all'immobile, che dalla sua navigazione
                 esce. */
              !isStaff ? null : (
              <>
                <InviteTenantDialog leaseId={id} />
                {data.status === 'draft' || data.status === 'pending_signature' ? (
                  <ConfirmAction
                    trigger={<Button variant="primary">{t.lease.activate}</Button>}
                    title={t.lease.activateConfirm}
                    explanation={t.lease.activateExplain}
                    confirmLabel={t.lease.activate}
                    variant="primary"
                    onConfirm={() => activate.mutateAsync()}
                  />
                ) : null}
                {data.status === 'active' ? <TerminateAction onConfirm={terminate.mutateAsync} /> : null}
              </>
              )
            }
          />

          <PageSection>
            <Card>
              <CardHeader>
                <CardTitle>{t.lease.title}</CardTitle>
                <LeaseStatusBadge status={data.status} effectiveStatus={data.effective_status} />
              </CardHeader>
              <CardBody>
                <DescriptionList>
                  <DescriptionItem label={t.lease.property}>
                    <Link
                      to={`/immobili/${data.property_id}`}
                      className="text-accent underline-offset-4 hover:underline"
                    >
                      {t.property.title}
                    </Link>
                  </DescriptionItem>
                  <DescriptionItem label={t.lease.rentAmount} numeric>
                    {formatAmount(data.rent_amount, { currency: data.currency })}
                  </DescriptionItem>
                  <DescriptionItem label={t.lease.depositAmount} numeric>
                    {formatAmount(data.deposit_amount, { currency: data.currency })}
                  </DescriptionItem>
                  <DescriptionItem label={t.lease.paymentDay} numeric>
                    {data.rent_due_day}
                  </DescriptionItem>
                  <DescriptionItem label={t.lease.startDate}>
                    {formatDate(data.start_date)}
                  </DescriptionItem>
                  <DescriptionItem label={t.lease.endDate}>
                    {formatDate(data.end_date)}
                  </DescriptionItem>
                </DescriptionList>
              </CardBody>
            </Card>
          </PageSection>

          <PageSection>
            <Card>
              <CardHeader>
                <CardTitle>{t.lease.parties}</CardTitle>
                {isStaff && (data.status === 'draft' || data.status === 'pending_signature') ? (
                  <AddPartyDialog leaseId={id} />
                ) : null}
              </CardHeader>
              {data.parties.length === 0 ? (
                <Empty title={t.lease.noTenantParty} hint={t.lease.activateExplain} />
              ) : (
                <TableFrame>
                  <TableCaption>{t.lease.parties}</TableCaption>
                  <thead>
                    <tr>
                      <Th>{t.lease.partyName}</Th>
                      <Th>{t.lease.partyRole}</Th>
                      <Th>{t.lease.partyEmail}</Th>
                      <ThNumeric>{t.lease.partyShare}</ThNumeric>
                    </tr>
                  </thead>
                  <tbody>
                    {data.parties.map((party) => (
                      <Tr key={party.id}>
                        <Td>
                          {party.full_name}
                          {party.is_primary ? (
                            <span className="ml-2 text-xs text-ink-muted">
                              ({t.lease.partyPrimary})
                            </span>
                          ) : null}
                        </Td>
                        <Td>{t.lease.partyRoles[party.role]}</Td>
                        <Td>{party.email ?? t.common.none}</Td>
                        <TdNumeric>{formatDecimal(party.share_percent, { maximumFractionDigits: 2 })} %</TdNumeric>
                      </Tr>
                    ))}
                  </tbody>
                </TableFrame>
              )}
            </Card>
          </PageSection>

          <PageSection>
            <LeaseChargesCard leaseId={id} currency={data.currency} canIssue={isStaff} />
          </PageSection>

        </>
      )}
    </Async>
  )
}

function TerminateAction({ onConfirm }: { onConfirm: (reason: string) => Promise<unknown> }) {
  const [reason, setReason] = useState('')
  return (
    <ConfirmAction
      trigger={<Button variant="dangerGhost">{t.lease.terminate}</Button>}
      title={t.lease.terminateConfirm}
      explanation={t.lease.terminateExplain}
      confirmLabel={t.lease.terminate}
      onConfirm={() => onConfirm(reason)}
    >
      <Field label={t.lease.terminationDate} hint={t.common.optional}>
        {(props) => (
          <Input {...props} value={reason} onChange={(event) => setReason(event.target.value)} />
        )}
      </Field>
    </ConfirmAction>
  )
}

function AddPartyDialog({ leaseId }: { leaseId: string }) {
  const client = useApi()
  const queryClient = useQueryClient()
  const [open, setOpen] = useState(false)
  const [error, setError] = useState<unknown>(null)
  const [values, setValues] = useState({
    full_name: '',
    email: '',
    role: 'tenant' as LeasePartyRole,
    share_percent: '100',
    is_primary: true,
  })

  const mutation = useMutation({
    mutationFn: () =>
      addLeaseParty(client, leaseId, {
        role: values.role,
        full_name: values.full_name,
        ...(values.email ? { email: values.email } : {}),
        ...(values.share_percent ? { share_percent: values.share_percent } : {}),
        is_primary: values.is_primary,
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.lease(leaseId) })
      setValues({ full_name: '', email: '', role: 'tenant', share_percent: '100', is_primary: false })
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
          {t.lease.addParty}
        </Button>
      }
      title={t.lease.addParty}
      description={t.lease.sharesNotHundred}
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
        <Field label={t.lease.partyName} required>
          {(props) => (
            <Input
              {...props}
              value={values.full_name}
              onChange={(event) => setValues((v) => ({ ...v, full_name: event.target.value }))}
            />
          )}
        </Field>
        <Field label={t.lease.partyEmail} hint={t.invitation.tokenExplain}>
          {(props) => (
            <Input
              {...props}
              type="email"
              value={values.email}
              onChange={(event) => setValues((v) => ({ ...v, email: event.target.value }))}
            />
          )}
        </Field>
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <Field label={t.lease.partyRole}>
            {(props) => (
              <Select
                {...props}
                value={values.role}
                onChange={(event) =>
                  setValues((v) => ({ ...v, role: event.target.value as LeasePartyRole }))
                }
              >
                {PARTY_ROLES.map((role) => (
                  <option key={role} value={role}>
                    {t.lease.partyRoles[role]}
                  </option>
                ))}
              </Select>
            )}
          </Field>
          <Field label={t.lease.partyShare}>
            {(props) => (
              <Input
                {...props}
                inputMode="decimal"
                value={values.share_percent}
                onChange={(event) => setValues((v) => ({ ...v, share_percent: event.target.value }))}
              />
            )}
          </Field>
        </div>
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            className="size-4"
            checked={values.is_primary}
            onChange={(event) => setValues((v) => ({ ...v, is_primary: event.target.checked }))}
          />
          {t.lease.partyPrimary}
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
 * L'invito, e il token consegnato per quello che è.
 *
 * `Notifier` è una porta senza adattatore: **nessuna email parte**. Una schermata
 * che dicesse «invito inviato» sarebbe una bugia nell'interfaccia, quindi il token
 * si mostra e si dice che va consegnato a mano. È brutto ed è la verità.
 *
 * Il token è una **credenziale**: si mostra una volta, non si conserva, e non
 * finisce in nessun log — ecco perché non passa da nessuna cache di query.
 *
 * # La revoca vive qui, e da nessun'altra parte
 *
 * `revokeInvitation` vuole l'id dell'invito, e **nessuna rotta elenca gli
 * inviti**: l'unico momento in cui il client ce l'ha è subito dopo averlo creato,
 * cioè adesso. Serve al caso vero — «ho sbagliato indirizzo, lo rifaccio» — e va
 * offerto qui dicendo che dopo resta solo lasciarlo scadere.
 *
 * Costruire un elenco per offrirla altrove vorrebbe dire aggiungere una rotta,
 * cioè backend dentro una fetta di client.
 *
 * # La conferma è **dentro** questa modale e non un'altra sopra
 *
 * `ConfirmAction` aprirebbe un secondo dialogo sopra il primo: due strati modali
 * per un gesto che si spiega in due righe, e il codice appena emesso sparirebbe
 * dietro. Qui la modale cambia corpo e torna al form, che è anche il gesto
 * successivo — rifare l'invito con l'indirizzo giusto.
 */
function InviteTenantDialog({ leaseId }: { leaseId: string }) {
  const client = useApi()
  const [open, setOpen] = useState(false)
  const [error, setError] = useState<unknown>(null)
  const [issued, setIssued] = useState<Invitation | null>(null)
  const [confirmingRevoke, setConfirmingRevoke] = useState(false)
  const [revoked, setRevoked] = useState(false)
  const [copied, setCopied] = useState(false)
  const [values, setValues] = useState({ email: '', full_name: '' })

  const mutation = useMutation({
    mutationFn: () => inviteTenant(client, leaseId, values),
    onSuccess: (invitation) => {
      setIssued(invitation)
      setRevoked(false)
      setError(null)
    },
    onError: setError,
  })

  const revoke = useMutation({
    mutationFn: (invitationId: string) => revokeInvitation(client, invitationId),
    onSuccess: () => {
      // Si torna al form **con i campi pieni**: chi revoca lo fa quasi sempre
      // per correggere l'indirizzo, e ridigitare il nome sarebbe una punizione
      // per aver notato l'errore.
      setIssued(null)
      setConfirmingRevoke(false)
      setCopied(false)
      setRevoked(true)
      setError(null)
    },
    onError: setError,
  })

  return (
    <FormDialog
      trigger={
        <Button>
          <UserPlus aria-hidden />
          {t.invitation.invite}
        </Button>
      }
      title={
        issued && confirmingRevoke
          ? t.invitation.revokeTitle
          : issued
            ? t.invitation.tokenTitle
            : t.invitation.inviteTitle
      }
      description={issued && !confirmingRevoke ? t.invitation.tokenExplain : undefined}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) {
          // Il token si dimentica alla chiusura: resta il tempo di copiarlo.
          setIssued(null)
          setError(null)
          setCopied(false)
          setConfirmingRevoke(false)
          setRevoked(false)
          setValues({ email: '', full_name: '' })
        }
      }}
    >
      {issued && confirmingRevoke ? (
        <div className="flex flex-col gap-4">
          <InlineError error={error} />
          <p className="text-sm text-ink-muted">{t.invitation.revokeExplain}</p>
          <div className="flex flex-wrap justify-end gap-2">
            <Button onClick={() => setConfirmingRevoke(false)} disabled={revoke.isPending}>
              {t.common.cancel}
            </Button>
            <Button
              variant="danger"
              disabled={revoke.isPending}
              onClick={() => revoke.mutate(issued.id)}
            >
              {revoke.isPending ? t.common.saving : t.invitation.revoke}
            </Button>
          </div>
        </div>
      ) : issued ? (
        <div className="flex flex-col gap-3">
          <div>
            <p className="text-xs font-medium tracking-wide text-ink-muted uppercase">
              {t.invitation.tokenLabel}
            </p>
            <code className="mt-1 block break-all rounded-lg bg-surface-muted p-3 font-numeric text-sm">
              {issued.accept_token}
            </code>
          </div>
          <p className="text-sm text-ink-muted">{t.invitation.tokenWhereToUse}</p>
          <div className="flex flex-wrap justify-end gap-2">
            <Button variant="dangerGhost" onClick={() => setConfirmingRevoke(true)}>
              {t.invitation.revoke}
            </Button>
            <Button
              onClick={() => {
                void navigator.clipboard.writeText(issued.accept_token).then(() => setCopied(true))
              }}
            >
              <Copy aria-hidden />
              {copied ? t.common.copied : t.common.copy}
            </Button>
            <Button variant="primary" onClick={() => setOpen(false)}>
              {t.common.close}
            </Button>
          </div>
        </div>
      ) : (
        <form
          noValidate
          className="flex flex-col gap-4"
          onSubmit={(event) => {
            event.preventDefault()
            mutation.mutate()
          }}
        >
          <InlineError error={error} />
          {revoked ? (
            <p role="status" className="rounded-lg bg-surface-muted px-3 py-2 text-sm text-ink-muted">
              {t.invitation.revokeDone}
            </p>
          ) : null}
          <Field label={t.invitation.fullName} required>
            {(props) => (
              <Input
                {...props}
                value={values.full_name}
                onChange={(event) => setValues((v) => ({ ...v, full_name: event.target.value }))}
              />
            )}
          </Field>
          <Field label={t.invitation.email} required>
            {(props) => (
              <Input
                {...props}
                type="email"
                value={values.email}
                onChange={(event) => setValues((v) => ({ ...v, email: event.target.value }))}
              />
            )}
          </Field>
          <div className="flex justify-end gap-2">
            <Button onClick={() => setOpen(false)}>{t.common.cancel}</Button>
            <Button type="submit" variant="primary" disabled={mutation.isPending}>
              {mutation.isPending ? t.common.saving : t.invitation.send}
            </Button>
          </div>
        </form>
      )}
    </FormDialog>
  )
}

function LeaseChargesCard({
  leaseId,
  currency,
  canIssue,
}: {
  leaseId: string
  currency: string
  /** Emettere un addebito è dello staff; leggerli è di chi li deve pagare. */
  canIssue: boolean
}) {
  const client = useApi()
  const charges = useQuery({
    queryKey: keys.charges({ leaseId }),
    queryFn: () => listCharges(client, { lease_id: leaseId }),
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t.lease.charges}</CardTitle>
        {canIssue ? <IssueChargeDialog leaseId={leaseId} /> : null}
      </CardHeader>
      <Async
        query={charges}
        isEmpty={(page) => page.items.length === 0}
        empty={<Empty title={t.charge.empty} />}
      >
        {(page) => (
          <TableFrame>
            <TableCaption>{t.lease.charges}</TableCaption>
            <thead>
              <tr>
                <Th>{t.charge.type}</Th>
                <Th>{t.charge.status}</Th>
                <Th>{t.charge.dueDate}</Th>
                <ThNumeric>{t.charge.amount}</ThNumeric>
                <ThNumeric>{t.charge.outstandingAmount}</ThNumeric>
              </tr>
            </thead>
            <tbody>
              {page.items.map((charge) => (
                <Tr key={charge.id}>
                  <Td>{t.charge.types[charge.charge_type]}</Td>
                  <Td>
                    <ChargeStatusBadge
                      status={charge.status}
                      effectiveStatus={charge.effective_status}
                    />
                  </Td>
                  <Td>{formatDate(charge.due_date)}</Td>
                  <TdNumeric>{formatAmount(charge.amount, { currency })}</TdNumeric>
                  <TdNumeric>{formatAmount(charge.outstanding_amount, { currency })}</TdNumeric>
                </Tr>
              ))}
            </tbody>
          </TableFrame>
        )}
      </Async>
    </Card>
  )
}

function IssueChargeDialog({ leaseId }: { leaseId: string }) {
  const client = useApi()
  const queryClient = useQueryClient()
  const [open, setOpen] = useState(false)
  const [error, setError] = useState<unknown>(null)
  const [values, setValues] = useState({
    charge_type: 'rent' as const,
    amount: '',
    due_date: todayIso(),
    description: '',
  })

  const mutation = useMutation({
    mutationFn: () =>
      issueCharge(client, leaseId, {
        charge_type: values.charge_type,
        amount: values.amount,
        due_date: values.due_date,
        ...(values.description ? { description: values.description } : {}),
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['charges'] })
      await queryClient.invalidateQueries({ queryKey: keys.dashboard })
      setValues({ charge_type: 'rent', amount: '', due_date: todayIso(), description: '' })
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
          {t.lease.issueCharge}
        </Button>
      }
      title={t.charge.newTitle}
      description={t.charge.noPatch}
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
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <Field label={t.charge.amount} required>
            {(props) => (
              <Input
                {...props}
                inputMode="decimal"
                placeholder="800.00"
                value={values.amount}
                onChange={(event) => setValues((v) => ({ ...v, amount: event.target.value }))}
              />
            )}
          </Field>
          <Field label={t.charge.dueDate} required>
            {(props) => (
              <Input
                {...props}
                type="date"
                value={values.due_date}
                onChange={(event) => setValues((v) => ({ ...v, due_date: event.target.value }))}
              />
            )}
          </Field>
        </div>
        <Field label={t.charge.description}>
          {(props) => (
            <Input
              {...props}
              value={values.description}
              onChange={(event) => setValues((v) => ({ ...v, description: event.target.value }))}
            />
          )}
        </Field>
        <div className="flex justify-end gap-2">
          <Button onClick={() => setOpen(false)}>{t.common.cancel}</Button>
          <Button type="submit" variant="primary" disabled={mutation.isPending}>
            {mutation.isPending ? t.common.saving : t.charge.create}
          </Button>
        </div>
      </form>
    </FormDialog>
  )
}
