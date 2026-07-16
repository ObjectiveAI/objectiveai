import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [tailwindcss(), react()],
  // Three entry points, one shared src/: the main viewer, the
  // per-agent conversation window, and the per-laboratory filesystem
  // window (both opened by the Rust shell via init-script globals).
  build: {
    rollupOptions: {
      input: {
        main: "index.html",
        agent: "agent.html",
        laboratory: "laboratory.html",
      },
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
