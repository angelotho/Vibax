/// <reference types="vitest/config" />
import path from 'node:path'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

const host = process.env.TAURI_DEV_HOST

// No Windows, abrir o projeto como "d:\..." em vez de "D:\..." faz o Vite
// resolver módulos por dois caminhos diferentes (duas cópias do React).
const root = path.resolve(import.meta.dirname).replace(/^[a-z]:/, (d) => d.toUpperCase())

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  root,
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { '@': path.join(root, 'src') },
    dedupe: ['react', 'react-dom'],
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    target: 'chrome120',
    minify: !process.env.TAURI_ENV_DEBUG ? 'oxc' : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    chunkSizeWarningLimit: 900,
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
})
