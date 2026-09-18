import { useState, type ReactNode } from 'react'

import { runInSequence, type BatchSummary } from '@/api/batch'
import { messageFor } from '@/api/problem'
import { Button, type ButtonProps } from '@/components/ui/button'
import { ConfirmAction } from '@/components/ui/confirm'
import { SelectionBar, type Selection } from '@/components/ui/table'
import { t } from '@/i18n'

/**
 * Il verbo di un'entità, con le sue frasi.
 *
 * Ogni tabella è di **una** entità, quindi ha **un** verbo — Archivia, Annulla —
 * e la conferma dice la conseguenza di quel verbo riusando la frase che la scheda
 * mostra già per l'azione singola. Non si scrive una seconda frase per il blocco:
 * si cambia il numero. `confirmMany` porta `{count}`; `explain` è la stessa frase
 * della scheda, senza numero, perché la conseguenza non cambia con la quantità.
 *
 * **Nessun verbo dice «Elimina».** Il prodotto non cancella niente — il ruolo
 * applicativo non ha il privilegio `DELETE` — e ogni entità esce di scena a modo
 * suo, con una conseguenza diversa che un bottone unico nasconderebbe proprio
 * nel momento in cui una persona sta decidendo.
 */
export interface Verb {
  /** L'etichetta del bottone: «Archivia», «Annulla». */
  label: string
  /** La domanda per una riga: «Archiviare questo immobile?». */
  confirmOne: string
  /** La domanda per N righe, con `{count}`: «Archiviare {count} immobili?». */
  confirmMany: string
  /** La conseguenza, la stessa frase della scheda. */
  explain: string
  /** Una riga in più da dire solo in blocco, quando serve. */
  explainMany?: string | undefined
}

export function confirmTitle(verb: Verb, count: number): string {
  return count === 1 ? verb.confirmOne : verb.confirmMany.replace('{count}', String(count))
}

/**
 * Il giro su N righe, dentro React: avanzamento, motivi per riga, e **una**
 * invalidazione alla fine.
 *
 * Le righe fatte — o già così — escono dalla selezione; quelle non fatte
 * **restano selezionate** con il motivo accanto, così riprovarle è un clic e
 * non una riselezione a mano. `onSettled` rilegge ciò che il giro ha toccato, e
 * viene chiamato una volta sola: venticinque invalidazioni sarebbero venticinque
 * riletture della stessa pagina.
 */
export function useBulkAction({
  selection,
  action,
  onSettled,
}: {
  selection: Selection
  action: (id: string) => Promise<unknown>
  /** Che cosa rileggere dopo il giro. Una volta, non N. */
  onSettled: () => Promise<unknown>
}) {
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null)
  const [failures, setFailures] = useState<ReadonlyMap<string, string>>(() => new Map())
  const [summary, setSummary] = useState<BatchSummary | null>(null)

  async function run(ids: readonly string[]): Promise<BatchSummary> {
    setProgress({ done: 0, total: ids.length })
    setSummary(null)
    try {
      const result = await runInSequence(
        ids,
        action,
        (done, total) => setProgress({ done, total }),
      )
      setFailures((previous) => {
        const next = new Map(previous)
        for (const id of result.settled) next.delete(id)
        for (const failure of result.failed) next.set(failure.id, messageFor(failure.error))
        return next
      })
      selection.remove(result.settled)
      setSummary(result)
      await onSettled()
      return result
    } finally {
      setProgress(null)
    }
  }

  return {
    run,
    progress,
    /** Il motivo per cui l'ultima azione non ha toccato questa riga. */
    failureFor: (id: string): string | undefined => failures.get(id),
    /** L'esito dell'ultimo giro, se qualcosa non è passato. */
    partial:
      summary && summary.failed.length > 0
        ? t.selection.partial
            .replace('{done}', String(summary.settled.length))
            .replace('{failed}', String(summary.failed.length))
        : null,
    /** Dimentica i motivi: dopo una deselezione non c'è più niente da riprovare. */
    forget: () => {
      setFailures(new Map())
      setSummary(null)
    },
  }
}

/**
 * Il bottone del verbo con la sua conferma, per **una** riga o per N.
 *
 * È lo stesso componente nei due posti — la colonna delle azioni e la barra
 * della selezione — perché è la stessa operazione: la riga singola è un giro
 * di lunghezza uno. Tenerne due sarebbe avere due conferme da tenere allineate.
 */
export function VerbButton({
  verb,
  ids,
  run,
  size = 'sm',
  variant = 'ghost',
  icon,
  rowLabel,
}: {
  verb: Verb
  ids: readonly string[]
  run: (ids: readonly string[]) => Promise<unknown>
  size?: ButtonProps['size']
  variant?: ButtonProps['variant']
  icon?: ReactNode
  /** Il nome della riga nell'etichetta accessibile, per il bottone per riga. */
  rowLabel?: string | undefined
}) {
  const explanation =
    ids.length > 1 && verb.explainMany ? `${verb.explain} ${verb.explainMany}` : verb.explain
  return (
    <ConfirmAction
      trigger={
        /* Il nome della riga nell'etichetta accessibile: in una tabella di venti
           righe «Annulla» ripetuto venti volte non dice quale. In `aria-label` e
           non in uno `<span>` nascosto, perché il calcolo del nome accessibile
           incolla i nodi di testo senza spazio. */
        <Button
          size={size}
          variant={variant}
          disabled={ids.length === 0}
          {...(rowLabel ? { 'aria-label': `${verb.label} — ${rowLabel}` } : {})}
        >
          {icon}
          {verb.label}
        </Button>
      }
      title={confirmTitle(verb, ids.length)}
      explanation={explanation}
      confirmLabel={verb.label}
      onConfirm={() => run(ids)}
    />
  )
}

/**
 * La barra della selezione con il verbo dentro, e l'esito parziale sotto.
 *
 * `bulk` e `selection` vengono dagli hook di sopra; la schermata passa solo il
 * verbo. È il pezzo che ogni elenco monta identico, e per questo sta qui e non
 * in sette copie.
 */
export function BulkBar({
  selection,
  verb,
  bulk,
  icon,
}: {
  selection: Selection
  verb: Verb
  bulk: ReturnType<typeof useBulkAction>
  icon?: ReactNode
}) {
  return (
    <>
      <SelectionBar
        selection={selection}
        progress={bulk.progress}
        onClear={() => {
          selection.clear()
          bulk.forget()
        }}
        action={
          <VerbButton
            verb={verb}
            ids={[...selection.selected]}
            run={bulk.run}
            variant="dangerGhost"
            icon={icon}
          />
        }
      />
      {bulk.partial ? (
        <p role="alert" className="border-b border-border-subtle px-4 py-2 text-xs text-danger">
          {bulk.partial}
        </p>
      ) : null}
    </>
  )
}
