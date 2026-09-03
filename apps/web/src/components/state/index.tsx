import { AlertTriangle, Inbox, Loader2 } from 'lucide-react'
import type { ReactNode } from 'react'

import { Button } from '@/components/ui/button'
import { t } from '@/i18n'
import { ApiError, messageFor } from '@/api/problem'
import { cn } from '@/lib/utils'

/**
 * I tre stati di ogni schermata, come componenti di prima classe.
 *
 * Non sono `if` sparsi, e la differenza non è estetica: un `if` scritto dove
 * serve viene scritto **solo** dove qualcuno se ne ricorda, e la schermata che
 * l'ha dimenticato mostra il vuoto per un caricamento o una pagina bianca per un
 * errore. Averli come componenti rende il caso mancante visibile mentre si
 * scrive la schermata, non mentre la si usa.
 */

/** Caricamento: annunciato, non solo disegnato. */
export function Loading({ label, className }: { label?: string; className?: string }) {
  return (
    <div
      role="status"
      aria-live="polite"
      className={cn('flex items-center gap-2 px-4 py-10 text-sm text-ink-muted', className)}
    >
      <Loader2 className="size-4 animate-spin" aria-hidden />
      <span>{label ?? t.state.loadingTitle}</span>
    </div>
  )
}

/**
 * Uno scheletro per un elenco che sta arrivando.
 *
 * Serve dove il salto di layout darebbe più fastidio dell'attesa: una tabella che
 * appare tutta insieme sposta ciò che c'è sotto e fa perdere il segno.
 */
export function SkeletonRows({ rows = 4 }: { rows?: number }) {
  return (
    <div role="status" aria-live="polite" aria-label={t.state.loadingTitle} className="space-y-2 p-4">
      {Array.from({ length: rows }, (_, index) => (
        <div key={index} className="h-9 animate-pulse rounded-lg bg-surface-muted" />
      ))}
    </div>
  )
}

/**
 * Il vuoto **spiegato**.
 *
 * «Nessun risultato» non è uno stato progettato: dice che non c'è niente e non
 * dice che cosa farne. Qui il vuoto porta con sé la ragione e, quando esiste, il
 * gesto che lo riempie.
 */
export function Empty({
  title,
  hint,
  action,
  className,
}: {
  title?: string
  hint?: string
  action?: ReactNode
  className?: string
}) {
  return (
    <div className={cn('flex flex-col items-center gap-2 px-4 py-12 text-center', className)}>
      <Inbox className="size-6 text-ink-muted/60" aria-hidden />
      <p className="text-sm font-medium text-ink">{title ?? t.state.emptyTitle}</p>
      {hint ? <p className="max-w-sm text-sm text-ink-muted">{hint}</p> : null}
      {action ? <div className="mt-2">{action}</div> : null}
    </div>
  )
}

/**
 * L'errore, con il riferimento della richiesta.
 *
 * `request_id` compare nel corpo di ogni errore dell'API apposta: chi segnala un
 * problema porta con sé la chiave con cui si ritrova la richiesta nei log.
 * Nasconderlo perché è brutto significa buttare via l'unica cosa che rende una
 * segnalazione utile.
 */
export function ErrorState({
  error,
  onRetry,
  className,
}: {
  error: unknown
  onRetry?: () => void
  className?: string
}) {
  const requestId = error instanceof ApiError ? error.requestId : undefined
  return (
    <div
      role="alert"
      className={cn('flex flex-col items-center gap-2 px-4 py-10 text-center', className)}
    >
      <AlertTriangle className="size-6 text-danger" aria-hidden />
      <p className="text-sm font-medium text-ink">{t.state.errorTitle}</p>
      <p className="max-w-md text-sm text-ink-muted">{messageFor(error)}</p>
      {requestId ? (
        <p className="text-xs text-ink-muted">
          {t.state.errorRequestId}: <code className="font-numeric">{requestId}</code>
        </p>
      ) : null}
      {onRetry ? (
        <Button className="mt-2" onClick={onRetry}>
          {t.common.retry}
        </Button>
      ) : null}
    </div>
  )
}

/**
 * Un errore in linea, per un form o un riquadro: stesso messaggio, meno spazio.
 */
export function InlineError({ error, className }: { error: unknown; className?: string }) {
  if (!error) return null
  const requestId = error instanceof ApiError ? error.requestId : undefined
  return (
    <div
      role="alert"
      className={cn(
        'rounded-lg border border-danger/30 bg-danger-soft px-3 py-2 text-sm text-danger',
        className,
      )}
    >
      {messageFor(error)}
      {requestId ? (
        <span className="ml-1 text-xs opacity-80">({requestId})</span>
      ) : null}
    </div>
  )
}

/**
 * Sceglie fra i tre stati una volta sola.
 *
 * Chi la usa non può dimenticarne uno: la firma li chiede tutti, e il caso felice
 * riceve dati già garantiti presenti.
 */
export function Async<T>({
  query,
  children,
  empty,
  isEmpty,
  loading,
}: {
  query: { data: T | undefined; isPending: boolean; error: unknown; refetch: () => void }
  children: (data: T) => ReactNode
  empty?: ReactNode
  isEmpty?: (data: T) => boolean
  loading?: ReactNode
}) {
  if (query.isPending) return <>{loading ?? <SkeletonRows />}</>
  if (query.error) return <ErrorState error={query.error} onRetry={query.refetch} />
  if (query.data === undefined) return <ErrorState error={query.error} onRetry={query.refetch} />
  if (empty && isEmpty?.(query.data)) return <>{empty}</>
  return <>{children(query.data)}</>
}
