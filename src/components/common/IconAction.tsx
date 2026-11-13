import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { cn } from "@/lib/utils"

/** Botão de ícone com dica e área de clique ampliada (40×40). */
export function IconAction({
  label,
  onClick,
  children,
  tone = "default",
  disabled,
}: {
  label: string
  onClick: () => void
  children: React.ReactNode
  tone?: "default" | "danger" | "primary"
  disabled?: boolean
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          aria-label={label}
          onClick={onClick}
          disabled={disabled}
          className={cn(
            "press relative grid size-8 place-items-center rounded-lg text-muted-foreground",
            "after:absolute after:-inset-1 hover:bg-foreground/7 hover:text-foreground",
            "focus-visible:outline-2 focus-visible:outline-ring disabled:opacity-40",
            tone === "danger" && "hover:bg-destructive/12 hover:text-destructive",
            tone === "primary" && "text-primary hover:bg-violet-soft hover:text-primary",
          )}
        >
          {children}
        </button>
      </TooltipTrigger>
      <TooltipContent side="top">{label}</TooltipContent>
    </Tooltip>
  )
}
