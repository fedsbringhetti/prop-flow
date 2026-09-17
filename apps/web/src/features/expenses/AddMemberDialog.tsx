import { useMutation, useQueryClient } from '@tanstack/react-query'
import { UserPlus } from 'lucide-react'
import { useState } from 'react'

import { addGroupMember } from '@/api/endpoints'
import type { ExpenseGroup, PropertyResident } from '@/api/types'
import { Empty, InlineError, Loading } from '@/components/state'
import { Button } from '@/components/ui/button'
import { FormDialog } from '@/components/ui/confirm'
import { Field, Input, Select } from '@/components/ui/field'
import { keys } from '@/app/query'
import { useApi } from '@/app/session'
import { t } from '@/i18n'
import { useResidents } from './names'

interface Candidate {
  resident: PropertyResident
  /** Perché non si può scegliere, quando non si può. */
  blocked: string | undefined
  /** Era dentro ed è uscito: aggiungerlo lo fa **rientrare**. */
  rejoins: boolean
}

/**
 * Aggiungere qualcuno al gruppo, **con il suo nome**.
 *
 * # Il nome è la ragione per cui questo form esiste così
 *
 * `display_name` è facoltativo per l'API e il client lo manda **sempre**, preso
 * dal `full_name` del residente scelto. Non è una comodità: è l'unico posto in cui
 * quel nome vivrà — saldi, quote, spese e rimborsi identificano le persone con il
 * solo `user_id` — e per chi lascerà la casa non ci sarà più modo di recuperarlo,
 * perché l'elenco dei residenti copre chi abita **ora**.
 *
 * # Chi non ha accettato l'invito si mostra, e non si può scegliere
 *
 * Un residente senza `user_id` non ha un identificatore da confrontare con la
 * policy. **Va mostrato non selezionabile con la ragione, non nascosto**:
 * nasconderlo farebbe cercare una persona che sullo schermo c'è. È la terza volta
 * che questa forma si presenta, dopo `payer_user_id` e `lease_id`.
 *
 * # Chi è già dentro e chi rientra
 *
 * `POST .../members` iscrive **o fa rientrare**: l'indice unico su
 * `(group_id, user_id)` non è parziale, quindi il rientro è un aggiornamento. Su
 * un membro già attivo risponde `409 expense_group.membership_exists`, e quel caso
 * si mostra bloccato invece di offrirlo; chi era uscito si può scegliere, e il
 * form dice che sta rientrando. **Chi rientra ridichiara il peso**: la stanza può
 * essere cambiata, e conservare quello vecchio farebbe pagare la doppia a chi è
 * tornato nella singola.
 *
 * # Chi non abita più qui non è un errore
 *
 * L'API non verifica la residenza, ed è voluto — `property_id` è facoltativo, e
 * una regola che si aggira omettendo un campo facoltativo si legge come una
 * garanzia senza esserlo. Il client non la reintroduce: qui si offrono i residenti
 * perché è la via comoda, non perché sia l'unica lecita.
 */
export function AddMemberDialog({ group }: { group: ExpenseGroup }) {
  const client = useApi()
  const queryClient = useQueryClient()

  const [open, setOpen] = useState(false)
  const [submitted, setSubmitted] = useState(false)
  const [userId, setUserId] = useState('')
  const [weight, setWeight] = useState('')

  const residents = useResidents(group.property_id)

  const members = group.members ?? []
  const candidates: Candidate[] = (residents.data?.items ?? []).map((resident) => {
    const member = resident.user_id
      ? members.find((entry) => entry.user_id === resident.user_id)
      : undefined
    return {
      resident,
      blocked: !resident.user_id
        ? t.expense.residentNoAccount
        : member && !member.left_at
          ? t.expense.residentAlreadyIn
          : undefined,
      rejoins: Boolean(member?.left_at),
    }
  })
  const selectable = candidates.filter((candidate) => !candidate.blocked)

  const mutation = useMutation({
    mutationFn: () => {
      const chosen = candidates.find((candidate) => candidate.resident.user_id === userId)
      return addGroupMember(client, group.id, {
        user_id: userId,
        // Sempre, e preso dai residenti: è l'unica occasione di dargli un nome.
        ...(chosen ? { display_name: chosen.resident.full_name } : {}),
        ...(weight.trim() ? { weight: weight.trim().replace(',', '.') } : {}),
      })
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: keys.expenseGroup(group.id) })
      setOpen(false)
      setUserId('')
      setWeight('')
      setSubmitted(false)
    },
  })

  const chosen = candidates.find((candidate) => candidate.resident.user_id === userId)
  const whoError = submitted && !userId ? t.validation.required : undefined

  return (
    <FormDialog
      trigger={
        <Button size="sm">
          <UserPlus aria-hidden />
          {t.expense.addMember}
        </Button>
      }
      title={t.expense.addMemberTitle}
      description={t.expense.addMemberExplain}
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) {
          setUserId('')
          setWeight('')
          setSubmitted(false)
          mutation.reset()
        }
      }}
    >
      {!group.property_id ? (
        <p className="rounded-lg bg-surface-muted px-3 py-2 text-sm text-ink-muted">
          {t.expense.propertyMissing}
        </p>
      ) : residents.isPending ? (
        <Loading />
      ) : residents.error ? (
        <InlineError error={residents.error} />
      ) : selectable.length === 0 ? (
        <Empty title={t.expense.addMemberNoCandidates} hint={t.expense.addMemberNoCandidatesHint} />
      ) : (
        <form
          noValidate
          className="flex flex-col gap-4"
          onSubmit={(event) => {
            event.preventDefault()
            setSubmitted(true)
            if (!userId) return
            mutation.mutate()
          }}
        >
          <InlineError error={mutation.error} />

          <Field label={t.expense.addMemberWho} error={whoError} required>
            {(props) => (
              <Select
                {...props}
                autoFocus
                value={userId}
                onChange={(event) => setUserId(event.target.value)}
              >
                <option value="">{t.common.none}</option>
                {candidates.map((candidate, index) => (
                  <option
                    // Chi non ha accettato l'invito non ha un identificatore:
                    // l'indice completa la chiave, come nell'elenco dei residenti.
                    key={`${candidate.resident.user_id ?? 'invitato'}-${index}`}
                    value={candidate.resident.user_id ?? ''}
                    disabled={Boolean(candidate.blocked)}
                  >
                    {candidate.resident.full_name}
                    {candidate.blocked ? ` — ${candidate.blocked}` : ''}
                    {candidate.rejoins ? ` — ${t.expense.residentRejoins}` : ''}
                  </option>
                ))}
              </Select>
            )}
          </Field>

          <Field label={t.expense.memberWeight} hint={t.expense.weightHint}>
            {(props) => (
              <Input
                {...props}
                inputMode="decimal"
                placeholder="1"
                value={weight}
                onChange={(event) => setWeight(event.target.value)}
              />
            )}
          </Field>

          {chosen?.rejoins ? (
            <p className="rounded-lg border border-warning/30 bg-warning-soft px-3 py-2 text-xs text-warning">
              {t.expense.residentRejoins}
            </p>
          ) : null}

          <p className="text-xs text-ink-muted">{t.expense.addMemberOutsider}</p>

          <div className="flex justify-end gap-2">
            <Button type="button" onClick={() => setOpen(false)} disabled={mutation.isPending}>
              {t.common.cancel}
            </Button>
            <Button type="submit" variant="primary" disabled={mutation.isPending}>
              {mutation.isPending ? t.common.saving : t.expense.addMember}
            </Button>
          </div>
        </form>
      )}
    </FormDialog>
  )
}
