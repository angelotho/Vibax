import { create } from "zustand"

export type Page = "download" | "queue" | "history" | "settings"

interface UiState {
  page: Page
  /** Link sugerido a partir da área de transferência. */
  clipboardUrl: string | null
  /** Link a ser carregado no campo principal (ex.: "Usar" na sugestão). */
  pendingUrl: string | null
  batchOpen: boolean
  go: (page: Page) => void
  setClipboardUrl: (url: string | null) => void
  setPendingUrl: (url: string | null) => void
  setBatchOpen: (open: boolean) => void
}

export const useUi = create<UiState>((set) => ({
  page: "download",
  clipboardUrl: null,
  pendingUrl: null,
  batchOpen: false,
  go: (page) => set({ page }),
  setClipboardUrl: (clipboardUrl) => set({ clipboardUrl }),
  setPendingUrl: (pendingUrl) => set({ pendingUrl }),
  setBatchOpen: (batchOpen) => set({ batchOpen }),
}))
