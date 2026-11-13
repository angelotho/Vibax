import { readText } from "@tauri-apps/plugin-clipboard-manager"
import { ClipboardPaste, Link2, ListPlus, Loader2, Search } from "lucide-react"
import { forwardRef } from "react"

import { Button } from "@/components/ui/button"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"

export type UrlBarState = "idle" | "busy" | "invalid"

interface Props {
  value: string
  state: UrlBarState
  onChange: (v: string) => void
  /** Envia o texto informado; `fromPaste` permite analisar automaticamente ao colar. */
  onSubmit: (text: string, fromPaste?: boolean) => void
  onOpenBatch: () => void
  describedBy?: string
}

/** Cápsula de endereço: a peça central da tela "Baixar". */
export const UrlBar = forwardRef<HTMLInputElement, Props>(function UrlBar(
  { value, state, onChange, onSubmit, onOpenBatch, describedBy },
  ref,
) {
  const busy = state === "busy"

  const pasteFromClipboard = async () => {
    try {
      const text = (await readText())?.trim()
      if (text) {
        onChange(text)
        onSubmit(text, true)
      }
    } catch {
      // Área de transferência vazia ou sem texto.
    }
  }

  return (
    <form
      className="url-capsule flex h-16 items-center gap-2 pl-5 pr-2"
      data-state={state}
      onSubmit={(e) => {
        e.preventDefault()
        onSubmit(value)
      }}
    >
      <Link2 className="size-5 shrink-0 text-muted-foreground" strokeWidth={1.75} aria-hidden />
      <input
        ref={ref}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onPaste={(e) => {
          const text = e.clipboardData.getData("text").trim()
          if (text && !value.trim()) {
            e.preventDefault()
            onChange(text)
            onSubmit(text, true)
          }
        }}
        placeholder="Cole o link do vídeo, por exemplo https://www.youtube.com/watch?v=…"
        aria-label="Link do vídeo"
        aria-invalid={state === "invalid"}
        aria-describedby={describedBy}
        spellCheck={false}
        autoComplete="off"
        autoFocus
        className="h-full min-w-0 flex-1 bg-transparent text-[15px] outline-none placeholder:text-muted-foreground/70"
      />
      {!value.trim() && (
        <Button type="button" variant="ghost" className="press h-10 gap-2 rounded-xl px-3" onClick={pasteFromClipboard}>
          <ClipboardPaste className="size-4" strokeWidth={1.75} />
          Colar
        </Button>
      )}
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label="Adicionar vários links"
            className="press size-10 rounded-xl"
            onClick={onOpenBatch}
          >
            <ListPlus className="size-[18px]" strokeWidth={1.75} />
          </Button>
        </TooltipTrigger>
        <TooltipContent>Adicionar vários links de uma vez</TooltipContent>
      </Tooltip>
      <Button
        type="submit"
        disabled={busy || !value.trim()}
        className={cn("press h-11 min-w-[124px] gap-2 rounded-[14px] px-5 text-[14px] font-semibold")}
      >
        {busy ? (
          <Loader2 className="size-4 animate-spin motion-reduce:animate-none" />
        ) : (
          <Search className="size-4" strokeWidth={2} />
        )}
        {busy ? "Analisando" : "Analisar"}
      </Button>
    </form>
  )
})
