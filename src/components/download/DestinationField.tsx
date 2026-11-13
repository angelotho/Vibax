import { open } from "@tauri-apps/plugin-dialog"
import { FolderOpen } from "lucide-react"
import { useEffect, useState } from "react"

import { Button } from "@/components/ui/button"
import { api } from "@/lib/api"
import { formatBytes, shortPath } from "@/lib/format"
import type { FolderInfo } from "@/lib/types"

/** Pasta de destino com espaço livre e botão para alterar. */
export function DestinationField({ value, onChange }: { value: string; onChange: (dir: string) => void }) {
  const [info, setInfo] = useState<FolderInfo | null>(null)

  useEffect(() => {
    let alive = true
    if (value) api.folderInfo(value).then((i) => alive && setInfo(i)).catch(() => {})
    return () => {
      alive = false
    }
  }, [value])

  const choose = async () => {
    const dir = await open({ directory: true, multiple: false, defaultPath: value || undefined, title: "Escolha a pasta de destino" })
    if (typeof dir === "string" && dir) onChange(dir)
  }

  return (
    <div className="flex items-center gap-3 rounded-xl bg-muted px-3.5 py-2.5">
      <FolderOpen className="size-4 shrink-0 text-muted-foreground" strokeWidth={1.75} />
      <div className="min-w-0 flex-1">
        <p className="truncate text-[13px] font-medium" title={value}>
          {shortPath(value, 3)}
        </p>
        <p className="tnum text-[11.5px] text-muted-foreground">
          {info?.freeBytes != null ? `${formatBytes(info.freeBytes)} livres` : info && !info.exists ? "A pasta será criada" : " "}
        </p>
      </div>
      <Button type="button" variant="outline" size="sm" className="press h-8 rounded-lg px-3" onClick={choose}>
        Alterar
      </Button>
    </div>
  )
}
