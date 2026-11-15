import { Heart, X } from "lucide-react"

const KEY = "vibax.supportCard.dismissedAt"
const SNOOZE_MS = 14 * 24 * 60 * 60 * 1000

/** O card volta a aparecer 14 dias depois de fechado. */
export function isSupportCardSnoozed(): boolean {
  try {
    const at = Number(localStorage.getItem(KEY))
    return Number.isFinite(at) && at > 0 && Date.now() - at < SNOOZE_MS
  } catch {
    return false
  }
}

export function snoozeSupportCard() {
  try {
    localStorage.setItem(KEY, String(Date.now()))
  } catch {
    /* sem storage: o card só some até reabrir o app */
  }
}

export function SupportCard({ onDonate, onDismiss }: { onDonate: () => void; onDismiss: () => void }) {
  return (
    <div className="relative overflow-hidden rounded-[18px] bg-gradient-to-br from-primary to-fuchsia-600 p-3 text-primary-foreground shadow-raised animate-in fade-in-0 slide-in-from-bottom-1 duration-300">
      <div aria-hidden className="pointer-events-none absolute -right-6 -top-6 size-20 rounded-full bg-white/15 blur-xl" />
      <button
        type="button"
        onClick={onDismiss}
        aria-label="Dispensar por enquanto"
        title="Dispensar por enquanto"
        className="absolute right-1.5 top-1.5 grid size-6 place-items-center rounded-full text-white/70 transition-colors duration-150 hover:bg-white/15 hover:text-white"
      >
        <X className="size-3.5" />
      </button>

      <div className="flex items-center gap-2 pr-5 text-[13px] font-semibold">
        <Heart className="size-4 fill-white animate-[heartbeat_1.6s_ease-in-out_infinite] motion-reduce:animate-none" />
        Curtiu o Vibax?
      </div>
      <p className="mt-1 text-pretty text-[12px] leading-snug text-white/85">
        É grátis e sempre será. Um Pix de qualquer valor ajuda a manter o projeto vivo.
      </p>
      <button
        type="button"
        onClick={onDonate}
        className="mt-2.5 h-8 w-full rounded-[6px] bg-white text-[12.5px] font-semibold text-primary transition-[scale,background-color] duration-150 hover:bg-white/90 active:scale-[0.96]"
      >
        Doar via Pix
      </button>
    </div>
  )
}
