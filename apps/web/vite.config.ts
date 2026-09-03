import { fileURLToPath, URL } from 'node:url'

import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
/// <reference types="vitest/config" />
import { defineConfig } from 'vitest/config'

/**
 * Una sola origine, in sviluppo come in produzione.
 *
 * Il proxy inoltra `/api/v1` all'API Rust, quindi il browser vede un solo host e
 * un solo schema. Ne segue tutto ciò che questa fetta **non** deve fare: niente
 * CORS da montare, niente elenco di origini ammesse da tenere allineato fra
 * ambienti, e un cookie di prima parte che può essere `SameSite=Strict` invece di
 * `SameSite=None; Secure`. In produzione lo stesso contratto lo realizza un
 * reverse proxy — `/` ai file statici, `/api/v1` all'API.
 *
 * `changeOrigin` resta **false**: l'API non fa virtual hosting, e riscrivere
 * `Host` cambierebbe l'origine che il browser e il server vedono, cioè
 * precisamente la proprietà per cui il proxy esiste.
 */
const API_TARGET = process.env['PROPFLOW_API_URL'] ?? 'http://localhost:8080'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  server: {
    port: 5173,
    proxy: {
      '/api/v1': { target: API_TARGET, changeOrigin: false },
    },
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    globals: true,
    include: ['src/**/*.test.{ts,tsx}'],
  },
})
