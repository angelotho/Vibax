import type { LucideIcon } from "lucide-react"

export function EmptyState({
  icon: Icon,
  title,
  children,
  action,
}: {
  icon: LucideIcon
  title: string
  children?: React.ReactNode
  action?: React.ReactNode
}) {
  return (
    <div className="flex flex-col items-center justify-center px-6 py-16 text-center">
      <div className="grid size-12 place-items-center rounded-2xl bg-violet-soft text-primary">
        <Icon className="size-5" strokeWidth={1.75} />
      </div>
      <h2 className="mt-4 text-[15px] font-semibold">{title}</h2>
      {children && <p className="mt-1 max-w-sm text-[13px] text-muted-foreground">{children}</p>}
      {action && <div className="mt-5">{action}</div>}
    </div>
  )
}
