import { useEffect, useMemo, useRef, useState, type ComponentProps, type ReactNode } from 'react'

import { Button } from '@/components/ui/button'
import { t } from '@/i18n'
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

// ---------------------------------------------------------------------------
// La selezione, nella primitiva e una volta sola
// ---------------------------------------------------------------------------

/**
 * Le righe selezionate di una tabella, come **insieme di identificatori**.
 *
 * Di identificatori e non di righe, ed è la regola che rende il conteggio vero:
 * dopo una rilettura le righe fatte spariscono dalla pagina, e la selezione si
 * **pota** su ciò che la pagina contiene adesso. Senza, «3 selezionati» resta
 * scritto sopra una tabella in cui ne restano 2, e il bottone agisce su una riga
 * che non c'è.
 *
 * La potatura è una **derivazione** e non un effetto: l'insieme esposto è
 * l'intersezione fra ciò che è stato spuntato e `ids`, calcolata a ogni render.
 * Un `useEffect` che ripulisse lo stato arriverebbe un render dopo, e per quel
 * render il conteggio direbbe il numero sbagliato — che è precisamente il
 * difetto da evitare.
 *
 * `ids` sono le righe **su cui si può agire**, non tutte quelle della tabella: una
 * riga che risponderebbe sempre `409` non si offre alla selezione, per la stessa
 * ragione per cui un bottone che produce sempre un errore è peggio di un bottone
 * assente.
 *
 * «Seleziona tutto» è **la pagina** — le righe caricate — e mai l'elenco intero:
 * gli elenchi sono a cursore, quindi «tutte le 340 righe» non è un'operazione che
 * il client possa fare senza scaricarle tutte, e scaricarle tutte per agirci sopra
 * è il caso in cui una persona dovrebbe fermarsi a pensare.
 */
export interface Selection {
  /** Gli identificatori selezionati **e ancora in pagina**. */
  readonly selected: ReadonlySet<string>
  /** Quanti sono selezionati, già potati. */
  readonly count: number
  /** Quante righe la pagina offre alla selezione. */
  readonly total: number
  readonly allSelected: boolean
  readonly someSelected: boolean
  isSelected: (id: string) => boolean
  toggle: (id: string) => void
  /** Tutta la pagina, o niente se è già tutta selezionata. */
  toggleAll: () => void
  clear: () => void
  /** Toglie dalla selezione gli identificatori indicati: le righe fatte. */
  remove: (ids: Iterable<string>) => void
}

export function useSelection(ids: readonly string[]): Selection {
  const [chosen, setChosen] = useState<ReadonlySet<string>>(() => new Set())

  const selected = useMemo(
    () => new Set(ids.filter((id) => chosen.has(id))),
    [ids, chosen],
  )

  const total = ids.length
  const count = selected.size
  const allSelected = total > 0 && count === total
  const someSelected = count > 0 && !allSelected

  return {
    selected,
    count,
    total,
    allSelected,
    someSelected,
    isSelected: (id) => selected.has(id),
    toggle: (id) =>
      setChosen((previous) => {
        const next = new Set(previous)
        if (next.has(id)) next.delete(id)
        else next.add(id)
        return next
      }),
    toggleAll: () => setChosen(allSelected ? new Set() : new Set(ids)),
    clear: () => setChosen(new Set()),
    remove: (gone) =>
      setChosen((previous) => {
        const next = new Set(previous)
        for (const id of gone) next.delete(id)
        return next
      }),
  }
}

/**
 * La casella di intestazione: **la pagina**, mai «tutti».
 *
 * `indeterminate` non è un attributo HTML ma una proprietà del nodo, quindi si
 * scrive con un `ref`. Senza, una selezione parziale mostrerebbe una casella vuota
 * sopra tre righe spuntate.
 */
export function SelectAllCheckbox({ selection }: { selection: Selection }) {
  const ref = useRef<HTMLInputElement>(null)
  useEffect(() => {
    if (ref.current) ref.current.indeterminate = selection.someSelected
  }, [selection.someSelected])

  return (
    <input
      ref={ref}
      type="checkbox"
      className="size-4 align-middle accent-accent"
      aria-label={t.selection.selectPage}
      checked={selection.allSelected}
      disabled={selection.total === 0}
      onChange={selection.toggleAll}
    />
  )
}

export function ThSelect({ selection, className }: { selection: Selection; className?: string }) {
  return (
    <Th className={cn('w-8 px-2', className)}>
      <SelectAllCheckbox selection={selection} />
    </Th>
  )
}

/**
 * La casella di una riga, con il motivo accanto quando l'ultima azione non è
 * riuscita **su quella riga**.
 *
 * Il motivo sta qui e non in un avviso in cima, perché con tre motivi diversi su
 * tre righe un avviso cumulativo non dice quale. E la casella **ferma la
 * propagazione del clic**: le righe degli elenchi portano un collegamento alla
 * scheda, e chi vuole aprirla deve poterlo fare ancora — la riga intera non
 * diventa «seleziona».
 *
 * `label` è il nome della riga nell'etichetta accessibile: in una tabella di
 * venti immobili «Seleziona» ripetuto venti volte non dice quale.
 */
export function RowCheckbox({
  selection,
  id,
  label,
  failure,
  disabled,
}: {
  selection: Selection
  id: string
  label: string
  /** Il motivo per cui l'ultima azione non ha toccato questa riga, già tradotto. */
  failure?: string | undefined
  disabled?: boolean | undefined
}) {
  return (
    <span className="inline-flex flex-col gap-1">
      <input
        type="checkbox"
        className="size-4 accent-accent"
        aria-label={`${t.selection.selectRow} — ${label}`}
        checked={selection.isSelected(id)}
        disabled={disabled}
        onClick={(event) => event.stopPropagation()}
        onChange={() => selection.toggle(id)}
      />
      {failure ? (
        <span role="status" className="max-w-48 text-xs leading-tight text-danger">
          {failure}
        </span>
      ) : null}
    </span>
  )
}

export function TdSelect(props: ComponentProps<typeof RowCheckbox> & { className?: string }) {
  const { className, ...checkbox } = props
  return (
    <Td className={cn('w-8 px-2', className)}>
      <RowCheckbox {...checkbox} />
    </Td>
  )
}

/**
 * La barra che compare quando c'è almeno una riga selezionata: il conteggio, il
 * verbo e l'avanzamento.
 *
 * Il conteggio dice **su quante righe caricate**, e mai «tutti»: è la decisione 4
 * della sezione 7 resa leggibile. `action` è il bottone del verbo, costruito dalla
 * schermata perché è lei a sapere quale verbo e quale conferma; `progress` è
 * «k di N» mentre il giro è in corso, e sostituisce il bottone perché premerlo
 * due volte farebbe partire due giri sulle stesse righe.
 */
export function SelectionBar({
  selection,
  action,
  progress,
  onClear = selection.clear,
}: {
  selection: Selection
  action: ReactNode
  progress?: { done: number; total: number } | null | undefined
  /** Deselezionare: di default svuota la selezione, e chi ha dei motivi per
   *  riga da dimenticare lo dice qui. */
  onClear?: () => void
}) {
  if (selection.count === 0 && !progress) return null
  return (
    <div
      role="region"
      aria-label={t.selection.region}
      className="flex flex-wrap items-center gap-3 border-b border-border-subtle bg-surface-muted/60 px-4 py-2 text-sm"
    >
      <span aria-live="polite" className="font-medium">
        {progress
          ? t.selection.progress
              .replace('{done}', String(progress.done))
              .replace('{total}', String(progress.total))
          : t.selection.count
              .replace('{count}', String(selection.count))
              .replace('{total}', String(selection.total))}
      </span>
      {progress ? null : (
        <span className="flex flex-wrap items-center gap-2">
          {action}
          <Button size="sm" variant="ghost" onClick={onClear}>
            {t.selection.clear}
          </Button>
        </span>
      )}
    </div>
  )
}
