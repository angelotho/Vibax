// Checagem rápida no cliente, usada só para sugerir links copiados.
// A validação definitiva é feita no backend (src-tauri/src/url_guard.rs).

export function looksLikeUrl(text: string): boolean {
  const t = text.trim()
  if (!t || t.length > 2048 || /\s/.test(t)) return false
  const withScheme = /^[a-z][a-z0-9+.-]*:\/\//i.test(t) ? t : `https://${t}`
  try {
    const u = new URL(withScheme)
    return (u.protocol === "https:" || u.protocol === "http:") && u.hostname.includes(".") && !u.username
  } catch {
    return false
  }
}

/** Conta quantas linhas/trechos parecem links (para o modo "vários links"). */
export function countUrls(text: string): number {
  return text.split(/[\s,;]+/).filter((t) => looksLikeUrl(t)).length
}
