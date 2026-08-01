import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;

// Stable dev URLs for the host-React shims the tab.html import map
// names — resolveId maps the URL onto the shim source, so the dev
// server serves it transformed, importing the SAME optimized react
// dep as every host module (one React instance). Production gets the
// same URLs from the unhashed `host/` rollup entries below.
const HOST_SHIMS: Record<string, string> = {
  "/host/react.js": "./src/host/react.ts",
  "/host/react-dom.js": "./src/host/react-dom.ts",
  "/host/react-dom-client.js": "./src/host/react-dom-client.ts",
  "/host/react-jsx-runtime.js": "./src/host/react-jsx-runtime.ts",
  "/host/react-jsx-dev-runtime.js": "./src/host/react-jsx-dev-runtime.ts",
};
const hostShims = {
  name: "host-react-shims",
  apply: "serve" as const,
  resolveId(id: string) {
    const source = HOST_SHIMS[id];
    return source ? fileURLToPath(new URL(source, import.meta.url)) : undefined;
  },
};

export default defineConfig(async () => ({
  plugins: [tailwindcss(), react(), hostShims],
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
  // Entries: `index.html` is the tab STRIP and `status.html` the
  // bottom bar — two band-sized chrome webviews per OS window rather
  // than one full-window document, so the content band between them is
  // covered by nothing (see src/status.tsx). `tab.html` is the generic
  // CONTENT bootstrap (one
  // child webview per tab, importing whatever module Rust's
  // descriptor names), and each `src/tabs/*` is one built-in tab
  // component — emitted as a stably-named unhashed chunk so the
  // module paths JS hands to `tabs_open` (`/tabs/<stem>.js`) hold in
  // production. NOTE: an explicit rollup input drops the implicit
  // default — everything must be listed.
  build: {
    rollupOptions: {
      // Every `tabs/*` and `host/*` entry exists to be IMPORTED at
      // runtime — the tab bootstrap dynamic-imports the former and the
      // `tab.html` import map resolves the latter — and BOTH of those
      // import paths are invisible to the bundler (`@vite-ignore`, and
      // an import map is not a module graph edge). An app build
      // therefore sees entries whose exports nobody uses and strips
      // their signatures, leaving modules that export NOTHING.
      //
      // The failure is silent in both directions: a stripped tab
      // module makes `tab.tsx`'s `module[export ?? "default"]` come
      // back undefined and render null, and a stripped host shim makes
      // every plugin bundle die at link time on
      // `Importing binding name 'jsxs' is not found`.
      preserveEntrySignatures: "strict",
      input: {
        index: "index.html",
        status: "status.html",
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
        "host/react": "src/host/react.ts",
        "host/react-dom": "src/host/react-dom.ts",
        "host/react-dom-client": "src/host/react-dom-client.ts",
        "host/react-jsx-runtime": "src/host/react-jsx-runtime.ts",
        "host/react-jsx-dev-runtime": "src/host/react-jsx-dev-runtime.ts",
      },
      output: {
        entryFileNames: (chunk: { name: string }) =>
          chunk.name.startsWith("tabs/") || chunk.name.startsWith("host/")
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
