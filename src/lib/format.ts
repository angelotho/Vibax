// Formatação para exibição (pt-BR).

const UNITS = ["B", "KB", "MB", "GB", "TB"]

export function formatBytes(bytes: number | null | undefined, digits = 1): string {
  if (bytes == null || !Number.isFinite(bytes) || bytes < 0) return "—"
  if (bytes < 1024) return `${Math.round(bytes)} B`
  let v = bytes
  let i = 0
  while (v >= 1024 && i < UNITS.length - 1) {
    v /= 1024
    i++
  }
  return `${v.toLocaleString("pt-BR", { maximumFractionDigits: v >= 100 ? 0 : digits, minimumFractionDigits: 0 })} ${UNITS[i]}`
}

export function formatSpeed(bps: number | null | undefined): string {
  if (bps == null || !Number.isFinite(bps) || bps <= 0) return "—"
  return `${formatBytes(bps)}/s`
}

/** 75 → "1:15"; 3725 → "1:02:05". */
export function formatDuration(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) return "—"
  const s = Math.round(seconds)
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  const sec = s % 60
  const pad = (n: number) => n.toString().padStart(2, "0")
  return h > 0 ? `${h}:${pad(m)}:${pad(sec)}` : `${m}:${pad(sec)}`
}

/** Tempo restante legível: "8 s", "3 min", "1 h 05 min". */
export function formatEta(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) return "—"
  if (seconds < 60) return `${Math.max(1, Math.round(seconds))} s`
  const m = Math.round(seconds / 60)
  if (m < 60) return `${m} min`
  const h = Math.floor(m / 60)
  return `${h} h ${(m % 60).toString().padStart(2, "0")} min`
}

export function formatPercent(fraction: number | null | undefined): string {
  if (fraction == null || !Number.isFinite(fraction)) return "—"
  const p = Math.min(100, Math.max(0, fraction * 100))
  return `${p < 10 && p > 0 ? p.toFixed(1).replace(".", ",") : Math.floor(p)}%`
}

export function formatDate(ms: number | null | undefined, now = Date.now()): string {
  if (!ms) return "—"
  const d = new Date(ms)
  const today = new Date(now)
  const sameDay = d.toDateString() === today.toDateString()
  const yesterday = new Date(now - 86_400_000).toDateString() === d.toDateString()
  const time = d.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" })
  if (sameDay) return `Hoje, ${time}`
  if (yesterday) return `Ontem, ${time}`
  return d.toLocaleDateString("pt-BR", { day: "2-digit", month: "short", year: "numeric" })
}

/** Último segmento de um caminho do Windows. */
export function baseName(path: string | null | undefined): string {
  if (!path) return ""
  const parts = path.split(/[\\/]/).filter(Boolean)
  return parts[parts.length - 1] ?? path
}

/** Caminho encurtado para caber na interface: "D:\…\Vídeos\Vibax". */
export function shortPath(path: string, keep = 2): string {
  const parts = path.split(/[\\/]/).filter(Boolean)
  if (parts.length <= keep + 1) return path
  return `${parts[0]}\\…\\${parts.slice(-keep).join("\\")}`
}
