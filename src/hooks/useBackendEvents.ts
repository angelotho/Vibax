import { useEffect } from "react"

import { events } from "@/lib/api"
import { useDownloads } from "@/store/downloads"

/** Mantém a fila sincronizada com os eventos emitidos pelo backend. */
export function useBackendEvents() {
  useEffect(() => {
    const { upsert, remove, load } = useDownloads.getState()
    const subs = [events.onJobUpdated(upsert), events.onJobRemoved(remove)]
    load().catch(() => {})
    return () => {
      subs.forEach((p) => p.then((un) => un()))
    }
  }, [])
}
