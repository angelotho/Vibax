import { ArrowDownToLine, Clock3, Globe, Loader2, UserRound } from "lucide-react"
import { useMemo, useState } from "react"
import { toast } from "sonner"

import { ChoiceGroup } from "@/components/common/ChoiceGroup"
import { Thumb } from "@/components/common/Thumb"
import { Button } from "@/components/ui/button"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { api } from "@/lib/api"
import { formatBytes, formatDuration } from "@/lib/format"
import type { AudioFormat, Container, DownloadChoice, MediaInfo, QualityOption, Settings } from "@/lib/types"
import { useUi } from "@/store/ui"

import { DestinationField } from "./DestinationField"
import { MP3_BITRATES } from "./PreferencePicker"

const CONTAINER_LABEL: Record<Container, string> = { mp4: "MP4", webm: "WebM", mkv: "MKV" }

function containerHint(container: Container, vcodec: string): string {
  if (container === "mp4") {
    if (vcodec === "H.264") return "H.264: abre em qualquer player, TV ou celular."
    if (vcodec === "AV1") return "AV1 em MP4: arquivo menor; players antigos podem não reproduzir."
    if (vcodec === "VP9") return "VP9 em MP4, sem recompressão; alguns players antigos não reproduzem."
    return `${vcodec || "Vídeo"} em MP4.`
  }
  if (container === "webm") return `${vcodec} com áudio Opus: formato aberto, ideal para navegadores.`
  return `${vcodec || "Vídeo"} em MKV: aceita qualquer codec, ideal para players como VLC.`
}

function defaultQuality(qualities: QualityOption[], pref: number): QualityOption | undefined {
  if (!qualities.length) return undefined
  if (pref === 0) return qualities[0]
  return qualities.find((q) => q.height <= pref) ?? qualities[qualities.length - 1]
}

export function MediaPreview({ info, analyzedUrl, settings }: { info: MediaInfo; analyzedUrl: string; settings: Settings }) {
  const go = useUi((s) => s.go)
  const hasVideo = info.qualities.length > 0
  const hasAudio = info.audio != null || info.qualities.some((q) => q.containers.some((c) => c.videoHasAudio))

  const [mode, setMode] = useState<"video" | "audio">(() =>
    !hasVideo ? "audio" : !hasAudio ? "video" : settings.defaultKind,
  )
  const [height, setHeight] = useState<number>(() => defaultQuality(info.qualities, settings.defaultQuality)?.height ?? 0)
  const quality = info.qualities.find((q) => q.height === height) ?? info.qualities[0]
  const available = quality?.containers ?? []
  const [containerPref, setContainerPref] = useState<Container>(settings.defaultContainer)
  const option = available.find((c) => c.container === containerPref) ?? available[0]
  const [audioFormat, setAudioFormat] = useState<AudioFormat>(settings.defaultAudioFormat)
  const [bitrate, setBitrate] = useState<number>(settings.mp3Bitrate)
  const [dest, setDest] = useState(settings.downloadDir)
  const [busy, setBusy] = useState(false)

  const estimate = useMemo(() => {
    if (mode === "video") return option?.sizeApprox ?? null
    if (audioFormat === "mp3" && info.duration) return (info.duration * bitrate * 1000) / 8
    return info.audio?.sizeApprox ?? null
  }, [mode, option, audioFormat, bitrate, info])

  const submit = async () => {
    let choice: DownloadChoice
    if (mode === "video") {
      if (!quality || !option) return
      choice = {
        kind: "video",
        height: quality.height,
        label: quality.label,
        container: option.container,
        videoFormatId: option.videoFormatId,
        audioFormatId: option.audioFormatId,
        videoHasAudio: option.videoHasAudio,
      }
    } else {
      choice = {
        kind: "audio",
        format: audioFormat,
        bitrate: audioFormat === "mp3" ? bitrate : null,
        audioFormatId: info.audio?.formatId ?? null,
      }
    }
    setBusy(true)
    try {
      await api.enqueueDownload(analyzedUrl, choice, dest)
      toast.success("Download adicionado à fila", {
        description: info.title,
        action: { label: "Ver fila", onClick: () => go("queue") },
      })
    } catch (e) {
      toast.error((e as { message?: string }).message ?? "Não foi possível iniciar o download.")
    } finally {
      setBusy(false)
    }
  }

  return (
    <section aria-label="Prévia do conteúdo" className="rounded-3xl bg-card p-5 shadow-raised">
      <div className="flex gap-5">
        <Thumb remote={info.thumbnail} className="w-[300px] rounded-2xl" audio={!hasVideo} />
        <div className="flex min-w-0 flex-1 flex-col">
          <h2 className="selectable line-clamp-2 text-[19px] font-semibold leading-tight tracking-[-0.015em]">{info.title}</h2>
          <div className="mt-3 flex flex-wrap gap-2 text-[12.5px] text-muted-foreground">
            {info.uploader && (
              <span className="inline-flex items-center gap-1.5">
                <UserRound className="size-3.5" strokeWidth={1.75} />
                {info.uploader}
              </span>
            )}
            {info.duration != null && (
              <span className="tnum inline-flex items-center gap-1.5">
                <Clock3 className="size-3.5" strokeWidth={1.75} />
                {formatDuration(info.duration)}
              </span>
            )}
            <span className="inline-flex items-center gap-1.5">
              <Globe className="size-3.5" strokeWidth={1.75} />
              {info.sourceLabel}
            </span>
          </div>

          <div className="mt-auto pt-4">
            <ChoiceGroup
              label="Tipo de download"
              value={mode}
              onChange={setMode}
              options={[
                { value: "video", label: "Vídeo", disabled: !hasVideo },
                { value: "audio", label: "Somente áudio", disabled: !hasAudio },
              ]}
            />
          </div>
        </div>
      </div>

      <div className="mt-6 flex flex-col gap-5">
        {mode === "video" && quality ? (
          <>
            <div>
              <h3 className="mb-2 text-[13px] font-medium text-muted-foreground">
                Qualidade{" "}
                <span className="text-muted-foreground/70">(somente as resoluções oferecidas pela fonte)</span>
              </h3>
              <ChoiceGroup
                layout="chips"
                label="Qualidade"
                className="grid-cols-[repeat(auto-fill,minmax(112px,1fr))]"
                value={String(quality.height)}
                onChange={(v) => setHeight(Number(v))}
                options={info.qualities.map((q) => {
                  const opt = q.containers.find((c) => c.container === containerPref) ?? q.containers[0]
                  return {
                    value: String(q.height),
                    label: q.label,
                    hint: opt?.sizeApprox ? `≈ ${formatBytes(opt.sizeApprox)}` : "Tamanho variável",
                  }
                })}
              />
            </div>
            <div>
              <h3 className="mb-2 text-[13px] font-medium text-muted-foreground">Formato</h3>
              <div className="flex flex-wrap items-center gap-4">
                <ChoiceGroup
                  label="Formato do arquivo"
                  value={option?.container ?? "mp4"}
                  onChange={setContainerPref}
                  options={(["mp4", "webm", "mkv"] as Container[])
                    .filter((c) => available.some((a) => a.container === c))
                    .map((c) => ({ value: c, label: CONTAINER_LABEL[c] }))}
                />
                {option && <p className="text-[12.5px] text-muted-foreground">{containerHint(option.container, option.vcodec)}</p>}
              </div>
            </div>
          </>
        ) : (
          <div>
            <h3 className="mb-2 text-[13px] font-medium text-muted-foreground">Formato do áudio</h3>
            <div className="flex flex-wrap items-center gap-4">
              <ChoiceGroup
                label="Formato do áudio"
                value={audioFormat}
                onChange={setAudioFormat}
                options={[
                  { value: "mp3", label: "MP3" },
                  { value: "m4a", label: "M4A" },
                ]}
              />
              {audioFormat === "mp3" ? (
                <Select value={String(bitrate)} onValueChange={(v) => setBitrate(Number(v))}>
                  <SelectTrigger className="h-10 w-[140px]" aria-label="Qualidade do MP3">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {MP3_BITRATES.map((b) => (
                      <SelectItem key={b} value={String(b)}>
                        {b} kbps
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              ) : (
                <p className="text-[12.5px] text-muted-foreground">AAC sem recompressão quando a fonte já oferece esse formato.</p>
              )}
            </div>
          </div>
        )}

        <div className="flex flex-wrap items-end gap-4 border-t pt-5">
          <div className="min-w-[280px] flex-1">
            <h3 className="mb-2 text-[13px] font-medium text-muted-foreground">Salvar em</h3>
            <DestinationField value={dest} onChange={setDest} />
          </div>
          <div className="flex items-center gap-4">
            <div className="text-right">
              <p className="text-[12px] text-muted-foreground">Tamanho estimado</p>
              <p className="tnum text-[15px] font-semibold">{estimate ? `≈ ${formatBytes(estimate)}` : "Desconhecido"}</p>
            </div>
            <Button
              onClick={submit}
              disabled={busy || (mode === "video" && !option)}
              className="press h-12 gap-2 rounded-2xl px-7 text-[15px] font-semibold"
            >
              {busy ? <Loader2 className="size-[18px] animate-spin" /> : <ArrowDownToLine className="size-[18px]" strokeWidth={2} />}
              Baixar
            </Button>
          </div>
        </div>
      </div>
    </section>
  )
}
