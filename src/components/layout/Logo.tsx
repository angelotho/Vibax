import { cn } from "@/lib/utils"

/** Marca do Vibax: o "V" que aponta para baixo, sobre a bandeja de download. */
export function LogoMark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 1024 1024" className={cn("size-5", className)} aria-hidden>
      <defs>
        <linearGradient id="vbx-v" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#C9A8FF" />
          <stop offset="1" stopColor="#8B4DFF" />
        </linearGradient>
      </defs>
      <rect x="32" y="32" width="960" height="960" rx="224" fill="#160D26" />
      <path
        d="M292 300 L512 676 L732 300"
        fill="none"
        stroke="url(#vbx-v)"
        strokeWidth="128"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path d="M372 812 H652" stroke="#B48CFF" strokeOpacity="0.6" strokeWidth="56" strokeLinecap="round" />
    </svg>
  )
}
