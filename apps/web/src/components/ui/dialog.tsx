import * as DialogPrimitive from '@radix-ui/react-dialog'
import { X } from 'lucide-react'
import type { ReactNode } from 'react'

import { t } from '@/i18n'
import { cn } from '@/lib/utils'

/**
 * La modale viene da Radix e non è scritta a mano.
 *
 * Il focus va spostato dentro all'apertura, trattenuto finché è aperta, e
 * restituito all'elemento che l'ha aperta alla chiusura; `Escape` deve chiudere;
 * il resto della pagina deve sparire dall'albero di accessibilità. Sono cinque
 * comportamenti che nessuno riscrive bene a mano, e senza i quali una modale è
 * una trappola per chi naviga da tastiera.
 */
export const Dialog = DialogPrimitive.Root
export const DialogTrigger = DialogPrimitive.Trigger
export const DialogClose = DialogPrimitive.Close

export function DialogContent({
  title,
  description,
  children,
  className,
}: {
  title: string
  /** Sempre presente: è ciò che un lettore di schermo annuncia dopo il titolo. */
  description?: string
  children: ReactNode
  className?: string
}) {
  return (
    <DialogPrimitive.Portal>
      <DialogPrimitive.Overlay className="fixed inset-0 z-40 bg-ink/30 backdrop-blur-[1px]" />
      <DialogPrimitive.Content
        className={cn(
          'fixed top-1/2 left-1/2 z-50 w-[calc(100vw-2rem)] max-w-lg -translate-x-1/2 -translate-y-1/2',
          'max-h-[calc(100vh-2rem)] overflow-y-auto rounded-card border border-border-subtle bg-surface shadow-xl',
          className,
        )}
      >
        <div className="flex items-start justify-between gap-4 border-b border-border-subtle px-4 py-3">
          <div>
            <DialogPrimitive.Title className="text-sm font-semibold text-ink">
              {title}
            </DialogPrimitive.Title>
            {description ? (
              <DialogPrimitive.Description className="mt-1 text-xs text-ink-muted">
                {description}
              </DialogPrimitive.Description>
            ) : null}
          </div>
          <DialogPrimitive.Close
            aria-label={t.common.close}
            className="rounded-md p-1 text-ink-muted hover:bg-surface-muted hover:text-ink"
          >
            <X className="size-4" />
          </DialogPrimitive.Close>
        </div>
        <div className="px-4 py-4">{children}</div>
      </DialogPrimitive.Content>
    </DialogPrimitive.Portal>
  )
}
