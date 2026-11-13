import {
  CircleCheck,
  CircleSlash,
  Clock3,
  ExternalLink,
  FolderOpen,
  Loader2,
  OctagonAlert,
  Pause,
  Play,
  RotateCcw,
  Trash2,
  X,
} from "lucide-react"
import { toast } from "sonner"

import { ErrorNote } from "@/components/common/ErrorNote"
import { IconAction } from "@/components/common/IconAction"
import { Thumb } from "@/components/common/Thumb"
import { api } from "@/lib/api"
import { formatBytes, formatEta, formatPercent, formatSpeed } from "@/lib/format"
import type { Job, JobStatus } from "@/lib/types"
import { cn } from "@/lib/utils"

export const STATUS_LABEL: Record<JobStatus, string> = {
  queued: "Aguardando",
  analyzing: "Analisando",
  downloading: "Baixando",
  processing: "Processando",
  paused: "Pausado",
  completed: "Concluído",
  cancelled: "Cancelado",
  error: "Erro",
}

function StatusIcon({ status }: { status: JobStatus }) {
  const cls = "size-3.5"
  switch (status) {
    case "queued":
      return <Clock3 className={cls} />
    case "analyzing":
    case "downloading":
    case "processing":
      return <Loader2 className={cn(cls, "animate-spin motion-reduce:animate-none")} />
    case "paused":
      return <Pause className={cls} />
    case "completed":
      return <CircleCheck className={cls} />
    case "cancelled":
      return <CircleSlash className={cls} />
    case "error":
      return <OctagonAlert className={cls} />
  }
}

const STATUS_TONE: Record<JobStatus, string> = {
  queued: "text-muted-foreground",
  analyzing: "text-primary",
  downloading: "text-primary",
  processing: "text-primary",
  paused: "text-warning",
  completed: "text-success",
  cancelled: "text-muted-foreground",
  error: "text-destructive",
}

/** Executa uma ação e mostra erro de forma clara se falhar. */
export async function act(fn: () => Promise<unknown>) {
  try {
    await fn()
  } catch (e) {
    const err = e as { message?: string }
    toast.error(err.message ?? "Não foi possível concluir a ação.")
  }
}

function ProgressBar({ job }: { job: Job }) {
  const indeterminate =
    job.status === "analyzing" || job.status === "processing" || (job.status === "downloading" && job.progress == null)
  const pct = job.status === "completed" ? 100 : Math.round((job.progress ?? 0) * 1000) / 10
  return (
    <div
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={indeterminate ? undefined : pct}
      aria-label="Progresso do download"
      className="relative h-1 overflow-hidden rounded-full bg-foreground/8"
    >
      {indeterminate ? (
        <div className="indeterminate absolute inset-0" />
      ) : (
        <div
          className={cn(
            "h-full rounded-full transition-[width] duration-300 ease-out",
            job.status === "completed" && "bg-success",
            job.status === "error" && "bg-destructive",
            job.status === "paused" && "bg-warning",
            job.status === "cancelled" && "bg-muted-foreground/40",
            ["downloading", "queued"].includes(job.status) && "bg-primary",
          )}
          style={{ width: `${pct}%` }}
        />
      )}
    </div>
  )
}

function Metrics({ job }: { job: Job }) {
  const size = job.fileSize ?? job.totalBytes
  if (job.status === "downloading") {
    return (
      <div className="tnum flex flex-wrap items-center gap-x-4 gap-y-0.5 text-[12px] text-muted-foreground">
        <span className="font-medium text-foreground">{formatPercent(job.progress)}</span>
        <span>
          {formatBytes(job.downloadedBytes)}
          {size ? ` de ${formatBytes(size)}` : ""}
        </span>
        {job.speed ? <span>{formatSpeed(job.speed)}</span> : null}
        {job.eta != null ? <span>faltam {formatEta(job.eta)}</span> : null}
        {job.stage && <span className="text-primary/90">{job.stage}</span>}
      </div>
    )
  }
  const parts: React.ReactNode[] = []
  if (job.status === "processing" || job.status === "analyzing") parts.push(job.stage ?? STATUS_LABEL[job.status])
  else if (job.status === "paused") {
    parts.push(job.stage ?? `Pausado em ${formatPercent(job.progress)}`)
    if (size) parts.push(`${formatBytes(job.downloadedBytes)} de ${formatBytes(size)}`)
  } else if (job.status === "queued") parts.push(size ? `Aguardando vaga, ${formatBytes(size)}` : "Aguardando vaga na fila")
  else if (job.status === "completed") parts.push(formatBytes(size))
  else if (job.status === "cancelled") parts.push("Download cancelado")
  return (
    <div className="tnum flex flex-wrap items-center gap-x-4 text-[12px] text-muted-foreground">
      {parts.map((p, i) => (
        <span key={i}>{p}</span>
      ))}
    </div>
  )
}

export function JobActions({ job, compact }: { job: Job; compact?: boolean }) {
  const s = job.status
  return (
    <div className="flex shrink-0 items-center gap-0.5">
      {(s === "downloading" || s === "queued") && (
        <IconAction label="Pausar" onClick={() => act(() => api.pauseJob(job.id))}>
          <Pause className="size-4" strokeWidth={1.75} />
        </IconAction>
      )}
      {s === "paused" && (
        <IconAction label="Retomar" tone="primary" onClick={() => act(() => api.resumeJob(job.id))}>
          <Play className="size-4" strokeWidth={1.75} />
        </IconAction>
      )}
      {(s === "error" || s === "cancelled") && (
        <IconAction label="Tentar de novo" tone="primary" onClick={() => act(() => api.retryJob(job.id))}>
          <RotateCcw className="size-4" strokeWidth={1.75} />
        </IconAction>
      )}
      {s === "completed" && (
        <>
          <IconAction label="Abrir arquivo" tone="primary" onClick={() => act(() => api.openJobFile(job.id))}>
            <ExternalLink className="size-4" strokeWidth={1.75} />
          </IconAction>
          <IconAction label="Mostrar na pasta" onClick={() => act(() => api.revealJobFile(job.id))}>
            <FolderOpen className="size-4" strokeWidth={1.75} />
          </IconAction>
        </>
      )}
      {["queued", "analyzing", "downloading", "processing", "paused"].includes(s) ? (
        <IconAction label="Cancelar download" tone="danger" onClick={() => act(() => api.cancelJob(job.id))}>
          <X className="size-4" strokeWidth={1.75} />
        </IconAction>
      ) : (
        !compact && (
          <IconAction
            label={s === "completed" ? "Remover da fila (continua no histórico)" : "Remover da fila"}
            tone="danger"
            onClick={() => act(() => api.removeJob(job.id))}
          >
            <Trash2 className="size-4" strokeWidth={1.75} />
          </IconAction>
        )
      )}
    </div>
  )
}

export function JobRow({ job, compact }: { job: Job; compact?: boolean }) {
  const isAudio = job.choice?.kind === "audio" || job.preference?.kind === "audio"
  return (
    <li className={cn("group flex gap-4 rounded-2xl px-3 py-3 transition-colors duration-150 hover:bg-foreground/3")}>
      <Thumb localPath={job.thumbPath} remote={job.thumbnail} audio={isAudio} className={compact ? "w-24" : "w-32"} />
      <div className="flex min-w-0 flex-1 flex-col justify-center gap-1.5">
        <div className="flex items-start gap-3">
          <div className="min-w-0 flex-1">
            <p className="truncate text-[14px] font-medium leading-snug" title={job.title ?? job.url}>
              {job.title ?? job.url}
            </p>
            <div className="mt-1 flex flex-wrap items-center gap-1.5 text-[11.5px]">
              <span className={cn("inline-flex items-center gap-1 font-medium", STATUS_TONE[job.status])}>
                <StatusIcon status={job.status} />
                {STATUS_LABEL[job.status]}
              </span>
              <span className="rounded-md bg-muted px-1.5 py-px font-medium text-muted-foreground">{job.formatLabel}</span>
              {job.qualityLabel && (
                <span className="rounded-md bg-muted px-1.5 py-px font-medium text-muted-foreground">{job.qualityLabel}</span>
              )}
              {job.sourceLabel && !compact && <span className="text-muted-foreground">{job.sourceLabel}</span>}
            </div>
          </div>
          <JobActions job={job} compact={compact} />
        </div>
        {job.status !== "error" && job.status !== "cancelled" && job.status !== "completed" && <ProgressBar job={job} />}
        {job.status === "error" && job.error ? <ErrorNote error={job.error} compact /> : <Metrics job={job} />}
      </div>
    </li>
  )
}
