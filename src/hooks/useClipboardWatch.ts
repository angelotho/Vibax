import { readText } from "@tauri-apps/plugin-clipboard-manager"
import { useEffect, useRef } from "react"

import { looksLikeUrl } from "@/lib/url"
import { useSettings } from "@/store/settings"
import { useUi } from "@/store/ui"

/**
 * Quando a janela recebe foco, verifica se há um link novo na área de
 * transferência e o oferece como sugestão. Nada é lido em segundo plano e
 * nenhum conteúdo sai do computador.
 */
export function useClipboardWatch() {
  const enabled = useSettings((s) => s.settings?.clipboardWatch ?? false)
  const lastSeen = useRef<string | null>(null)

  useEffect(() => {
    if (!enabled) {
      useUi.getState().setClipboardUrl(null)
      return
    }
    const check = async () => {
      try {
        const text = (await readText())?.trim() ?? ""
        if (!text || text === lastSeen.current) return
        lastSeen.current = text
        if (looksLikeUrl(text)) useUi.getState().setClipboardUrl(text)
      } catch {
        // Área de transferência vazia ou com conteúdo não textual.
      }
    }
    check()
    window.addEventListener("focus", check)
    return () => window.removeEventListener("focus", check)
  }, [enabled])
}
