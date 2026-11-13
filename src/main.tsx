import { StrictMode } from "react"
import { createRoot } from "react-dom/client"

import App from "./App"
import "./index.css"

// Tema escuro aplicado antes da primeira pintura para evitar clarão branco.
document.documentElement.classList.add("dark")

// Menu de contexto do navegador não faz sentido num app desktop (exceto em campos de texto).
document.addEventListener("contextmenu", (e) => {
  const t = e.target as HTMLElement
  if (!t.closest("input, textarea, .selectable")) e.preventDefault()
})

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
