import { renderHook } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { useIntentKey } from './idempotency'

/**
 * La difesa contro il doppio incasso, verificata dal lato in cui si rompe.
 *
 * **Il difetto che questi test escludono è silenzioso**: una chiave rigenerata a
 * ogni tentativo produce un secondo pagamento, il saldo scende due volte, e
 * `audit_log` mostra due inserimenti perfettamente legittimi. Non c'è un errore da
 * vedere — c'è un numero sbagliato, e nessun indizio che lo sia.
 *
 * Ne segue che il test da scrivere non è «la chiave esiste» ma **«è la stessa»**,
 * ripetuta, e «cambia quando l'intento cambia» — perché l'errore opposto è
 * altrettanto muto: due incassi distinti che condividessero la chiave darebbero un
 * `200` con il primo, e il secondo non entrerebbe mai.
 */
describe('la chiave di idempotenza', () => {
  it('è la stessa a ogni tentativo dello stesso intento', () => {
    const { result } = renderHook(() => useIntentKey())

    const first = result.current.current()
    const retry = result.current.current()
    const secondRetry = result.current.current()

    expect(first).toBe(retry)
    expect(first).toBe(secondRetry)
    expect(first).not.toBe('')
  })

  it('sopravvive a un render, che è ciò che un ritentativo provoca', () => {
    const { result, rerender } = renderHook(() => useIntentKey())

    const first = result.current.current()
    // Un errore mostrato, uno stato di invio che cambia: il componente si ridisegna
    // molte volte fra il primo tentativo e il secondo.
    rerender()
    rerender()

    expect(result.current.current()).toBe(first)
  })

  it('cambia solo quando si dichiara un intento nuovo', () => {
    const { result } = renderHook(() => useIntentKey())

    const first = result.current.current()
    result.current.renew()
    const second = result.current.current()

    expect(second).not.toBe(first)
    // E il nuovo intento è a sua volta stabile: `renew` apre un intento, non lo
    // rende usa e getta.
    expect(result.current.current()).toBe(second)
  })

  it('restituisce un oggetto stabile fra i render', () => {
    const { result, rerender } = renderHook(() => useIntentKey())

    const api = result.current
    rerender()

    // Se cambiasse, una `useMutation` che ne dipendesse si ricostruirebbe a ogni
    // render e perderebbe lo stato dell'invio in corso.
    expect(result.current).toBe(api)
  })

  it('non ricicla la chiave fra due intenti diversi sullo stesso schermo', () => {
    const first = renderHook(() => useIntentKey())
    const second = renderHook(() => useIntentKey())

    expect(first.result.current.current()).not.toBe(second.result.current.current())
  })
})
