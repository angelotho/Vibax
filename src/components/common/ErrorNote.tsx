import { ChevronDown, CircleAlert, Lock, ShieldAlert, WifiOff } from "lucide-react"
import { useState } from "react"

import type { AppError } from "@/lib/types"
import { cn } from "@/lib/utils"

function iconFor(code: AppError["code"]) {
  switch (code) {
    case "network":
      return WifiOff
    case "auth_required":
    case "drm":
      return Lock
    case "permission":
    case "disk_full":
      return ShieldAlert
    default:
      return CircleAlert
  }
}

/** Mensagem de erro clara, com detalhes técnicos sob demanda. */
export function ErrorNote({
  error,
  action,
  compact,
  className,
}: {
  error: AppError
  action?: React.ReactNode
  compact?: boolean
  className?: string
}) {
  const [open, setOpen] = useState(false)
  const Icon = iconFor(error.code)
  return (
    <div role="alert" className={cn("rounded-xl bg-destructive/8 text-[13px]", compact ? "px-3 py-2" : "px-4 py-3", className)}>
      <div className="flex items-start gap-2.5">
        <Icon className="mt-0.5 size-4 shrink-0 text-destructive" strokeWidth={2} />
        <p className="flex-1 leading-relaxed text-foreground/90">{error.message}</p>
        {action}
      </div>
      {error.detail && (
        <div className="pl-[26px]">
          <button
            type="button"
            onClick={() => setOpen((v) => !v)}
            aria-expanded={open}
            className="mt-1 inline-flex items-center gap-1 text-[12px] text-muted-foreground hover:text-foreground"
          >
            Detalhes técnicos
            <ChevronDown className={cn("size-3 transition-transform duration-150", open && "rotate-180")} />
          </button>
          {open && (
            <p className="selectable mt-1.5 break-all rounded-lg bg-background/60 px-2.5 py-2 font-mono text-[11.5px] text-muted-foreground">
              {error.detail}
            </p>
          )}
        </div>
      )}
    </div>
  )
}
