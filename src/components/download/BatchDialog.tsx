import { Loader2 } from "lucide-react"
import { useEffect, useState } from "react"
import { toast } from "sonner"

import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Textarea } from "@/components/ui/textarea"
import { api } from "@/lib/api"
import type { Preference } from "@/lib/types"
import { useSettings } from "@/store/settings"
import { useUi } from "@/store/ui"

import { DestinationField } from "./DestinationField"
import { preferenceFromSettings } from "./PlaylistPreview"
import { PreferencePicker } from "./PreferencePicker"

/** Vários links de uma vez: cada um entra na fila e é analisado lá. */
export function BatchDialog() {
  const open = useUi((s) => s.batchOpen)
  const setOpen = useUi((s) => s.setBatchOpen)
  const go = useUi((s) => s.go)
  const settings = useSettings((s) => s.settings)
  const [text, setText] = useState("")
  const [parsed, setParsed] = useState<{ valid: string[]; invalid: string[] }>({ valid: [], invalid: [] })
  const [pref, setPref] = useState<Preference | null>(null)
  const [dest, setDest] = useState("")
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    if (open && settings) {
      setPref(preferenceFromSettings(settings))
      setDest(settings.downloadDir)
    }
    if (!open) {
      setText("")
      setParsed({ valid: [], invalid: [] })
    }
  }, [open, settings])

  useEffect(() => {
    const t = setTimeout(() => {
      if (!text.trim()) return setParsed({ valid: [], invalid: [] })
      api.extractUrls(text).then(setParsed).catch(() => {})
    }, 200)
    return () => clearTimeout(t)
  }, [text])

  const submit = async () => {
    if (!pref || parsed.valid.length === 0) return
    setBusy(true)
    try {
      const jobs = await api.enqueueMany(parsed.valid, pref, dest)
      toast.success(jobs.length === 1 ? "1 link adicionado à fila" : `${jobs.length} links adicionados à fila`)
      setOpen(false)
      go("queue")
    } catch (e) {
      toast.error((e as { message?: string }).message ?? "Não foi possível adicionar os links.")
    } finally {
      setBusy(false)
    }
  }

  const n = parsed.valid.length
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent className="sm:max-w-[620px]">
        <DialogHeader>
          <DialogTitle>Adicionar vários links</DialogTitle>
          <DialogDescription>Cole um link por linha. Cada vídeo é analisado na fila antes de baixar.</DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-4">
          <div>
            <Textarea
              value={text}
              onChange={(e) => setText(e.target.value)}
              placeholder={"https://www.youtube.com/watch?v=…\nhttps://vimeo.com/…"}
              spellCheck={false}
              aria-label="Links"
              className="min-h-[148px] resize-none font-mono text-[12.5px]"
              autoFocus
            />
            <p className="tnum mt-1.5 text-[12px] text-muted-foreground" aria-live="polite">
              {n === 0 && parsed.invalid.length === 0
                ? "Nenhum link ainda."
                : `${n} ${n === 1 ? "link válido" : "links válidos"}${parsed.invalid.length ? `, ${parsed.invalid.length} ${parsed.invalid.length === 1 ? "trecho ignorado" : "trechos ignorados"}` : ""}`}
            </p>
          </div>
          {pref && <PreferencePicker value={pref} onChange={setPref} />}
          {dest && <DestinationField value={dest} onChange={setDest} />}
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={() => setOpen(false)}>
            Cancelar
          </Button>
          <Button onClick={submit} disabled={busy || n === 0} className="press gap-2">
            {busy && <Loader2 className="size-4 animate-spin" />}
            {n > 1 ? `Adicionar ${n} à fila` : "Adicionar à fila"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
