import { resolve } from "node:path";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Two pages: the click-through overlay (one window per monitor) and the settings panel.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: {
    target: "es2022",
    rollupOptions: {
      input: {
        overlay: resolve(import.meta.dirname, "overlay.html"),
        panel: resolve(import.meta.dirname, "panel.html"),
      },
    },
  },
});
