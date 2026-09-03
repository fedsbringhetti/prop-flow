import * as LabelPrimitive from '@radix-ui/react-label'
import type { ComponentProps, ReactNode } from 'react'
import { useId } from 'react'

import { cn } from '@/lib/utils'

/**
 * Un campo di form: etichetta, controllo, spiegazione, errore.
 *
 * Esiste come componente e non come tre elementi ripetuti perché è dove
 * l'accessibilità si perde. Qui l'etichetta è legata al controllo, la spiegazione
 * e l'errore sono in `aria-describedby`, e `aria-invalid` segue lo stato: sono
 * quattro attributi che nessuno ricorda di scrivere dodici volte di seguito, e
 * senza i quali un lettore di schermo annuncia «campo di testo» e nient'altro.
 */
export interface FieldProps {
  label: string
  /** Spiegazione sotto il controllo: il perché, non la ripetizione del nome. */
  hint?: string | undefined
  error?: string | undefined
  required?: boolean
  className?: string
  children: (props: {
    id: string
    'aria-describedby': string | undefined
    'aria-invalid': boolean
  }) => ReactNode
}

export function Field({ label, hint, error, required, className, children }: FieldProps) {
  const id = useId()
  const hintId = hint ? `${id}-hint` : undefined
  const errorId = error ? `${id}-error` : undefined
  const describedBy = [hintId, errorId].filter(Boolean).join(' ') || undefined

  return (
    <div className={cn('flex flex-col gap-1.5', className)}>
      <LabelPrimitive.Root htmlFor={id} className="text-sm font-medium text-ink">
        {label}
        {required ? (
          <span aria-hidden className="ml-0.5 text-danger">
            *
          </span>
        ) : null}
      </LabelPrimitive.Root>
      {children({ id, 'aria-describedby': describedBy, 'aria-invalid': Boolean(error) })}
      {hint ? (
        <p id={hintId} className="text-xs text-ink-muted">
          {hint}
        </p>
      ) : null}
      {error ? (
        // `role="alert"` e non un semplice paragrafo: un errore che compare dopo
        // l'invio deve essere annunciato, non solo disegnato.
        <p id={errorId} role="alert" className="text-xs font-medium text-danger">
          {error}
        </p>
      ) : null}
    </div>
  )
}

const controlClasses =
  'w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm text-ink transition-colors placeholder:text-ink-muted/60 disabled:cursor-not-allowed disabled:bg-surface-muted disabled:opacity-70 aria-[invalid=true]:border-danger'

export function Input({ className, ...props }: ComponentProps<'input'>) {
  return <input className={cn(controlClasses, 'h-9', className)} {...props} />
}

export function Textarea({ className, ...props }: ComponentProps<'textarea'>) {
  return <textarea className={cn(controlClasses, 'min-h-20 resize-y', className)} {...props} />
}

/**
 * Una `<select>` nativa, e non un menù ricostruito.
 *
 * Radix serve dove il browser non offre niente — modali, menù, combobox — non
 * dove offre un controllo che funziona già da tastiera, che su un telefono apre
 * il selettore del sistema e che nessun lettore di schermo può fraintendere.
 * Riscriverlo sarebbe il modo più rapido di costruire qualcosa che *sembra* più
 * curato ed è meno usabile.
 */
export function Select({ className, ...props }: ComponentProps<'select'>) {
  return <select className={cn(controlClasses, 'h-9 pr-8', className)} {...props} />
}
