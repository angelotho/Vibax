import { useEffect, useState } from "react"

import type { Theme } from "@/lib/types"
import { useSettings } from "@/store/settings"

/** Aplica a classe .dark conforme a preferência (ou o tema do Windows). */
export function useApplyTheme(): "dark" | "light" {
  const pref: Theme = useSettings((s) => s.settings?.theme) ?? "dark"
  const [systemDark, setSystemDark] = useState(() => window.matchMedia("(prefers-color-scheme: dark)").matches)

  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)")
    const onChange = (e: MediaQueryListEvent) => setSystemDark(e.matches)
    mq.addEventListener("change", onChange)
    return () => mq.removeEventListener("change", onChange)
  }, [])

  const effective = pref === "system" ? (systemDark ? "dark" : "light") : pref
  useEffect(() => {
    document.documentElement.classList.toggle("dark", effective === "dark")
  }, [effective])
  return effective
}
