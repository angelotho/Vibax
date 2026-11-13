import { ListVideo, Loader2 } from "lucide-react"
import { useState } from "react"
import { toast } from "sonner"

import { Thumb } from "@/components/common/Thumb"
import { Button } from "@/components/ui/button"
import { api } from "@/lib/api"
import { formatDuration } from "@/lib/format"
import type { MediaInfo, Preference, Settings } from "@/lib/types"
import { useUi } from "@/store/ui"

import { DestinationField } from "./DestinationField"
import { PreferencePicker } from "./PreferencePicker"

export function preferenceFromSettings(s: Settings): Preference {
  return s.defaultKind === "audio"
    ? { kind: "audio", format: s.defaultAudioFormat, bitrate: s.defaultAudioFormat === "mp3" ? s.mp3Bitrate : null }
    : { kind: "video", maxHeight: s.defaultQuality === 0 ? 4_294_967_295 : s.defaultQuality, container: s.defaultContainer }
}

/** Playlist: lista os itens e adiciona todos à fila com a mesma preferência. */
export function PlaylistPreview({ info, settings }: { info: MediaInfo; settings: Settings }) {
  const go = useUi((s) => s.go)
  const [pref, setPref] = useState<Preference>(() => preferenceFromSettings(settings))
  const [dest, setDest] = useState(settings.downloadDir)
  const [busy, setBusy] = useState(false)
  const count = info.entries.length

  const addAll = async () => {
    setBusy(true)
    try {
      const jobs = await api.enqueueMany(
        info.entries.map((e) => e.url),
        pref,
        dest,
      )
      toast.success(`${jobs.length} vídeos adicionados à fila`, { action: { label: "Ver fila", onClick: () => go("queue") } })
    } catch (e) {
      toast.error((e as { message?: string }).message ?? "Não foi possível adicionar a playlist.")
    } finally {
      setBusy(false)
    }
  }

  return (
    <section aria-label="Prévia da playlist" className="rounded-3xl bg-card p-5 shadow-raised">
      <div className="flex gap-5">
        <Thumb remote={info.thumbnail} className="w-[220px] rounded-2xl" />
        <div className="min-w-0 flex-1">
          <p className="inline-flex items-center gap-1.5 text-[12.5px] font-medium text-primary">
            <ListVideo className="size-3.5" /> Playlist com {count} {count === 1 ? "vídeo" : "vídeos"}
          </p>
          <h2 className="selectable mt-1 line-clamp-2 text-[19px] font-semibold leading-tight tracking-[-0.015em]">{info.title}</h2>
          {info.uploader && <p className="mt-1 text-[13px] text-muted-foreground">{info.uploader}</p>}
          <p className="mt-3 text-[12.5px] text-muted-foreground">
            Cada vídeo é analisado na fila e baixado na maior qualidade disponível dentro do limite escolhido.
          </p>
        </div>
      </div>

      <ol className="mt-5 max-h-[188px] overflow-y-auto rounded-xl bg-muted/60 py-1">
        {info.entries.map((e, i) => (
          <li key={e.url} className="flex items-center gap-3 px-3.5 py-1.5 text-[13px]">
            <span className="tnum w-6 text-right text-muted-foreground/70">{i + 1}</span>
            <span className="min-w-0 flex-1 truncate">{e.title ?? e.url}</span>
            {e.duration != null && <span className="tnum text-muted-foreground">{formatDuration(e.duration)}</span>}
          </li>
        ))}
      </ol>

      <div className="mt-5 grid grid-cols-[1fr_1fr] gap-6">
        <PreferencePicker value={pref} onChange={setPref} />
        <div className="flex flex-col justify-between gap-4">
          <DestinationField value={dest} onChange={setDest} />
          <Button onClick={addAll} disabled={busy} className="press h-12 gap-2 self-end rounded-2xl px-6 text-[15px] font-semibold">
            {busy && <Loader2 className="size-4 animate-spin" />}
            Adicionar {count} à fila
          </Button>
        </div>
      </div>
    </section>
  )
}
