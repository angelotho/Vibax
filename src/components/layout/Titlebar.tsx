import { getCurrentWindow } from "@tauri-apps/api/window"
import { Copy, Minus, Square, X } from "lucide-react"
import { useEffect, useState } from "react"

import { cn } from "@/lib/utils"

import { LogoMark } from "./Logo"

const win = getCurrentWindow()

function WinButton({
  label,
  onClick,
  danger,
  children,
}: {
  label: string
  onClick: () => void
  danger?: boolean
  children: React.ReactNode
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className={cn(
        "grid h-full w-12 place-items-center text-muted-foreground transition-colors duration-150",
        "hover:bg-foreground/8 hover:text-foreground focus-visible:bg-foreground/8 focus-visible:outline-none",
        danger && "hover:bg-[#e81123] hover:text-white focus-visible:bg-[#e81123] focus-visible:text-white",
      )}
    >
      {children}
    </button>
  )
}

/** Barra de título própria (a janela não usa a moldura padrão do Windows). */
export function Titlebar() {
  const [maximized, setMaximized] = useState(false)

  useEffect(() => {
    win.isMaximized().then(setMaximized)
    const un = win.onResized(() => win.isMaximized().then(setMaximized))
    return () => {
      un.then((f) => f())
    }
  }, [])

  return (
    <header
      data-tauri-drag-region
      className="relative z-50 flex h-10 shrink-0 items-center justify-between bg-rail"
      onDoubleClick={(e) => {
        if (e.target === e.currentTarget) win.toggleMaximize()
      }}
    >
      <div data-tauri-drag-region className="pointer-events-none flex items-center gap-2 pl-4">
        <LogoMark className="size-[18px]" />
        <span className="text-[13px] font-semibold tracking-[-0.01em]">Vibax</span>
      </div>
      <div className="flex h-full">
        <WinButton label="Minimizar" onClick={() => win.minimize()}>
          <Minus className="size-4" strokeWidth={1.5} />
        </WinButton>
        <WinButton label={maximized ? "Restaurar" : "Maximizar"} onClick={() => win.toggleMaximize()}>
          {maximized ? (
            <Copy className="size-3.5 -scale-x-100" strokeWidth={1.5} />
          ) : (
            <Square className="size-3.5" strokeWidth={1.5} />
          )}
        </WinButton>
        <WinButton label="Fechar" danger onClick={() => win.close()}>
          <X className="size-4" strokeWidth={1.5} />
        </WinButton>
      </div>
    </header>
  )
}
