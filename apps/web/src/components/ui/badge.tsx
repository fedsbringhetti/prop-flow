import { cva, type VariantProps } from 'class-variance-authority'
import type { ComponentProps } from 'react'

import { cn } from '@/lib/utils'

/**
 * Il colore di uno stato dice **che cosa richiede attenzione**, non che cosa è
 * bello. `warning` è ciò che aspetta una persona, `danger` ciò che è in ritardo o
 * caduto, `muted` ciò che è concluso e non chiede nulla.
 */
const badgeVariants = cva(
  'inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-xs font-medium whitespace-nowrap',
  {
    variants: {
      tone: {
        neutral: 'bg-surface-muted text-ink-muted',
        info: 'bg-accent-soft text-accent',
        success: 'bg-success-soft text-success',
        warning: 'bg-warning-soft text-warning',
        danger: 'bg-danger-soft text-danger',
        muted: 'bg-surface-muted text-ink-muted line-through decoration-ink-muted/40',
      },
    },
    defaultVariants: { tone: 'neutral' },
  },
)

export type BadgeTone = NonNullable<VariantProps<typeof badgeVariants>['tone']>

export function Badge({
  className,
  tone,
  ...props
}: ComponentProps<'span'> & VariantProps<typeof badgeVariants>) {
  return <span className={cn(badgeVariants({ tone }), className)} {...props} />
}
