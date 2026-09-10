import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { LeaseStatusBadge, PropertyStatusBadge, RoomStatusBadge } from './status'

/**
 * **Il caso che regredisce è l'anomalia, non la norma.**
 *
 * `R30` si legge come «mostrate un badge solo», e chi la applicasse così
 * toglierebbe l'unica cosa che il doppio badge esisteva per dire: una stanza messa
 * in ristrutturazione **mentre ci abita una persona**. La regola vera è più stretta
 * — il dichiarato compare solo quando è un giudizio che il fatto contraddice — e i
 * test che contano sono quelli sulla riga che *resta* doppia.
 *
 * Ogni famiglia ha quindi due assert e non uno: il caso normale che si accorcia, e
 * l'anomalia che non deve accorciarsi con lui.
 */
describe('lo stato dichiarato accanto al derivato', () => {
  it('una stanza disponibile e occupata dice solo «Occupata»', () => {
    render(<RoomStatusBadge status="available" effectiveStatus="occupied" />)
    expect(screen.getByText('Occupata')).toBeInTheDocument()
    expect(screen.queryByText('(Disponibile)')).not.toBeInTheDocument()
  })

  it('una stanza in ristrutturazione e occupata continua a dirlo tutte e due', () => {
    render(<RoomStatusBadge status="unavailable" effectiveStatus="occupied" />)
    expect(screen.getByText('Occupata')).toBeInTheDocument()
    // Qualcuno l'ha messa fuori uso mentre ci abita una persona: è precisamente il
    // caso per cui la parentesi esiste, e l'assert che si toglie semplificando.
    expect(screen.getByText('(Non disponibile)')).toBeInTheDocument()
  })

  it('un immobile disponibile e affittato dice solo «Affittato»', () => {
    render(<PropertyStatusBadge status="available" effectiveStatus="rented" />)
    expect(screen.getByText('Affittato')).toBeInTheDocument()
    expect(screen.queryByText('(Disponibile)')).not.toBeInTheDocument()
  })

  it('un immobile in manutenzione e affittato continua a dirlo tutte e due', () => {
    render(<PropertyStatusBadge status="maintenance" effectiveStatus="rented" />)
    expect(screen.getByText('Affittato')).toBeInTheDocument()
    expect(screen.getByText('(In manutenzione)')).toBeInTheDocument()
  })

  it('una bozza che risulta affittata è comunque un’anomalia da vedere', () => {
    render(<PropertyStatusBadge status="draft" effectiveStatus="partially_rented" />)
    expect(screen.getByText('Parzialmente affittato')).toBeInTheDocument()
    expect(screen.getByText('(Bozza)')).toBeInTheDocument()
  })

  it('un contratto attivo e scaduto dice solo «Scaduto»', () => {
    render(<LeaseStatusBadge status="active" effectiveStatus="expired" />)
    expect(screen.getByText('Scaduto')).toBeInTheDocument()
    expect(screen.queryByText('(Attivo)')).not.toBeInTheDocument()
  })

  it('un contratto senza divergenza dice il suo stato e basta', () => {
    render(<LeaseStatusBadge status="draft" effectiveStatus="draft" />)
    expect(screen.getByText('Bozza')).toBeInTheDocument()
  })

  it('senza stato effettivo si mostra quello memorizzato', () => {
    render(<RoomStatusBadge status="unavailable" />)
    expect(screen.getByText('Non disponibile')).toBeInTheDocument()
    expect(screen.queryByText('(Non disponibile)')).not.toBeInTheDocument()
  })
})
