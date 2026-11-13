import { convertFileSrc } from "@tauri-apps/api/core"
import { Film, Music } from "lucide-react"
import { useState } from "react"

import { cn } from "@/lib/utils"

/** Miniatura 16:9 (arquivo local salvo pelo Vibax, ou a imagem remota da fonte). */
export function Thumb({
  localPath,
  remote,
  audio,
  className,
}: {
  localPath?: string | null
  remote?: string | null
  audio?: boolean
  className?: string
}) {
  const [failed, setFailed] = useState(false)
  const src = localPath ? convertFileSrc(localPath) : remote
  const Icon = audio ? Music : Film
  return (
    <div className={cn("img-outline relative aspect-video shrink-0 overflow-hidden rounded-lg bg-muted", className)}>
      {src && !failed ? (
        <img
          src={src}
          alt=""
          draggable={false}
          loading="lazy"
          referrerPolicy="no-referrer"
          onError={() => setFailed(true)}
          className="size-full object-cover"
        />
      ) : (
        <div className="grid size-full place-items-center text-muted-foreground/60">
          <Icon className="size-5" strokeWidth={1.5} />
        </div>
      )}
    </div>
  )
}
