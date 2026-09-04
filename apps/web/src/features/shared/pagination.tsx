import { useInfiniteQuery, type QueryKey } from '@tanstack/react-query'

import type { Page } from '@/api/types'
import { Button } from '@/components/ui/button'
import { t } from '@/i18n'

/**
 * Paginazione **a cursore**, come la fa l'API.
 *
 * Keyset e non offset: con l'offset il database scarta N righe a ogni pagina, e
 * le righe inserite nel frattempo fanno saltare o ripetere elementi. Il client
 * segue lo stesso schema — passa `after` e accumula — invece di reinventare
 * numeri di pagina che non esistono dall'altra parte.
 *
 * `next_cursor` **assente** significa fine dei risultati: è l'unico segnale, e non
 * c'è un conteggio totale da mostrare. Un elenco senza totale è meno rassicurante
 * e più onesto di un totale che costerebbe una seconda query a ogni pagina.
 */
export function usePagedQuery<T>({
  queryKey,
  fetchPage,
  enabled = true,
}: {
  queryKey: QueryKey
  fetchPage: (after: string | undefined) => Promise<Page<T>>
  enabled?: boolean
}) {
  const query = useInfiniteQuery({
    queryKey,
    enabled,
    initialPageParam: undefined as string | undefined,
    queryFn: ({ pageParam }) => fetchPage(pageParam),
    getNextPageParam: (lastPage) => lastPage.next_cursor,
  })

  const items = query.data?.pages.flatMap((page) => page.items)

  return {
    ...query,
    // `data` è l'elenco accumulato: chi usa `Async` riceve le righe, non le
    // pagine, e non deve appiattirle di nuovo in ogni schermata.
    data: items,
  }
}

export function LoadMore({
  hasNextPage,
  isFetchingNextPage,
  fetchNextPage,
}: {
  hasNextPage: boolean
  isFetchingNextPage: boolean
  fetchNextPage: () => void
}) {
  if (!hasNextPage) return null
  return (
    <div className="flex justify-center border-t border-border-subtle p-3">
      <Button onClick={fetchNextPage} disabled={isFetchingNextPage}>
        {isFetchingNextPage ? t.state.loadingTitle : t.common.loadMore}
      </Button>
    </div>
  )
}
