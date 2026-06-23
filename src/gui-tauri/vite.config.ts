import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri expects the dev server on port 1420 (matches tauri.conf.json devUrl).
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // Don't watch the Rust source from the Vite dev server.
      ignored: ["**/src-tauri/**"],
    },
  },
  // Tauri custom protocol in production.
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2021",
    minify: "esbuild",
    sourcemap: false,
  },
});
