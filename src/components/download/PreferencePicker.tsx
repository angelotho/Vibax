import { ChoiceGroup } from "@/components/common/ChoiceGroup"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import type { AudioFormat, Container, Preference } from "@/lib/types"

export const QUALITY_PREFS: { value: number; label: string }[] = [
  { value: 0, label: "Melhor disponível" },
  { value: 2160, label: "Até 4K" },
  { value: 1440, label: "Até 1440p" },
  { value: 1080, label: "Até 1080p" },
  { value: 720, label: "Até 720p" },
  { value: 480, label: "Até 480p" },
  { value: 360, label: "Até 360p" },
]

export const MP3_BITRATES = [128, 192, 256, 320]

/**
 * Preferência usada quando o conteúdo ainda não foi analisado (vários links,
 * playlists): cada item recebe a maior qualidade disponível dentro do limite.
 */
export function PreferencePicker({ value, onChange }: { value: Preference; onChange: (p: Preference) => void }) {
  return (
    <div className="flex flex-col gap-4">
      <ChoiceGroup
        label="Tipo de download"
        value={value.kind}
        onChange={(k) =>
          onChange(k === "video" ? { kind: "video", maxHeight: 1080, container: "mp4" } : { kind: "audio", format: "mp3", bitrate: 192 })
        }
        options={[
          { value: "video", label: "Vídeo" },
          { value: "audio", label: "Somente áudio" },
        ]}
      />
      {value.kind === "video" ? (
        <div className="grid grid-cols-2 gap-3">
          <Field label="Qualidade máxima">
            <Select
              value={String(value.maxHeight >= 100000 ? 0 : value.maxHeight)}
              onValueChange={(v) => onChange({ ...value, maxHeight: Number(v) === 0 ? 4_294_967_295 : Number(v) })}
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {QUALITY_PREFS.map((q) => (
                  <SelectItem key={q.value} value={String(q.value)}>
                    {q.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
          <Field label="Formato">
            <Select value={value.container} onValueChange={(v) => onChange({ ...value, container: v as Container })}>
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="mp4">MP4 (recomendado)</SelectItem>
                <SelectItem value="webm">WebM</SelectItem>
                <SelectItem value="mkv">MKV</SelectItem>
              </SelectContent>
            </Select>
          </Field>
        </div>
      ) : (
        <div className="grid grid-cols-2 gap-3">
          <Field label="Formato">
            <Select
              value={value.format}
              onValueChange={(v) => onChange({ ...value, format: v as AudioFormat, bitrate: v === "mp3" ? (value.bitrate ?? 192) : null })}
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="mp3">MP3</SelectItem>
                <SelectItem value="m4a">M4A (AAC)</SelectItem>
              </SelectContent>
            </Select>
          </Field>
          {value.format === "mp3" && (
            <Field label="Qualidade do MP3">
              <Select value={String(value.bitrate ?? 192)} onValueChange={(v) => onChange({ ...value, bitrate: Number(v) })}>
                <SelectTrigger className="w-full">
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
            </Field>
          )}
        </div>
      )}
    </div>
  )
}

export function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1.5">
      <span className="text-[12.5px] font-medium text-muted-foreground">{label}</span>
      {children}
    </label>
  )
}
