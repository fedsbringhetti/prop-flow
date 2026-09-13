import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useState } from 'react'
import { describe, expect, it } from 'vitest'

import { SelectionBar, TableFrame, Td, TdSelect, Th, ThSelect, Tr, useSelection } from './table'

/**
 * La selezione nella primitiva, verificata sulle tre cose che il piano chiede.
 *
 * **La potatura dopo una rilettura** è quella che conta: la selezione è un
 * insieme di identificatori, e quando le righe fatte spariscono dalla pagina il
 * conteggio deve seguirle nello stesso render. Il test toglie una riga dalla
 * tabella **senza toccare la selezione**, come farebbe un refetch, e guarda il
 * numero. Verificato al contrario: con `selected = chosen` invece
 * dell'intersezione, «2 selezionate» resta scritto sopra una riga sola.
 */

function Harness({ initial }: { initial: string[] }) {
  const [rows, setRows] = useState(initial)
  const selection = useSelection(rows)
  return (
    <>
      <SelectionBar selection={selection} action={<button type="button">Archivia</button>} />
      <TableFrame>
        <thead>
          <tr>
            <ThSelect selection={selection} />
            <Th>Nome</Th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <Tr key={row}>
              <TdSelect selection={selection} id={row} label={row} />
              <Td>{row}</Td>
            </Tr>
          ))}
        </tbody>
      </TableFrame>
      {/* Il refetch, simulato: la prima riga è stata fatta e non c'è più. */}
      <button type="button" onClick={() => setRows((current) => current.slice(1))}>
        rileggi
      </button>
    </>
  )
}

describe('la selezione in tabella', () => {
  it('si pota a ogni rilettura: il conteggio segue le righe che restano', async () => {
    const user = userEvent.setup()
    render(<Harness initial={['alfa', 'beta', 'gamma']} />)

    await user.click(screen.getByLabelText('Seleziona — alfa'))
    await user.click(screen.getByLabelText('Seleziona — beta'))
    expect(screen.getByText('2 selezionate su 3 caricate')).toBeInTheDocument()

    await user.click(screen.getByText('rileggi'))

    // «alfa» è sparita con la rilettura: la selezione la lascia andare da sé,
    // senza che nessuno l'abbia deselezionata.
    expect(screen.getByText('1 selezionate su 2 caricate')).toBeInTheDocument()
    expect(screen.queryByLabelText('Seleziona — alfa')).not.toBeInTheDocument()
  })

  it('«seleziona tutto» è la pagina, e il conteggio lo dice', async () => {
    const user = userEvent.setup()
    render(<Harness initial={['alfa', 'beta', 'gamma']} />)

    await user.click(screen.getByLabelText('Seleziona tutte le righe caricate'))
    expect(screen.getByText('3 selezionate su 3 caricate')).toBeInTheDocument()
    expect(screen.queryByText(/tutti/i)).not.toBeInTheDocument()

    // Una seconda pressione svuota.
    await user.click(screen.getByLabelText('Seleziona tutte le righe caricate'))
    expect(screen.queryByText(/selezionate su/)).not.toBeInTheDocument()
  })

  it('la casella di intestazione è indeterminata su una selezione parziale', async () => {
    const user = userEvent.setup()
    render(<Harness initial={['alfa', 'beta']} />)

    await user.click(screen.getByLabelText('Seleziona — alfa'))
    const all = screen.getByLabelText<HTMLInputElement>('Seleziona tutte le righe caricate')
    expect(all.indeterminate).toBe(true)
    expect(all.checked).toBe(false)
  })

  it('si seleziona da tastiera, con la barra spaziatrice', async () => {
    const user = userEvent.setup()
    render(<Harness initial={['alfa', 'beta']} />)

    await user.tab() // la casella di intestazione
    await user.tab() // la prima riga
    expect(screen.getByLabelText('Seleziona — alfa')).toHaveFocus()
    await user.keyboard(' ')
    expect(screen.getByText('1 selezionate su 2 caricate')).toBeInTheDocument()
  })

  it('«Deseleziona» svuota la selezione', async () => {
    const user = userEvent.setup()
    render(<Harness initial={['alfa', 'beta']} />)

    await user.click(screen.getByLabelText('Seleziona — alfa'))
    await user.click(screen.getByText('Deseleziona'))
    expect(screen.queryByText(/selezionate su/)).not.toBeInTheDocument()
  })
})
