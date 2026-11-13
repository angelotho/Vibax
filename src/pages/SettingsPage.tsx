import { CircleCheck, Loader2, Minus, Plus, RefreshCw, ShieldCheck, Undo2 } from "lucide-react"
import { useEffect, useState } from "react"
import { toast } from "sonner"

import { ChoiceGroup } from "@/components/common/ChoiceGroup"
import { ErrorNote } from "@/components/common/ErrorNote"
import { DestinationField } from "@/components/download/DestinationField"
import { MP3_BITRATES, QUALITY_PREFS } from "@/components/download/PreferencePicker"
import { act } from "@/components/queue/JobRow"
import { Button } from "@/components/ui/button"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Switch } from "@/components/ui/switch"
import { api } from "@/lib/api"
import type { AppInfo, Container, NotifyMode, Settings, Theme } from "@/lib/types"
import { useSettings } from "@/store/settings"

function Section({ title, description, children }: { title: string; description?: string; children: React.ReactNode }) {
  return (
    <section className="grid grid-cols-[220px_1fr] gap-8 border-t py-7 first-of-type:border-t-0 first-of-type:pt-2">
      <div>
        <h2 className="text-[14.5px] font-semibold">{title}</h2>
        {description && <p className="mt-1 text-[12.5px] leading-relaxed text-muted-foreground">{description}</p>}
      </div>
      <div className="flex flex-col gap-5">{children}</div>
    </section>
  )
}

function Row({ label, hint, children, htmlFor }: { label: string; hint?: string; children: React.ReactNode; htmlFor?: string }) {
  return (
    <div className="flex items-center justify-between gap-6">
      <div className="min-w-0">
        <label htmlFor={htmlFor} className="text-[13.5px] font-medium">
          {label}
        </label>
        {hint && <p className="mt-0.5 text-[12.5px] text-muted-foreground">{hint}</p>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  )
}

export function SettingsPage() {
  const settings = useSettings((s) => s.settings)
  const update = useSettings((s) => s.update)
  const [info, setInfo] = useState<AppInfo | null>(null)
  const [updating, setUpdating] = useState(false)

  const loadInfo = () => api.appInfo().then(setInfo).catch(() => {})
  useEffect(() => {
    loadInfo()
  }, [])

  if (!settings) return null
  const save = (patch: Partial<Settings>) =>
    update(patch)
      .then(() => toast.success("Preferência salva", { duration: 1500 }))
      .catch((e) => toast.error((e as { message?: string }).message ?? "Não foi possível salvar."))

  const checkUpdate = async () => {
    setUpdating(true)
    try {
      const r = await api.updateYtdlp()
      if (r.updated) toast.success(`Mecanismo de download atualizado para ${r.latest}`)
      else toast.info(`Você já tem a versão mais recente (${r.latest}).`)
      await loadInfo()
    } catch (e) {
      toast.error((e as { message?: string }).message ?? "Não foi possível atualizar.")
    } finally {
      setUpdating(false)
    }
  }

  const tools = info?.tools
  return (
    <div className="mx-auto flex w-full max-w-[920px] flex-col px-10 pb-16 pt-10">
      <header className="mb-6">
        <h1 className="text-[28px] font-semibold leading-tight tracking-[-0.025em]">Configurações</h1>
        <p className="mt-1.5 text-[14px] text-muted-foreground">As alterações são salvas automaticamente neste computador.</p>
      </header>

      <Section title="Downloads" description="Valores usados ao abrir uma prévia e ao adicionar vários links de uma vez.">
        <div>
          <p className="mb-2 text-[13.5px] font-medium">Pasta padrão</p>
          <div className="flex items-center gap-2">
            <div className="flex-1">
              <DestinationField value={settings.downloadDir} onChange={(d) => save({ downloadDir: d })} />
            </div>
            <Button variant="ghost" className="press h-[52px] rounded-xl" onClick={() => act(() => api.openFolder(settings.downloadDir))}>
              Abrir pasta
            </Button>
          </div>
        </div>
        <Row label="Tipo padrão">
          <ChoiceGroup
            label="Tipo padrão"
            value={settings.defaultKind}
            onChange={(v) => save({ defaultKind: v })}
            options={[
              { value: "video", label: "Vídeo" },
              { value: "audio", label: "Somente áudio" },
            ]}
          />
        </Row>
        <Row label="Qualidade preferida" hint="Se não existir, o Vibax escolhe a mais próxima abaixo dela.">
          <Select value={String(settings.defaultQuality)} onValueChange={(v) => save({ defaultQuality: Number(v) })}>
            <SelectTrigger className="w-[190px]" aria-label="Qualidade preferida">
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
        </Row>
        <Row label="Formato de vídeo" hint="MP4 é o mais compatível com TVs, celulares e editores.">
          <ChoiceGroup
            label="Formato de vídeo"
            value={settings.defaultContainer}
            onChange={(v: Container) => save({ defaultContainer: v })}
            options={[
              { value: "mp4", label: "MP4" },
              { value: "webm", label: "WebM" },
              { value: "mkv", label: "MKV" },
            ]}
          />
        </Row>
        <Row label="Formato de áudio">
          <div className="flex items-center gap-2">
            <ChoiceGroup
              label="Formato de áudio"
              value={settings.defaultAudioFormat}
              onChange={(v) => save({ defaultAudioFormat: v })}
              options={[
                { value: "mp3", label: "MP3" },
                { value: "m4a", label: "M4A" },
              ]}
            />
            {settings.defaultAudioFormat === "mp3" && (
              <Select value={String(settings.mp3Bitrate)} onValueChange={(v) => save({ mp3Bitrate: Number(v) })}>
                <SelectTrigger className="w-[120px]" aria-label="Qualidade do MP3">
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
            )}
          </div>
        </Row>
        <Row label="Downloads simultâneos" hint="Os demais aguardam na fila e começam assim que uma vaga abrir.">
          <div className="flex items-center gap-1 rounded-xl bg-muted p-1">
            <Button
              variant="ghost"
              size="icon"
              aria-label="Diminuir"
              className="press size-8 rounded-lg"
              disabled={settings.maxConcurrent <= 1}
              onClick={() => save({ maxConcurrent: settings.maxConcurrent - 1 })}
            >
              <Minus className="size-4" />
            </Button>
            <span className="tnum w-8 text-center text-[14px] font-semibold" aria-live="polite">
              {settings.maxConcurrent}
            </span>
            <Button
              variant="ghost"
              size="icon"
              aria-label="Aumentar"
              className="press size-8 rounded-lg"
              disabled={settings.maxConcurrent >= 6}
              onClick={() => save({ maxConcurrent: settings.maxConcurrent + 1 })}
            >
              <Plus className="size-4" />
            </Button>
          </div>
        </Row>
        <Row label="Gravar título e capa no arquivo" hint="Inclui metadados e miniatura quando o formato permite." htmlFor="embed">
          <Switch id="embed" checked={settings.embedMetadata} onCheckedChange={(v) => save({ embedMetadata: v })} />
        </Row>
      </Section>

      <Section title="Aparência">
        <Row label="Tema">
          <ChoiceGroup
            label="Tema"
            value={settings.theme}
            onChange={(v: Theme) => save({ theme: v })}
            options={[
              { value: "dark", label: "Escuro" },
              { value: "light", label: "Claro" },
              { value: "system", label: "Igual ao Windows" },
            ]}
          />
        </Row>
      </Section>

      <Section title="Notificações e links">
        <Row label="Avisar ao concluir ou falhar">
          <Select value={settings.notifications} onValueChange={(v) => save({ notifications: v as NotifyMode })}>
            <SelectTrigger className="w-[300px]" aria-label="Notificações">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="background">Só com o Vibax em segundo plano</SelectItem>
              <SelectItem value="always">Sempre</SelectItem>
              <SelectItem value="never">Nunca</SelectItem>
            </SelectContent>
          </Select>
        </Row>
        <Row
          label="Detectar links copiados"
          hint="Ao voltar para o Vibax, sugere o link que estiver na área de transferência."
          htmlFor="clip"
        >
          <Switch id="clip" checked={settings.clipboardWatch} onCheckedChange={(v) => save({ clipboardWatch: v })} />
        </Row>
      </Section>

      <Section
        title="Componentes"
        description="Ferramentas de código aberto que acompanham o Vibax. Nada precisa ser instalado à parte."
      >
        {info?.toolsError && <ErrorNote error={info.toolsError} />}
        <dl className="grid grid-cols-[140px_1fr] gap-x-4 gap-y-2.5 text-[13px]">
          <dt className="text-muted-foreground">yt-dlp</dt>
          <dd className="tnum flex items-center gap-2">
            {tools?.ytdlpVersion ?? (info ? "Indisponível" : "Verificando…")}
            {tools?.ytdlpUpdated && (
              <span className="inline-flex items-center gap-1 rounded-md bg-violet-soft px-1.5 py-px text-[11.5px] font-medium text-primary">
                <CircleCheck className="size-3" /> atualizado pelo Vibax
              </span>
            )}
          </dd>
          <dt className="text-muted-foreground">FFmpeg</dt>
          <dd className="selectable truncate">{tools?.ffmpegVersion ?? "—"}</dd>
          <dt className="text-muted-foreground">Deno</dt>
          <dd className="tnum">{tools?.denoVersion ?? "—"}</dd>
        </dl>
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" className="press gap-2 rounded-xl" disabled={updating} onClick={checkUpdate}>
            {updating ? <Loader2 className="size-4 animate-spin" /> : <RefreshCw className="size-4" strokeWidth={1.75} />}
            {updating ? "Verificando…" : "Atualizar o yt-dlp"}
          </Button>
          {tools?.ytdlpUpdated && (
            <Button
              variant="ghost"
              className="press gap-2 rounded-xl"
              onClick={() =>
                act(async () => {
                  await api.resetYtdlp()
                  await loadInfo()
                  toast.success("Voltou a usar a versão que acompanha o Vibax")
                })
              }
            >
              <Undo2 className="size-4" strokeWidth={1.75} /> Usar a versão original
            </Button>
          )}
        </div>
        <p className="text-[12.5px] leading-relaxed text-muted-foreground">
          A atualização é baixada somente da página oficial do yt-dlp no GitHub e só é instalada se o checksum SHA-256 conferir.
          Licenças:{" "}
          <button type="button" className="text-foreground/85 underline decoration-foreground/30 underline-offset-2 hover:text-primary hover:decoration-primary" onClick={() => act(() => api.openKnownLink("ytdlp"))}>
            yt-dlp (Unlicense)
          </button>
          ,{" "}
          <button type="button" className="text-foreground/85 underline decoration-foreground/30 underline-offset-2 hover:text-primary hover:decoration-primary" onClick={() => act(() => api.openKnownLink("ffmpeg"))}>
            FFmpeg (GPLv3)
          </button>{" "}
          com{" "}
          <button
            type="button"
            className="text-foreground/85 underline decoration-foreground/30 underline-offset-2 hover:text-primary hover:decoration-primary"
            onClick={() => act(() => api.openKnownLink("ffmpeg-source"))}
          >
            código-fonte
          </button>
          ,{" "}
          <button type="button" className="text-foreground/85 underline decoration-foreground/30 underline-offset-2 hover:text-primary hover:decoration-primary" onClick={() => act(() => api.openKnownLink("deno"))}>
            Deno (MIT)
          </button>
          .
        </p>
      </Section>

      <Section title="Privacidade e uso">
        <div className="flex gap-3 rounded-xl bg-muted px-4 py-3.5">
          <ShieldCheck className="mt-0.5 size-4 shrink-0 text-success" strokeWidth={2} />
          <div className="text-[13px] leading-relaxed">
            <p>Sem contas, anúncios ou rastreadores. Fila, histórico e preferências ficam apenas neste computador.</p>
            <p className="mt-2 text-muted-foreground">
              O Vibax não usa cookies nem credenciais e não contorna DRM, login, paywalls ou outras restrições. Baixe somente
              conteúdos próprios, licenciados ou cujo download seja permitido pela fonte.
            </p>
          </div>
        </div>
        {info && (
          <p className="tnum text-[12px] text-muted-foreground">
            Vibax {info.version}. Dados locais em <span className="selectable">{info.dataDir}</span>
          </p>
        )}
      </Section>
    </div>
  )
}
