import { Layers, ListPlus, Sparkles } from "lucide-react"
import { toast } from "sonner"

import { EmptyState } from "@/components/common/EmptyState"
import { JobRow, act } from "@/components/queue/JobRow"
import { Button } from "@/components/ui/button"
import { api } from "@/lib/api"
import { ACTIVE, useQueueList } from "@/store/downloads"
import { useSettings } from "@/store/settings"
import { useUi } from "@/store/ui"

export function QueuePage() {
  const list = useQueueList()
  const go = useUi((s) => s.go)
  const setBatchOpen = useUi((s) => s.setBatchOpen)
  const max = useSettings((s) => s.settings?.maxConcurrent ?? 2)

  const running = list.filter((j) => ACTIVE.includes(j.status) || j.status === "queued" || j.status === "paused")
  const finished = list.filter((j) => j.status === "completed" || j.status === "cancelled" || j.status === "error").reverse()
  const clearable = finished.filter((j) => j.status !== "error").length
  const activeNow = list.filter((j) => ACTIVE.includes(j.status)).length

  return (
    <div className="mx-auto flex w-full max-w-[1000px] flex-col px-10 pb-12 pt-10">
      <header className="flex items-end justify-between gap-4">
        <div>
          <h1 className="text-[28px] font-semibold leading-tight tracking-[-0.025em]">Fila</h1>
          <p className="tnum mt-1.5 text-[14px] text-muted-foreground">
            {list.length === 0
              ? "Os downloads aparecem aqui enquanto são processados."
              : `${activeNow} de ${max} ${max === 1 ? "vaga em uso" : "vagas em uso"}, ${running.length} ${running.length === 1 ? "item pendente" : "itens pendentes"}`}
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" className="press h-9 gap-2 rounded-xl" onClick={() => setBatchOpen(true)}>
            <ListPlus className="size-4" strokeWidth={1.75} /> Adicionar links
          </Button>
          <Button
            variant="ghost"
            className="press h-9 gap-2 rounded-xl"
            disabled={clearable === 0}
            onClick={() =>
              act(async () => {
                const n = await api.clearFinished()
                if (n > 0) toast.success(n === 1 ? "1 item removido da fila" : `${n} itens removidos da fila`)
              })
            }
          >
            <Sparkles className="size-4" strokeWidth={1.75} /> Limpar concluídos
          </Button>
        </div>
      </header>

      {list.length === 0 ? (
        <EmptyState
          icon={Layers}
          title="Nenhum download na fila"
          action={
            <Button className="press rounded-xl" onClick={() => go("download")}>
              Colar um link
            </Button>
          }
        >
          Analise um link na tela Baixar ou adicione vários de uma vez.
        </EmptyState>
      ) : (
        <div className="mt-6 flex flex-col gap-6">
          {running.length > 0 && (
            <section aria-label="Pendentes">
              <h2 className="mb-1 px-3 text-[13px] font-medium text-muted-foreground">Pendentes</h2>
              <ul className="flex flex-col">
                {running.map((j) => (
                  <JobRow key={j.id} job={j} />
                ))}
              </ul>
            </section>
          )}
          {finished.length > 0 && (
            <section aria-label="Finalizados">
              <h2 className="mb-1 px-3 text-[13px] font-medium text-muted-foreground">Finalizados</h2>
              <ul className="flex flex-col">
                {finished.map((j) => (
                  <JobRow key={j.id} job={j} />
                ))}
              </ul>
            </section>
          )}
        </div>
      )}
    </div>
  )
}
