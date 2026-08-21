import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri 開発時は固定ポート 1420。ブラウザ開発時は /api, /r を Rust サーバへプロキシ。
const host = process.env.TAURI_DEV_HOST;
const backend = process.env.DOCSERVER_URL ?? "http://127.0.0.1:8765";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    proxy: {
      "/api": backend,
      "/r": backend,
    },
  },
  build: {
    target: ["es2021", "chrome100", "safari13"],
    sourcemap: false,
    rollupOptions: {
      output: {
        // vite 8 (rolldown) では関数形式のみ対応
        manualChunks(id: string) {
          if (id.includes("node_modules/mermaid")) return "mermaid";
          if (id.includes("node_modules/@mui/")) return "mui";
          return undefined;
        },
      },
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    globals: true,
  },
});
