import { CircleCheckIcon, InfoIcon, Loader2Icon, OctagonXIcon, TriangleAlertIcon } from "lucide-react"
import { Toaster as Sonner, type ToasterProps } from "sonner"

/** Notificações dentro do app; o tema acompanha a preferência do Vibax. */
const Toaster = ({ theme, ...props }: ToasterProps & { theme: "dark" | "light" }) => (
  <Sonner
    theme={theme}
    position="bottom-right"
    className="toaster group"
    icons={{
      success: <CircleCheckIcon className="size-4 text-success" />,
      info: <InfoIcon className="size-4" />,
      warning: <TriangleAlertIcon className="size-4 text-warning" />,
      error: <OctagonXIcon className="size-4 text-destructive" />,
      loading: <Loader2Icon className="size-4 animate-spin" />,
    }}
    style={
      {
        "--normal-bg": "var(--popover)",
        "--normal-text": "var(--popover-foreground)",
        "--normal-border": "var(--border)",
        "--border-radius": "calc(var(--radius) * 1.4)",
      } as React.CSSProperties
    }
    {...props}
  />
)

export { Toaster }
