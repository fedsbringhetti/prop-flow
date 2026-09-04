import type { ComponentProps } from 'react'

import { cn } from '@/lib/utils'

/**
 * Una tabella che regge mille righe **e** uno schermo da 375 px.
 *
 * Lo scorrimento orizzontale sta sul contenitore e non sulla pagina: una pagina
 * che scorre in orizzontale su un telefono è il modo più rapido di rendere
 * inutilizzabile un elenco. `<caption>` è visivamente nascosta ma letta: senza,
 * un lettore di schermo annuncia «tabella» e basta.
 */
export function TableFrame({ className, ...props }: ComponentProps<'table'>) {
  return (
    <div className={cn('w-full overflow-x-auto', className)}>
      <table className="w-full min-w-[36rem] border-collapse text-sm" {...props} />
    </div>
  )
}

export function TableCaption({ className, ...props }: ComponentProps<'caption'>) {
  return <caption className={cn('sr-only', className)} {...props} />
}

export function Th({ className, ...props }: ComponentProps<'th'>) {
  return (
    <th
      scope="col"
      className={cn(
        'border-b border-border-subtle px-3 py-2 text-left text-xs font-semibold tracking-wide text-ink-muted uppercase',
        className,
      )}
      {...props}
    />
  )
}

export function Td({ className, ...props }: ComponentProps<'td'>) {
  return (
    <td
      className={cn('border-b border-border-subtle px-3 py-2 align-middle text-ink', className)}
      {...props}
    />
  )
}

/** Una cella numerica: allineata a destra e con le cifre incolonnate. */
export function TdNumeric({ className, ...props }: ComponentProps<'td'>) {
  return <Td className={cn('tabular text-right', className)} {...props} />
}

export function ThNumeric({ className, ...props }: ComponentProps<'th'>) {
  return <Th className={cn('text-right', className)} {...props} />
}

export function Tr({ className, ...props }: ComponentProps<'tr'>) {
  return <tr className={cn('hover:bg-surface-muted/60', className)} {...props} />
}
