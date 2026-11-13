import { create } from "zustand"

import { api } from "@/lib/api"
import type { Job } from "@/lib/types"

interface DownloadsState {
  jobs: Record<string, Job>
  order: string[]
  loaded: boolean
  /** Incrementado quando o histórico muda (conclusão/remoção), para recarregá-lo. */
  historyVersion: number
  load: () => Promise<void>
  upsert: (job: Job) => void
  remove: (id: string) => void
}

export const useDownloads = create<DownloadsState>((set, get) => ({
  jobs: {},
  order: [],
  loaded: false,
  historyVersion: 0,
  load: async () => {
    const list = await api.listQueue()
    set({
      jobs: Object.fromEntries(list.map((j) => [j.id, j])),
      order: list.map((j) => j.id),
      loaded: true,
    })
  },
  upsert: (job) => {
    const prev = get().jobs[job.id]
    const becameDone = job.status === "completed" && prev?.status !== "completed"
    set((s) => ({
      jobs: { ...s.jobs, [job.id]: job },
      order: s.order.includes(job.id) ? s.order : [...s.order, job.id],
      historyVersion: becameDone ? s.historyVersion + 1 : s.historyVersion,
    }))
  },
  remove: (id) =>
    set((s) => {
      const { [id]: _removed, ...rest } = s.jobs
      return { jobs: rest, order: s.order.filter((x) => x !== id), historyVersion: s.historyVersion + 1 }
    }),
}))

export const ACTIVE: Job["status"][] = ["analyzing", "downloading", "processing"]

export function useQueueList(): Job[] {
  const jobs = useDownloads((s) => s.jobs)
  const order = useDownloads((s) => s.order)
  return order.map((id) => jobs[id]).filter(Boolean)
}

/** Resumo para o trilho lateral: itens pendentes e velocidade total. */
export function useQueueSummary() {
  const list = useQueueList()
  const active = list.filter((j) => ACTIVE.includes(j.status))
  const pending = list.filter((j) => j.status === "queued" || ACTIVE.includes(j.status))
  const speed = active.reduce((acc, j) => acc + (j.speed ?? 0), 0)
  return { active: active.length, pending: pending.length, speed }
}
