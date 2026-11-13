import { create } from "zustand"

import { api } from "@/lib/api"
import type { Settings } from "@/lib/types"

interface SettingsState {
  settings: Settings | null
  load: () => Promise<void>
  /** Salva alterações parciais; em caso de falha, restaura o valor anterior. */
  update: (patch: Partial<Settings>) => Promise<void>
}

export const useSettings = create<SettingsState>((set, get) => ({
  settings: null,
  load: async () => set({ settings: await api.getSettings() }),
  update: async (patch) => {
    const prev = get().settings
    if (!prev) return
    const next = { ...prev, ...patch }
    set({ settings: next })
    try {
      set({ settings: await api.saveSettings(next) })
    } catch (e) {
      set({ settings: prev })
      throw e
    }
  },
}))
