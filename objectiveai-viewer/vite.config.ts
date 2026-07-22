import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [tailwindcss(), react()],
  // TWO entries: `index.html` is the CHROME (tab strip + status bar,
  // one per OS window) and `tab.html` is the CONTENT (one child
  // webview per tab). The Rust shell registry decides which windows
  // host which tab webviews. NOTE: an explicit rollup input drops the
  // implicit default — both entries must be listed.
  build: {
    rollupOptions: {
      input: {
        index: "index.html",
        tab: "tab.html",
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
