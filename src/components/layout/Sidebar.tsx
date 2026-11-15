import { ArrowDownToLine, Heart, History, Layers, Settings2 } from "lucide-react"
import { useState } from "react"

import { DonateDialog } from "@/components/support/DonateDialog"
import { isSupportCardSnoozed, snoozeSupportCard, SupportCard } from "@/components/support/SupportCard"

import { formatSpeed } from "@/lib/format"
import { cn } from "@/lib/utils"
import { useQueueSummary } from "@/store/downloads"
import { type Page, useUi } from "@/store/ui"

const ITEMS: { page: Page; label: string; icon: typeof ArrowDownToLine; shortcut: string }[] = [
  { page: "download", label: "Baixar", icon: ArrowDownToLine, shortcut: "Ctrl+1" },
  { page: "queue", label: "Fila", icon: Layers, shortcut: "Ctrl+2" },
  { page: "history", label: "Histórico", icon: History, shortcut: "Ctrl+3" },
  { page: "settings", label: "Configurações", icon: Settings2, shortcut: "Ctrl+," },
]

export function Sidebar({ version }: { version?: string }) {
  const page = useUi((s) => s.page)
  const go = useUi((s) => s.go)
  const { active, pending, speed } = useQueueSummary()
  const [donateOpen, setDonateOpen] = useState(false)
  const [cardVisible, setCardVisible] = useState(() => !isSupportCardSnoozed())

  return (
    <nav aria-label="Navegação principal" className="flex w-[212px] shrink-0 flex-col bg-rail px-3 pb-3 pt-2">
      <ul className="flex flex-col gap-0.5">
        {ITEMS.map(({ page: p, label, icon: Icon, shortcut }) => {
          const current = page === p
          return (
            <li key={p}>
              <button
                type="button"
                onClick={() => go(p)}
                aria-current={current ? "page" : undefined}
                title={`${label} (${shortcut})`}
                className={cn(
                  "group relative flex h-10 w-full items-center gap-3 rounded-lg px-3 text-[13.5px] font-medium",
                  "transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-ring",
                  current
                    ? "bg-violet-soft text-foreground"
                    : "text-muted-foreground hover:bg-foreground/5 hover:text-foreground",
                )}
              >
                <span
                  aria-hidden
                  className={cn(
                    "absolute left-0 top-1/2 h-4 w-[3px] -translate-y-1/2 rounded-full bg-primary transition-opacity duration-150",
                    current ? "opacity-100" : "opacity-0",
                  )}
                />
                <Icon
                  className={cn("size-[18px]", current ? "text-primary" : "")}
                  strokeWidth={current ? 2 : 1.75}
                />
                <span className="flex-1 text-left">{label}</span>
                {p === "queue" && pending > 0 && (
                  <span className="tnum min-w-5 rounded-full bg-primary px-1.5 py-px text-center text-[11px] font-semibold text-primary-foreground">
                    {pending}
                  </span>
                )}
              </button>
            </li>
          )
        })}
      </ul>

      <div className="mt-auto flex flex-col gap-3">
        {active > 0 && (
          <button
            type="button"
            onClick={() => go("queue")}
            className="rounded-xl bg-raised p-3 text-left shadow-raised transition-colors hover:bg-accent"
          >
            <div className="flex items-center gap-2 text-[12.5px] font-medium">
              <span className="relative flex size-2">
                <span className="absolute inline-flex size-full animate-ping rounded-full bg-primary opacity-60 motion-reduce:animate-none" />
                <span className="relative inline-flex size-2 rounded-full bg-primary" />
              </span>
              {active === 1 ? "1 download ativo" : `${active} downloads ativos`}
            </div>
            {speed > 0 && <div className="tnum mt-1 pl-4 text-[12px] text-muted-foreground">{formatSpeed(speed)}</div>}
          </button>
        )}
        {cardVisible ? (
          <SupportCard
            onDonate={() => setDonateOpen(true)}
            onDismiss={() => {
              snoozeSupportCard()
              setCardVisible(false)
            }}
          />
        ) : (
        <button
          type="button"
          onClick={() => setDonateOpen(true)}
          className="group flex h-9 w-full items-center gap-3 rounded-lg px-3 text-[13px] font-medium text-muted-foreground transition-colors duration-150 hover:bg-foreground/5 hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-ring"
        >
          <Heart className="size-[16px] transition-colors group-hover:fill-primary group-hover:text-primary" strokeWidth={1.75} />
          Apoie o projeto
        </button>
        )}
        {version && <div className="tnum px-3 text-[11px] text-muted-foreground/70">Vibax {version}</div>}
      </div>
      <DonateDialog open={donateOpen} onOpenChange={setDonateOpen} />
    </nav>
  )
}
