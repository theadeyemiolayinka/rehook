import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    // Proxy API and inbound webhook endpoints to the backend during dev.
    proxy: {
      '/api': 'http://127.0.0.1:8080',
      '/i': 'http://127.0.0.1:8080',
    },
  },
  build: {
    // Output to a directory the Rust server serves in production.
    outDir: 'dist',
    sourcemap: false,
  },
})
