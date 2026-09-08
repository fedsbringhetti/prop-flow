import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Trash2, Upload } from 'lucide-react'
import { useRef, useState, type ReactNode } from 'react'

import {
  UPLOAD_LIMIT_BYTES,
  archiveDocument,
  documentContentPath,
  listDocuments,
  uploadDocument,
  type DocumentOwner,
} from '@/api/endpoints'
import { ApiError } from '@/api/problem'
import type { StoredDocument } from '@/api/types'
import { Async, Empty, InlineError } from '@/components/state'
import { Button } from '@/components/ui/button'
import { Card, CardBody, CardHeader, CardTitle } from '@/components/ui/card'
import { ConfirmAction } from '@/components/ui/confirm'
import { keys } from '@/app/query'
import { useApi, useIsStaff, useSession } from '@/app/session'
import { t } from '@/i18n'
import { formatBytes } from '@/lib/format'

/**
 * Gli allegati di un oggetto, **uno solo per tutti e sei**.
 *
 * Nasce estraendo la `ScansCard` delle bollette, che era il modello e l'unico
 * posto in cui esisteva. Copiarla sugli altri cinque owner avrebbe copiato anche
 * le tre cose che è facile sbagliare una volta e impossibile sbagliare sei volte
 * nello stesso modo: il controllo della dimensione prima dell'invio, il download
 * come `<a href>` invece che come `fetch`, e l'invalidazione della chiave giusta.
 *
 * # Il controllo della dimensione sta **prima** di inviare
 *
 * Il limite del corpo su queste rotte è 25 MiB, e oltre quello la risposta è un
 * `413` che non dice né quale limite né su quale file. Una fotografia da 30 MB
 * scelta da un telefono ci finisce dentro senza fatica.
 *
 * # Il download è un link, e non porta l'`Authorization`
 *
 * La rotta risponde `307` verso un URL pre-firmato: seguirlo con `fetch`
 * scaricherebbe i byte in memoria per poi doverli riconsegnare al browser. Un
 * `<a href>` lascia fare al browser ciò che sa fare, e funziona perché è il
 * browser a seguirlo nel contesto della scheda. È un debito registrato: il giorno
 * in cui l'API pretendesse l'header su quella rotta, il download smetterebbe di
 * funzionare **senza che nulla in questo repository cambi**.
 */
export function AttachmentsCard({
  owner,
  title,
  hint,
  kind,
  canUpload = true,
  emptyTitle,
  actions,
  onUploaded,
}: {
  owner: DocumentOwner
  title: string
  hint?: string
  /** Classificazione del documento. Assente vale `other` lato API. */
  kind?: string
  /**
   * Se chi guarda può allegare.
   *
   * Caricare è di chi è **membro** — l'inquilino allega al proprio contratto e
   * fotografa il guasto che segnala — quindi il default è vero, e le schermate
   * che vogliono restringerlo lo dicono. **Scaricare non si restringe mai**: un
   * allegato è la prova di ciò che è stato addebitato, e chi paga deve poterlo
   * aprire.
   */
  canUpload?: boolean
  emptyTitle?: string
  /** Azioni aggiuntive nell'intestazione, accanto al bottone di caricamento. */
  actions?: ReactNode
  /**
   * Che cosa rileggere **oltre** all'elenco, dopo un caricamento riuscito.
   *
   * Esiste per un caso solo, e conviene dirlo qui perché non se ne inventino
   * altri: su una bolletta in bozza il caricamento accoda la lettura automatica,
   * e il job non compare nella risposta: le chiavi dell'OCR vanno rilette. È
   * conoscenza della bolletta, non della scheda, quindi vive dove sta.
   */
  onUploaded?: () => void | Promise<void>
}) {
  const client = useApi()
  const queryClient = useQueryClient()
  const input = useRef<HTMLInputElement>(null)
  const [error, setError] = useState<unknown>(null)

  const queryKey = keys.documents(owner.type, owner.id)

  const documents = useQuery({
    queryKey,
    queryFn: () => listDocuments(client, owner),
  })

  const upload = useMutation({
    mutationFn: (file: File) => uploadDocument(client, owner, file, kind),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey })
      await onUploaded?.()
      setError(null)
    },
    onError: setError,
  })

  return (
    <Card>
      <CardHeader>
        <div>
          <CardTitle>{title}</CardTitle>
          {hint && canUpload ? <p className="mt-0.5 text-xs text-ink-muted">{hint}</p> : null}
        </div>
        {/* Le azioni **avvolgono**: a 375 px un bottone che non va a capo fa
            scorrere in orizzontale la pagina intera. */}
        <div className="flex min-w-0 flex-wrap gap-2">
          {actions}
          {canUpload ? (
            <div>
              <input
                ref={input}
                type="file"
                className="sr-only"
                accept="application/pdf,image/*"
                onChange={(event) => {
                  const file = event.target.files?.[0]
                  // Azzerato **prima** di ogni altra cosa: senza, riscegliere lo
                  // stesso file non emette un secondo `change` e il caricamento
                  // ritentato non parte.
                  event.target.value = ''
                  if (!file) return
                  if (file.size > UPLOAD_LIMIT_BYTES) {
                    setError(new ApiError({ status: 413, code: 'request.payload_too_large' }))
                    return
                  }
                  upload.mutate(file)
                }}
              />
              <Button size="sm" onClick={() => input.current?.click()} disabled={upload.isPending}>
                <Upload aria-hidden />
                {upload.isPending ? t.common.saving : t.document.upload}
              </Button>
            </div>
          ) : null}
        </div>
      </CardHeader>
      <CardBody className="flex flex-col gap-3">
        <InlineError error={error} />
        <Async
          query={documents}
          isEmpty={(page) => page.items.length === 0}
          empty={<Empty title={emptyTitle ?? t.document.empty} />}
        >
          {(page) => (
            <ul className="divide-y divide-border-subtle">
              {page.items.map((document) => (
                <AttachmentRow key={document.id} document={document} queryKey={queryKey} />
              ))}
            </ul>
          )}
        </Async>
      </CardBody>
    </Card>
  )
}

function AttachmentRow({
  document,
  queryKey,
}: {
  document: StoredDocument
  queryKey: readonly unknown[]
}) {
  const client = useApi()
  const queryClient = useQueryClient()
  const isStaff = useIsStaff()
  const { session } = useSession()

  /**
   * Chi può archiviare, **come lo dice la policy**: staff *oppure* chi ha caricato.
   *
   * È la `USING` di `documents_update` letta alla lettera, e per questo la riga
   * non si accontenta di `useIsStaff()`. Il codice che l'API restituisce a chi non
   * può è `org.staff_role_required` — vero solo a metà, perché all'uploader quel
   * ruolo non serve — e il modo migliore di non ripetere quella mezza verità è non
   * offrire il bottone a chi lo riceverebbe.
   *
   * **Non è un confine di sicurezza**: a negare è la policy di riga. Qui si evita
   * soltanto di offrire ciò che verrebbe rifiutato.
   */
  const canArchive = isStaff || (document.uploaded_by !== null && document.uploaded_by === session?.userId)

  const archive = useMutation({
    mutationFn: () => archiveDocument(client, document.id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey }),
  })

  return (
    <li className="flex flex-wrap items-center justify-between gap-2 py-2 text-sm">
      <span className="min-w-0 flex-1">
        <span className="block truncate">{document.original_filename}</span>
        <span className="block text-xs text-ink-muted">{formatBytes(document.size_bytes)}</span>
      </span>
      <span className="flex shrink-0 items-center gap-3">
        <a
          href={documentContentPath(document.id)}
          target="_blank"
          rel="noreferrer"
          className="text-accent underline-offset-4 hover:underline"
        >
          {t.document.download}
        </a>
        {canArchive ? (
          <ConfirmAction
            trigger={
              <Button size="sm" variant="dangerGhost" aria-label={t.document.archive}>
                <Trash2 aria-hidden />
              </Button>
            }
            title={t.document.archiveConfirm}
            explanation={t.document.archiveExplain}
            confirmLabel={t.document.archive}
            onConfirm={() => archive.mutateAsync()}
          />
        ) : null}
      </span>
    </li>
  )
}
