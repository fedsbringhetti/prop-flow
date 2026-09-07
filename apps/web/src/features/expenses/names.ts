import { useQuery } from '@tanstack/react-query'

import { listResidents } from '@/api/endpoints'
import type { ExpenseGroupMember, PropertyResident, Uuid } from '@/api/types'
import { keys } from '@/app/query'
import { useApi } from '@/app/session'
import { t } from '@/i18n'

/**
 * I nomi delle persone di un gruppo, che **nessuna risposta di questo modulo
 * porta**.
 *
 * `GroupBalanceResponse`, `ExpenseShareResponse`, `SettlementResponse` e
 * `ExpenseResponse.paid_by_user_id` identificano chiunque con il solo `user_id`.
 * L'unico nome che esista è `display_name` sulla membership, ed è **facoltativo**:
 * lo scrive chi iscrive la persona, o non esiste più. Ne segue la regola che il
 * client applica senza eccezioni — vedi `AddMemberDialog` e `NewGroupDialog`: ogni
 * ingresso porta un nome.
 *
 * # Perché i residenti non bastano, e sono comunque il ripiego
 *
 * `GET /properties/{id}/residents` dà `user_id` e `full_name`, ma copre **chi
 * abita ora**. La persona uscita di casa che sta ancora saldando il suo conto —
 * cioè precisamente il caso per cui la migration `0027` tiene il suo debito nei
 * saldi — non compare più fra i residenti. Risolvere i nomi di lì funzionerebbe
 * in ogni caso tranne quello che conta, e si romperebbe **solo sui gruppi vecchi
 * con qualcuno uscito**: il difetto più difficile da notare fra quelli possibili
 * qui.
 *
 * Restano il secondo posto dove guardare, per i gruppi creati prima di questa
 * regola o da `curl`. E `property_id` sul gruppo è facoltativo, quindi a volte non
 * c'è nemmeno un elenco da interrogare.
 *
 * # E se non c'è nessuna delle due cose
 *
 * Un'etichetta neutra dal catalogo, **mai l'UUID**: un identificatore in faccia a
 * chi legge non è informazione, è una resa. È la condizione da cui il selettore
 * delle organizzazioni è stato liberato una fetta fa.
 */
export type NameOf = (userId: Uuid) => string

export function resolveName(
  userId: Uuid,
  members: readonly ExpenseGroupMember[] | undefined,
  residents: readonly PropertyResident[] | undefined,
): string {
  const member = members?.find((entry) => entry.user_id === userId)
  const declared = member?.display_name?.trim()
  if (declared) return declared

  const resident = residents?.find((entry) => entry.user_id === userId)
  const known = resident?.full_name.trim()
  if (known) return known

  return t.expense.memberUnknown
}

/** Il nome di chi guarda, per il campo che lo scrive nella propria membership. */
export function ownName(
  userId: Uuid | null | undefined,
  residents: readonly PropertyResident[] | undefined,
): string {
  if (!userId) return ''
  return residents?.find((entry) => entry.user_id === userId)?.full_name ?? ''
}

/**
 * I residenti dell'immobile del gruppo, quando il gruppo ne ha uno.
 *
 * Serve due volte: a dare un nome a chi non lo ha dichiarato, e a scegliere chi
 * aggiungere. La query è condivisa con la scheda dell'immobile — stessa chiave,
 * stessa risposta.
 */
export function useResidents(propertyId: string | undefined) {
  const client = useApi()
  return useQuery({
    queryKey: keys.residents(propertyId ?? ''),
    queryFn: () => listResidents(client, propertyId ?? ''),
    enabled: Boolean(propertyId),
  })
}

/**
 * La funzione da passare a una tabella di saldi, quote o rimborsi.
 *
 * Non solleva e non attende: finché i residenti non sono arrivati risolve con ciò
 * che ha, che nel caso normale — il nome scritto all'ingresso — è già tutto.
 */
export function useMemberNames(
  members: readonly ExpenseGroupMember[] | undefined,
  propertyId: string | undefined,
): NameOf {
  const residents = useResidents(propertyId)
  const items = residents.data?.items
  return (userId: Uuid) => resolveName(userId, members, items)
}
