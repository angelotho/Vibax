// Tipos espelhados do backend Rust (serde camelCase).

export type ErrorCode =
  | "invalid_url"
  | "unsupported"
  | "network"
  | "unavailable"
  | "auth_required"
  | "drm"
  | "geo_blocked"
  | "live"
  | "forbidden"
  | "no_formats"
  | "disk_full"
  | "permission"
  | "processing"
  | "tool_missing"
  | "not_found"
  | "busy"
  | "internal"

export interface AppError {
  code: ErrorCode
  message: string
  detail?: string
}

export type Container = "mp4" | "webm" | "mkv"
export type AudioFormat = "mp3" | "m4a"

export interface ContainerOption {
  container: Container
  videoFormatId: string
  audioFormatId: string | null
  videoHasAudio: boolean
  vcodec: string
  sizeApprox: number | null
}

export interface QualityOption {
  height: number
  label: string
  fps: number | null
  containers: ContainerOption[]
}

export interface AudioOption {
  formatId: string
  acodec: string
  abr: number | null
  sizeApprox: number | null
}

export interface PlaylistEntry {
  url: string
  title: string | null
  duration: number | null
}

export interface MediaInfo {
  kind: "video" | "playlist"
  id: string
  url: string
  title: string
  uploader: string | null
  duration: number | null
  thumbnail: string | null
  source: string
  sourceLabel: string
  qualities: QualityOption[]
  audio: AudioOption | null
  entries: PlaylistEntry[]
}

export type DownloadChoice =
  | {
      kind: "video"
      height: number
      label: string
      container: Container
      videoFormatId: string | null
      audioFormatId: string | null
      videoHasAudio: boolean
    }
  | {
      kind: "audio"
      format: AudioFormat
      bitrate: number | null
      audioFormatId: string | null
    }

export type Preference =
  | { kind: "video"; maxHeight: number; container: Container }
  | { kind: "audio"; format: AudioFormat; bitrate: number | null }

export type JobStatus =
  | "queued"
  | "analyzing"
  | "downloading"
  | "processing"
  | "paused"
  | "completed"
  | "cancelled"
  | "error"

export interface Job {
  id: string
  url: string
  title: string | null
  uploader: string | null
  thumbnail: string | null
  thumbPath: string | null
  sourceLabel: string | null
  duration: number | null
  choice: DownloadChoice | null
  preference: Preference | null
  formatLabel: string
  qualityLabel: string | null
  destDir: string
  status: JobStatus
  stage: string | null
  progress: number | null
  downloadedBytes: number
  totalBytes: number | null
  speed: number | null
  eta: number | null
  filePath: string | null
  fileSize: number | null
  error: AppError | null
  createdAt: number
  startedAt: number | null
  finishedAt: number | null
  inQueue: boolean
  inHistory: boolean
  fileExists?: boolean
}

export type Theme = "dark" | "light" | "system"
export type NotifyMode = "always" | "background" | "never"

export interface Settings {
  downloadDir: string
  defaultKind: "video" | "audio"
  defaultContainer: Container
  defaultAudioFormat: AudioFormat
  defaultQuality: number
  mp3Bitrate: number
  maxConcurrent: number
  theme: Theme
  notifications: NotifyMode
  clipboardWatch: boolean
  embedMetadata: boolean
}

export interface UrlCheck {
  valid: boolean
  normalized: string | null
  reason: string | null
}

export interface ToolInfo {
  ytdlpVersion: string | null
  ytdlpPath: string
  ytdlpUpdated: boolean
  ffmpegVersion: string | null
  denoVersion: string | null
}

export interface AppInfo {
  version: string
  tools: ToolInfo | null
  toolsError: AppError | null
  dataDir: string
}

export interface UpdateResult {
  updated: boolean
  current: string | null
  latest: string
}

export interface FolderInfo {
  path: string
  exists: boolean
  freeBytes: number | null
}
