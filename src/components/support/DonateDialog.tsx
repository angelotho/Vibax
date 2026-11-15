import { writeText } from "@tauri-apps/plugin-clipboard-manager"
import { Check, Copy, Heart } from "lucide-react"
import { useMemo, useState } from "react"
import { toast } from "sonner"
import { renderSVG } from "uqr"

import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { PIX_KEY, pixPayload } from "@/lib/pix"

export function DonateDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  const payload = useMemo(() => pixPayload(), [])
  const svg = useMemo(() => renderSVG(payload, { border: 2, ecc: "M" }), [payload])
  const [copied, setCopied] = useState<"payload" | "key" | null>(null)

  const copy = async (what: "payload" | "key") => {
    try {
      await writeText(what === "payload" ? payload : PIX_KEY)
      setCopied(what)
      toast.success(what === "payload" ? "Pix copia e cola copiado" : "Chave Pix copiada")
      setTimeout(() => setCopied(null), 2000)
    } catch {
      toast.error("Não foi possível copiar.")
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-sm">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Heart className="size-4 fill-primary text-primary" /> Apoie o Vibax
          </DialogTitle>
          <DialogDescription>
            O Vibax é gratuito e sempre será. Se ele te ajudou, considere uma doação via Pix de qualquer valor.
          </DialogDescription>
        </DialogHeader>

        <div
          role="img"
          aria-label="QR Code Pix para doação"
          className="mx-auto size-52 overflow-hidden rounded-lg bg-white p-1 [&_svg]:size-full"
          dangerouslySetInnerHTML={{ __html: svg }}
        />

        <div className="flex flex-col gap-2">
          <Button onClick={() => copy("payload")}>
            {copied === "payload" ? <Check /> : <Copy />} Copiar Pix copia e cola
          </Button>
          <button
            type="button"
            onClick={() => copy("key")}
            title="Copiar chave aleatória"
            className="tnum flex items-center justify-center gap-1.5 rounded-md px-2 py-1 text-[11.5px] text-muted-foreground transition-colors hover:text-foreground"
          >
            {copied === "key" ? <Check className="size-3" /> : <Copy className="size-3" />}
            {PIX_KEY}
          </button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
