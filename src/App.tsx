import { useEffect, useRef, useState } from "react"

import { BatchDialog } from "@/components/download/BatchDialog"
import { Sidebar } from "@/components/layout/Sidebar"
import { Titlebar } from "@/components/layout/Titlebar"
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog"
import { Toaster } from "@/components/ui/sonner"
import { TooltipProvider } from "@/components/ui/tooltip"
import { useBackendEvents } from "@/hooks/useBackendEvents"
import { useClipboardWatch } from "@/hooks/useClipboardWatch"
import { useApplyTheme } from "@/hooks/useTheme"
import { api, events } from "@/lib/api"
import { DownloadPage } from "@/pages/DownloadPage"
import { HistoryPage } from "@/pages/HistoryPage"
import { QueuePage } from "@/pages/QueuePage"
import { SettingsPage } from "@/pages/SettingsPage"
import { useSettings } from "@/store/settings"
import { type Page, useUi } from "@/store/ui"

const PAGES: Record<Page, () => React.ReactElement | null> = {
  download: DownloadPage,
  queue: QueuePage,
  history: HistoryPage,
  settings: SettingsPage,
}

export default function App() {
  const theme = useApplyTheme()
  const page = useUi((s) => s.page)
  const go = useUi((s) => s.go)
  const [version, setVersion] = useState<string>()
  const [closeAsk, setCloseAsk] = useState<number | null>(null)
  const main = useRef<HTMLElement>(null)

  useBackendEvents()
  useClipboardWatch()

  useEffect(() => {
    useSettings.getState().load()
    api.appInfo().then((i) => setVersion(i.version)).catch(() => {})
    const un = events.onCloseRequested((n) => setCloseAsk(n))
    return () => {
      un.then((f) => f())
    }
  }, [])

  // Atalhos: Ctrl+1/2/3 para as seções e Ctrl+, para configurações.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!e.ctrlKey || e.altKey || e.shiftKey) return
      const map: Record<string, Page> = { "1": "download", "2": "queue", "3": "history", ",": "settings" }
      const target = map[e.key]
      if (target) {
        e.preventDefault()
        go(target)
      }
    }
    window.addEventListener("keydown", onKey)
    return () => window.removeEventListener("keydown", onKey)
  }, [go])

  useEffect(() => {
    main.current?.scrollTo({ top: 0 })
  }, [page])

  const Current = PAGES[page]
  return (
    <TooltipProvider delayDuration={400}>
      <div className="flex h-full flex-col bg-rail">
        <Titlebar />
        <div className="flex min-h-0 flex-1">
          <Sidebar version={version} />
          <main ref={main} className="min-w-0 flex-1 overflow-y-auto rounded-tl-2xl bg-background [scrollbar-gutter:stable]">
            <Current />
          </main>
        </div>
      </div>

      <BatchDialog />
      <Toaster theme={theme} />

      <AlertDialog open={closeAsk != null} onOpenChange={(o) => !o && setCloseAsk(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {closeAsk === 1 ? "Há 1 download em andamento" : `Há ${closeAsk} downloads em andamento`}
            </AlertDialogTitle>
            <AlertDialogDescription>
              Se fechar agora, eles ficam pausados e você pode retomá-los na próxima vez que abrir o Vibax.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Continuar baixando</AlertDialogCancel>
            <AlertDialogAction onClick={() => api.quit()}>Fechar e pausar</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </TooltipProvider>
  )
}
