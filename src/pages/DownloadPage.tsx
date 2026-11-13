import { ClipboardCheck, X } from "lucide-react"
import { useEffect, useRef, useState } from "react"

import { ErrorNote } from "@/components/common/ErrorNote"
import { MediaPreview } from "@/components/download/MediaPreview"
import { PlaylistPreview } from "@/components/download/PlaylistPreview"
import { UrlBar, type UrlBarState } from "@/components/download/UrlBar"
import { JobRow } from "@/components/queue/JobRow"
import { Button } from "@/components/ui/button"
import { Skeleton } from "@/components/ui/skeleton"
import { api } from "@/lib/api"
import type { AppError, MediaInfo } from "@/lib/types"
import { ACTIVE, useQueueList } from "@/store/downloads"
import { useSettings } from "@/store/settings"
import { useUi } from "@/store/ui"

type Phase =
  | { kind: "idle" }
  | { kind: "invalid"; message: string }
  | { kind: "analyzing"; url: string }
  | { kind: "ready"; url: string; info: MediaInfo }
  | { kind: "error"; url: string; error: AppError }

function PreviewSkeleton() {
  return (
    <div className="rounded-3xl bg-card p-5 shadow-raised" aria-busy="true" aria-label="Analisando o link">
      <div className="flex gap-5">
        <Skeleton className="aspect-video w-[300px] rounded-2xl" />
        <div className="flex flex-1 flex-col gap-3 pt-1">
          <Skeleton className="h-5 w-4/5" />
          <Skeleton className="h-5 w-2/5" />
          <Skeleton className="mt-2 h-3.5 w-1/3" />
        </div>
      </div>
      <div className="mt-6 grid grid-cols-6 gap-2">
        {Array.from({ length: 6 }, (_, i) => (
          <Skeleton key={i} className="h-14 rounded-xl" />
        ))}
      </div>
      <p className="mt-4 text-[12.5px] text-muted-foreground">Consultando as qualidades disponíveis na fonte…</p>
    </div>
  )
}

export function DownloadPage() {
  const settings = useSettings((s) => s.settings)
  const clipboardUrl = useUi((s) => s.clipboardUrl)
  const setClipboardUrl = useUi((s) => s.setClipboardUrl)
  const setBatchOpen = useUi((s) => s.setBatchOpen)
  const go = useUi((s) => s.go)
  const queue = useQueueList()
  const [value, setValue] = useState("")
  const [phase, setPhase] = useState<Phase>({ kind: "idle" })
  const request = useRef(0)
  const inputRef = useRef<HTMLInputElement>(null)

  const analyze = async (text: string) => {
    const id = ++request.current
    const check = await api.checkUrl(text)
    if (id !== request.current) return
    if (!check.valid || !check.normalized) {
      setPhase({ kind: "invalid", message: check.reason ?? "Link inválido." })
      return
    }
    const url = check.normalized
    setPhase({ kind: "analyzing", url })
    if (clipboardUrl && clipboardUrl.trim() === text.trim()) setClipboardUrl(null)
    try {
      const info = await api.analyzeUrl(url)
      if (id === request.current) setPhase({ kind: "ready", url, info })
    } catch (e) {
      if (id === request.current) setPhase({ kind: "error", url, error: e as AppError })
    }
  }

  // Link escolhido a partir da sugestão da área de transferência.
  const pendingUrl = useUi((s) => s.pendingUrl)
  const setPendingUrl = useUi((s) => s.setPendingUrl)
  useEffect(() => {
    if (pendingUrl) {
      setValue(pendingUrl)
      analyze(pendingUrl)
      setPendingUrl(null)
    }
  }, [pendingUrl])

  const barState: UrlBarState = phase.kind === "analyzing" ? "busy" : phase.kind === "invalid" ? "invalid" : "idle"
  const active = queue.filter((j) => ACTIVE.includes(j.status) || j.status === "queued").slice(0, 3)
  const showSuggestion = clipboardUrl && clipboardUrl.trim() !== value.trim() && phase.kind !== "analyzing"

  return (
    <div className="mx-auto flex w-full max-w-[920px] flex-col gap-6 px-10 pb-12 pt-10">
      <header>
        <h1 className="text-[28px] font-semibold leading-tight tracking-[-0.025em]">O que vamos baixar hoje?</h1>
        <p className="mt-1.5 text-[14px] text-muted-foreground">
          Cole um link. O Vibax mostra as qualidades que a fonte realmente oferece antes de você baixar.
        </p>
      </header>

      <div>
        <UrlBar
          ref={inputRef}
          value={value}
          state={barState}
          onChange={(v) => {
            setValue(v)
            if (phase.kind === "invalid") setPhase({ kind: "idle" })
          }}
          onSubmit={(text) => analyze(text)}
          onOpenBatch={() => setBatchOpen(true)}
          describedBy="url-help"
        />
        <div id="url-help" className="min-h-9 pt-2" aria-live="polite">
          {phase.kind === "invalid" ? (
            <p className="px-2 text-[13px] text-destructive">{phase.message}</p>
          ) : showSuggestion ? (
            <div className="inline-flex max-w-full items-center gap-2 rounded-full bg-violet-soft py-1 pl-3 pr-1 text-[12.5px]">
              <ClipboardCheck className="size-3.5 shrink-0 text-primary" strokeWidth={2} />
              <span className="shrink-0 text-muted-foreground">Link copiado</span>
              <span className="truncate font-medium">{clipboardUrl}</span>
              <Button
                size="sm"
                className="press h-6 rounded-full px-2.5 text-[12px]"
                onClick={() => {
                  setValue(clipboardUrl)
                  analyze(clipboardUrl)
                }}
              >
                Usar
              </Button>
              <button
                type="button"
                aria-label="Dispensar sugestão"
                onClick={() => setClipboardUrl(null)}
                className="grid size-6 place-items-center rounded-full text-muted-foreground hover:bg-foreground/8 hover:text-foreground"
              >
                <X className="size-3.5" />
              </button>
            </div>
          ) : null}
        </div>
      </div>

      {phase.kind === "analyzing" && <PreviewSkeleton />}
      {phase.kind === "error" && (
        <ErrorNote
          error={phase.error}
          action={
            <Button size="sm" variant="outline" className="press h-7 shrink-0" onClick={() => analyze(phase.url)}>
              Tentar de novo
            </Button>
          }
        />
      )}
      {phase.kind === "ready" &&
        settings &&
        (phase.info.kind === "playlist" ? (
          <PlaylistPreview key={phase.url} info={phase.info} settings={settings} />
        ) : (
          <MediaPreview key={phase.url} info={phase.info} analyzedUrl={phase.url} settings={settings} />
        ))}

      {active.length > 0 && (
        <section aria-label="Em andamento" className="mt-2">
          <div className="mb-1 flex items-center justify-between px-3">
            <h2 className="text-[13px] font-medium text-muted-foreground">Em andamento</h2>
            <Button variant="link" size="sm" className="h-auto px-0 text-[12.5px]" onClick={() => go("queue")}>
              Ver fila completa
            </Button>
          </div>
          <ul className="flex flex-col">
            {active.map((j) => (
              <JobRow key={j.id} job={j} compact />
            ))}
          </ul>
        </section>
      )}

      {phase.kind === "idle" && active.length === 0 && (
        <p className="px-2 text-[12.5px] leading-relaxed text-muted-foreground/80">
          Use o Vibax apenas com conteúdos próprios, licenciados ou cujo download seja permitido pela fonte. Conteúdos com
          DRM, login ou outras restrições de acesso não são baixados.
        </p>
      )}
    </div>
  )
}
