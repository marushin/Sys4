import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';

const page = (file) => fileURLToPath(new URL(file, import.meta.url));

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  // Tauri expects a fixed port and should own the terminal output.
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    // WebView2 (Chromium) only — no legacy transpilation needed.
    target: 'chrome110',
    rollupOptions: {
      input: {
        main: page('./index.html'),
        widget: page('./widget.html'),
      },
    },
  },
});
