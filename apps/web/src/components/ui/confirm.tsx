import { useState, type ReactNode } from 'react'

import { InlineError } from '@/components/state'
import { Button, type ButtonProps } from '@/components/ui/button'
import { Dialog, DialogContent, DialogTrigger } from '@/components/ui/dialog'
import { t } from '@/i18n'

/**
 * Conferma di un'azione che non si disfa da sé.
 *
 * **Spiega che cosa accade, non chiede «sei sicuro?»**: la seconda domanda non
 * aggiunge informazione, e chi la legge la conferma per riflesso. Qui la modale
 * dice che cosa cambia — «sparisce dagli elenchi», «resta visibile a chi doveva
 * pagarlo» — che è ciò che permette di decidere davvero.
 *
 * L'errore resta **dentro** la modale: chiuderla e mostrare l'errore altrove
 * farebbe perdere il contesto proprio a chi ha appena fatto qualcosa di
 * irreversibile.
 */
export function ConfirmAction({
  trigger,
  title,
  explanation,
  confirmLabel,
  variant = 'danger',
  onConfirm,
  children,
}: {
  trigger: ReactNode
  title: string
  explanation: string
  confirmLabel?: string
  variant?: ButtonProps['variant']
  onConfirm: () => Promise<unknown>
  /** Campi aggiuntivi, per le conferme che chiedono un dato (una data, un motivo). */
  children?: ReactNode
}) {
  const [open, setOpen] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<unknown>(null)

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) setError(null)
      }}
    >
      <DialogTrigger asChild>{trigger}</DialogTrigger>
      <DialogContent title={title} description={explanation}>
        <div className="flex flex-col gap-4">
          <InlineError error={error} />
          {children}
          <div className="flex justify-end gap-2">
            <Button onClick={() => setOpen(false)} disabled={busy}>
              {t.common.cancel}
            </Button>
            <Button
              variant={variant}
              disabled={busy}
              onClick={async () => {
                setBusy(true)
                setError(null)
                try {
                  await onConfirm()
                  setOpen(false)
                } catch (failure) {
                  setError(failure)
                } finally {
                  setBusy(false)
                }
              }}
            >
              {busy ? t.common.saving : (confirmLabel ?? t.common.confirm)}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}

/**
 * Una modale con un form dentro.
 *
 * Separata da [`ConfirmAction`] perché il gesto è diverso: lì si conferma, qui si
 * compone. Il form **non perde ciò che è stato digitato** quando la chiamata
 * fallisce — la modale resta aperta con i campi pieni, che è la sola cosa che
 * distingue un errore recuperabile da un lavoro da rifare.
 */
export function FormDialog({
  trigger,
  title,
  description,
  open,
  onOpenChange,
  children,
}: {
  trigger?: ReactNode
  title: string
  description?: string | undefined
  open?: boolean
  onOpenChange?: (open: boolean) => void
  children: ReactNode
}) {
  return (
    <Dialog {...(open === undefined ? {} : { open })} {...(onOpenChange ? { onOpenChange } : {})}>
      {trigger ? <DialogTrigger asChild>{trigger}</DialogTrigger> : null}
      <DialogContent title={title} {...(description ? { description } : {})}>
        {children}
      </DialogContent>
    </Dialog>
  )
}
