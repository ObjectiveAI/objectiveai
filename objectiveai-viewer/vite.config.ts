import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [tailwindcss(), react()],
  // Pre-optimize deps that only DYNAMICALLY-imported tab modules pull
  // in (the channel-request tab is spawned by Rust, so vite's crawl
  // never sees it up front). Without this, the first such import
  // mid-session re-optimizes the dep graph — the importing webview
  // briefly holds TWO React copies (invalid-hook crash) until vite's
  // full-reload signal heals it.
  optimizeDeps: {
    include: [
      "@radix-ui/react-collapsible",
      "react-markdown",
      "remark-breaks",
      "remark-gfm",
    ],
  },
  // Entries: `index.html` is the CHROME (tab strip + status bar, one
  // per OS window), `tab.html` is the generic CONTENT bootstrap (one
  // child webview per tab, importing whatever module Rust's
  // descriptor names), and each `src/tabs/*` is one built-in tab
  // component — emitted as a stably-named unhashed chunk so the
  // module paths JS hands to `tabs_open` (`/tabs/<stem>.js`) hold in
  // production. NOTE: an explicit rollup input drops the implicit
  // default — everything must be listed.
  build: {
    rollupOptions: {
      input: {
        index: "index.html",
        tab: "tab.html",
        "tabs/agents": "src/tabs/agents.tsx",
        "tabs/laboratories": "src/tabs/laboratories.tsx",
        "tabs/viewer-logs": "src/tabs/viewer-logs.tsx",
        "tabs/command-logs": "src/tabs/command-logs.tsx",
        "tabs/agent": "src/tabs/agent.tsx",
        "tabs/laboratory": "src/tabs/laboratory.tsx",
        "tabs/command-log": "src/tabs/command-log.tsx",
        "tabs/channel-request": "src/tabs/channel-request.tsx",
        "tabs/tabs": "src/tabs/tabs.tsx",
      },
      output: {
        entryFileNames: (chunk: { name: string }) =>
          chunk.name.startsWith("tabs/")
            ? "[name].js"
            : "assets/[name]-[hash].js",
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
