import { ToggleGroup } from "radix-ui"

import { cn } from "@/lib/utils"

export interface Choice<T extends string> {
  value: T
  label: React.ReactNode
  hint?: React.ReactNode
  disabled?: boolean
}

/**
 * Grupo de opções exclusivas (qualidade, formato, tema…), navegável por teclado.
 * `layout="chips"` mostra rótulo + dica em blocos; `layout="segmented"` é compacto.
 */
export function ChoiceGroup<T extends string>({
  value,
  onChange,
  options,
  label,
  layout = "segmented",
  className,
}: {
  value: T
  onChange: (v: T) => void
  options: Choice<T>[]
  label: string
  layout?: "segmented" | "chips"
  className?: string
}) {
  return (
    <ToggleGroup.Root
      type="single"
      value={value}
      onValueChange={(v) => v && onChange(v as T)}
      aria-label={label}
      className={cn(layout === "segmented" ? "inline-flex rounded-xl bg-muted p-1" : "grid gap-2", className)}
    >
      {options.map((o) => (
        <ToggleGroup.Item
          key={o.value}
          value={o.value}
          disabled={o.disabled}
          className={cn(
            "press relative text-left outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-40",
            layout === "segmented"
              ? "h-8 rounded-lg px-3.5 text-[13px] font-medium text-muted-foreground hover:text-foreground data-[state=on]:bg-raised data-[state=on]:text-foreground data-[state=on]:shadow-raised"
              : "flex min-h-14 flex-col justify-center rounded-xl bg-muted px-3.5 py-2 ring-1 ring-inset ring-transparent hover:bg-accent data-[state=on]:bg-violet-soft data-[state=on]:ring-primary",
          )}
        >
          <span className={cn(layout === "chips" && "text-[14px] font-semibold tracking-[-0.01em]")}>{o.label}</span>
          {o.hint != null && layout === "chips" && (
            <span className="tnum mt-0.5 text-[12px] text-muted-foreground">{o.hint}</span>
          )}
        </ToggleGroup.Item>
      ))}
    </ToggleGroup.Root>
  )
}
