import type { ReactNode } from 'react'

import { cn } from '@/lib/utils'

/**
 * L'intestazione di una schermata.
 *
 * Un solo `<h1>` per pagina, e l'azione principale accanto: è ciò che permette a
 * chi naviga da tastiera di capire dove si trova al primo `Tab`, e a chi guarda di
 * trovare l'azione sempre nello stesso posto.
 */
export function PageHeader({
  title,
  description,
  actions,
  breadcrumb,
}: {
  title: string
  description?: string | undefined
  actions?: ReactNode
  breadcrumb?: ReactNode
}) {
  return (
    <header className="mb-5 flex flex-col gap-3">
      {breadcrumb ? <div className="text-xs text-ink-muted">{breadcrumb}</div> : null}
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <h1 className="text-xl font-semibold tracking-tight text-ink">{title}</h1>
          {description ? <p className="mt-1 text-sm text-ink-muted">{description}</p> : null}
        </div>
        {/* Le azioni **avvolgono**, e non è cosmesi: senza, una schermata con
            cinque azioni — le segnalazioni ne hanno cinque nello stato `open` —
            fa scorrere in orizzontale la **pagina** a 375 px, che è il modo più
            rapido di rendere inutilizzabile una schermata su un telefono. Sono i
            bottoni a dover andare a capo, non il corpo a doversi allargare.

            `shrink-0` diceva il contrario: il riquadro prendeva la propria
            larghezza a contenuto pieno e usciva dallo schermo, e il suo
            `flex-wrap` non poteva fare niente perché non aveva un limite entro
            cui avvolgere. */}
        {actions ? <div className="flex min-w-0 flex-wrap gap-2">{actions}</div> : null}
      </div>
    </header>
  )
}

export function PageSection({
  className,
  children,
}: {
  className?: string
  children: ReactNode
}) {
  return <section className={cn('mb-6', className)}>{children}</section>
}

/**
 * Coppia etichetta/valore, per i dettagli.
 *
 * `<dl>` e non due `<div>`: la relazione fra l'etichetta e il valore è nel markup,
 * quindi un lettore di schermo la annuncia invece di leggere due testi accostati.
 */
export function DescriptionList({ children }: { children: ReactNode }) {
  return <dl className="grid grid-cols-1 gap-x-6 gap-y-3 sm:grid-cols-2">{children}</dl>
}

export function DescriptionItem({
  label,
  children,
  numeric,
}: {
  label: string
  children: ReactNode
  numeric?: boolean
}) {
  return (
    <div className="min-w-0">
      <dt className="text-xs font-medium tracking-wide text-ink-muted uppercase">{label}</dt>
      <dd className={cn('mt-0.5 text-sm break-words text-ink', numeric && 'tabular')}>
        {children}
      </dd>
    </div>
  )
}
