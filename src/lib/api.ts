// Única porta de comunicação com o backend. Cada função corresponde a um
// comando Rust em src-tauri/src/commands.rs.

import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

import type {
  AppError,
  AppInfo,
  DownloadChoice,
  FolderInfo,
  Job,
  MediaInfo,
  Preference,
  Settings,
  UpdateResult,
  UrlCheck,
} from "./types"

/** Normaliza qualquer falha de `invoke` para o formato AppError. */
export function toAppError(e: unknown): AppError {
  if (e && typeof e === "object" && "code" in e && "message" in e) return e as AppError
  if (typeof e === "string") return { code: "internal", message: e }
  return { code: "internal", message: "Ocorreu um erro inesperado." }
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args)
  } catch (e) {
    throw toAppError(e)
  }
}

export const api = {
  getSettings: () => call<Settings>("get_settings"),
  saveSettings: (settings: Settings) => call<Settings>("save_settings", { settings }),

  checkUrl: (text: string) => call<UrlCheck>("check_url", { text }),
  extractUrls: (text: string) => call<{ valid: string[]; invalid: string[] }>("extract_urls", { text }),
  analyzeUrl: (url: string) => call<MediaInfo>("analyze_url", { url }),

  enqueueDownload: (url: string, choice: DownloadChoice, destDir?: string) =>
    call<Job>("enqueue_download", { url, choice, destDir: destDir ?? null }),
  enqueueMany: (urls: string[], preference?: Preference, destDir?: string) =>
    call<Job[]>("enqueue_many", { urls, preference: preference ?? null, destDir: destDir ?? null }),

  listQueue: () => call<Job[]>("list_queue"),
  pauseJob: (id: string) => call<void>("pause_job", { id }),
  resumeJob: (id: string) => call<void>("resume_job", { id }),
  cancelJob: (id: string) => call<void>("cancel_job", { id }),
  retryJob: (id: string) => call<void>("retry_job", { id }),
  removeJob: (id: string) => call<void>("remove_job", { id }),
  clearFinished: () => call<number>("clear_finished"),
  activeCount: () => call<number>("active_count"),

  listHistory: (search?: string) => call<Job[]>("list_history", { search: search ?? null }),
  removeHistory: (id: string) => call<void>("remove_history", { id }),
  clearHistory: () => call<void>("clear_history"),
  redownload: (id: string) => call<Job>("redownload", { id }),

  openJobFile: (id: string) => call<void>("open_job_file", { id }),
  revealJobFile: (id: string) => call<void>("reveal_job_file", { id }),
  openFolder: (path: string) => call<void>("open_folder", { path }),
  folderInfo: (path: string) => call<FolderInfo>("folder_info", { path }),
  openKnownLink: (key: "ytdlp" | "ffmpeg" | "ffmpeg-source" | "deno" | "tauri") => call<void>("open_known_link", { key }),

  appInfo: () => call<AppInfo>("app_info"),
  updateYtdlp: () => call<UpdateResult>("update_ytdlp"),
  resetYtdlp: () => call<void>("reset_ytdlp"),
  quit: () => call<void>("quit_app"),
}

export const events = {
  onJobUpdated: (cb: (job: Job) => void): Promise<UnlistenFn> => listen<Job>("job-updated", (e) => cb(e.payload)),
  onJobRemoved: (cb: (id: string) => void): Promise<UnlistenFn> => listen<string>("job-removed", (e) => cb(e.payload)),
  onCloseRequested: (cb: (active: number) => void): Promise<UnlistenFn> =>
    listen<number>("close-requested", (e) => cb(e.payload)),
}
