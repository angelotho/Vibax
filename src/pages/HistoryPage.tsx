import { ExternalLink, FolderOpen, History, RotateCcw, Search, Trash2, TriangleAlert, X } from "lucide-react"
import { useEffect, useMemo, useState } from "react"
import { toast } from "sonner"

import { EmptyState } from "@/components/common/EmptyState"
import { IconAction } from "@/components/common/IconAction"
import { Thumb } from "@/components/common/Thumb"
import { act } from "@/components/queue/JobRow"
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog"
import { Button } from "@/components/ui/button"
import { api } from "@/lib/api"
import { formatBytes, formatDate, formatDuration } from "@/lib/format"
import type { Job } from "@/lib/types"
import { cn } from "@/lib/utils"
import { useDownloads } from "@/store/downloads"
import { useUi } from "@/store/ui"

type Filter = "all" | "video" | "audio"

/** Remove acentos e caixa para uma busca tolerante ("video" encontra "Vídeo"). */
function fold(s: string) {
  return s.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase()
}

function isAudio(j: Job) {
  return j.choice?.kind === "audio" || ["MP3", "M4A", "OPUS"].includes(j.formatLabel)
}

/** Destaca o termo buscado no título, respeitando acentos do texto original. */
function Highlight({ text, query }: { text: string; query: string }) {
  const q = fold(query.trim())
  if (!q) return <>{text}</>
  const folded = fold(text)
  const out: React.ReactNode[] = []
  let i = 0
  let at = folded.indexOf(q)
  while (at !== -1) {
    out.push(text.slice(i, at))
    out.push(
      <mark key={at} className="rounded-sm bg-violet-soft px-0.5 text-foreground">
        {text.slice(at, at + q.length)}
      </mark>,
    )
    i = at + q.length
    at = folded.indexOf(q, i)
  }
  out.push(text.slice(i))
  return <>{out}</>
}

export function HistoryPage() {
  const version = useDownloads((s) => s.historyVersion)
  const go = useUi((s) => s.go)
  const [items, setItems] = useState<Job[] | null>(null)
  const [query, setQuery] = useState("")
  const [filter, setFilter] = useState<Filter>("all")

  const reload = () => api.listHistory().then(setItems).catch(() => setItems([]))
  useEffect(() => {
    reload()
  }, [version])

  const searched = useMemo(() => {
    if (!items) return []
    const q = fold(query.trim())
    if (!q) return items
    return items.filter((j) => fold(`${j.title ?? ""} ${j.uploader ?? ""} ${j.url} ${j.sourceLabel ?? ""}`).includes(q))
  }, [items, query])

  const counts = {
    all: searched.length,
    video: searched.filter((j) => !isAudio(j)).length,
    audio: searched.filter(isAudio).length,
  }
  const visible = searched.filter((j) => filter === "all" || (filter === "audio") === isAudio(j))

  const removeOne = (id: string) =>
    act(async () => {
      await api.removeHistory(id)
      setItems((list) => list?.filter((j) => j.id !== id) ?? null)
    })

  return (
    <div className="mx-auto flex w-full max-w-[1000px] flex-col px-10 pb-12 pt-10">
      <header className="flex items-end justify-between gap-4">
        <div>
          <h1 className="text-[28px] font-semibold leading-tight tracking-[-0.025em]">Histórico</h1>
          <p className="mt-1.5 text-[14px] text-muted-foreground">Downloads concluídos neste computador, do mais recente ao mais antigo.</p>
        </div>
        {items && items.length > 0 && (
          <AlertDialog>
            <AlertDialogTrigger asChild>
              <Button variant="ghost" className="press h-9 gap-2 rounded-xl text-muted-foreground hover:text-destructive">
                <Trash2 className="size-4" strokeWidth={1.75} /> Limpar histórico
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>Limpar todo o histórico?</AlertDialogTitle>
                <AlertDialogDescription>
                  Os registros de {items.length} {items.length === 1 ? "download" : "downloads"} serão apagados. Os arquivos baixados
                  continuam nas pastas onde foram salvos.
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Manter histórico</AlertDialogCancel>
                <AlertDialogAction
                  variant="destructive"
                  onClick={() =>
                    act(async () => {
                      await api.clearHistory()
                      setItems([])
                      toast.success("Histórico limpo")
                    })
                  }
                >
                  Limpar histórico
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        )}
      </header>

      {items && items.length === 0 ? (
        <EmptyState
          icon={History}
          title="Nada no histórico ainda"
          action={
            <Button className="press rounded-xl" onClick={() => go("download")}>
              Baixar o primeiro vídeo
            </Button>
          }
        >
          Os downloads concluídos ficam registrados aqui para você encontrar os arquivos depois.
        </EmptyState>
      ) : (
        <>
          <div className="mt-6 flex flex-wrap items-center gap-3">
            <div className="relative min-w-[320px] flex-1">
              <Search className="pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
              <input
                type="search"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={(e) => e.key === "Escape" && setQuery("")}
                placeholder="Buscar por título, canal ou link"
                aria-label="Buscar no histórico"
                autoFocus
                className="h-10 w-full rounded-xl bg-muted pl-10 pr-10 text-[13.5px] outline-none ring-1 ring-transparent placeholder:text-muted-foreground/70 focus:ring-primary [&::-webkit-search-cancel-button]:hidden"
              />
              {query && (
                <button
                  type="button"
                  aria-label="Limpar busca"
                  onClick={() => setQuery("")}
                  className="absolute right-2 top-1/2 grid size-7 -translate-y-1/2 place-items-center rounded-lg text-muted-foreground hover:bg-foreground/8 hover:text-foreground"
                >
                  <X className="size-3.5" />
                </button>
              )}
            </div>
            <div role="radiogroup" aria-label="Tipo" className="inline-flex rounded-xl bg-muted p-1">
              {(
                [
                  ["all", "Todos"],
                  ["video", "Vídeos"],
                  ["audio", "Áudios"],
                ] as [Filter, string][]
              ).map(([f, label]) => (
                <button
                  key={f}
                  type="button"
                  role="radio"
                  aria-checked={filter === f}
                  onClick={() => setFilter(f)}
                  className={cn(
                    "press tnum h-8 rounded-lg px-3 text-[13px] font-medium text-muted-foreground hover:text-foreground",
                    filter === f && "bg-raised text-foreground shadow-raised",
                  )}
                >
                  {label} <span className="text-muted-foreground/80">{counts[f]}</span>
                </button>
              ))}
            </div>
          </div>

          <p className="tnum mt-3 px-1 text-[12.5px] text-muted-foreground" aria-live="polite">
            {query.trim()
              ? `${visible.length} ${visible.length === 1 ? "resultado" : "resultados"} para “${query.trim()}”`
              : items
                ? `${visible.length} ${visible.length === 1 ? "download" : "downloads"}`
                : "Carregando…"}
          </p>

          {items && visible.length === 0 ? (
            <div className="mt-8 flex flex-col items-center text-center">
              <p className="text-[14px] font-medium">Nenhum download encontrado para “{query.trim() || "este filtro"}”</p>
              <p className="mt-1 text-[13px] text-muted-foreground">Confira a grafia ou tente uma parte do título ou o nome do canal.</p>
              <div className="mt-4 flex gap-2">
                {query && (
                  <Button variant="outline" className="press rounded-xl" onClick={() => setQuery("")}>
                    Limpar busca
                  </Button>
                )}
                {filter !== "all" && (
                  <Button variant="ghost" className="press rounded-xl" onClick={() => setFilter("all")}>
                    Mostrar todos os tipos
                  </Button>
                )}
              </div>
            </div>
          ) : (
            <ul className="mt-2 flex flex-col">
              {visible.map((j) => (
                <li key={j.id} className="flex items-center gap-4 rounded-2xl px-3 py-2.5 transition-colors duration-150 hover:bg-foreground/3">
                  <Thumb localPath={j.thumbPath} remote={j.thumbnail} audio={isAudio(j)} className="w-28" />
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-[14px] font-medium" title={j.title ?? ""}>
                      <Highlight text={j.title ?? j.url} query={query} />
                    </p>
                    <div className="tnum mt-1 flex flex-wrap items-center gap-x-3 gap-y-0.5 text-[12px] text-muted-foreground">
                      <span className="rounded-md bg-muted px-1.5 py-px font-medium">{j.formatLabel}</span>
                      {j.qualityLabel && <span className="rounded-md bg-muted px-1.5 py-px font-medium">{j.qualityLabel}</span>}
                      <span>{formatBytes(j.fileSize)}</span>
                      {j.duration != null && <span>{formatDuration(j.duration)}</span>}
                      {j.uploader && (
                        <span className="max-w-[200px] truncate">
                          <Highlight text={j.uploader} query={query} />
                        </span>
                      )}
                      <span>{formatDate(j.finishedAt)}</span>
                    </div>
                    {j.fileExists === false && (
                      <p className="mt-1 inline-flex items-center gap-1.5 text-[12px] text-warning">
                        <TriangleAlert className="size-3.5" /> O arquivo foi movido ou excluído da pasta original.
                      </p>
                    )}
                  </div>
                  <div className="flex shrink-0 items-center gap-0.5">
                    {j.fileExists !== false && (
                      <>
                        <IconAction label="Abrir arquivo" tone="primary" onClick={() => act(() => api.openJobFile(j.id))}>
                          <ExternalLink className="size-4" strokeWidth={1.75} />
                        </IconAction>
                        <IconAction label="Mostrar na pasta" onClick={() => act(() => api.revealJobFile(j.id))}>
                          <FolderOpen className="size-4" strokeWidth={1.75} />
                        </IconAction>
                      </>
                    )}
                    <IconAction
                      label="Baixar novamente"
                      onClick={() =>
                        act(async () => {
                          await api.redownload(j.id)
                          toast.success("Adicionado à fila", { action: { label: "Ver fila", onClick: () => go("queue") } })
                        })
                      }
                    >
                      <RotateCcw className="size-4" strokeWidth={1.75} />
                    </IconAction>
                    <IconAction label="Remover do histórico" tone="danger" onClick={() => removeOne(j.id)}>
                      <Trash2 className="size-4" strokeWidth={1.75} />
                    </IconAction>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </>
      )}
    </div>
  )
}
