// The ONE build, shared by `pnpm run build`, `pnpm run dev`
// (`--watch`), and the Containerfile. Two opposite esbuild passes:
//
// - TABS (`home.tsx`, `credential.tsx`): ESM with react EXTERNAL —
//   the host viewer renders these components and owns the single
//   React instance (served through an import map; a bundle carrying
//   its own copy dies on the first hook). Everything else — the SDK,
//   canvas-confetti, @tauri-apps/api — bundles IN.
// - SCRIPTS (`overlay.ts`): a CLASSIC script injected into a page
//   this plugin does not own — `iife`, nothing external, CSS inlined
//   as text for CSSOM (no URL it could fetch is reachable there).
//
// Declared stylesheets are copied through as real files — a bundler
// strips `import "./x.css"` from a JS entry, so the manifest's
// `styles` (which the host injects and awaits) is the only path that
// works.
import { build, context } from "esbuild";
import { copyFileSync, mkdirSync } from "node:fs";

const watch = process.argv.includes("--watch");

const tabs = {
  entryPoints: ["src/home.tsx", "src/credential.tsx"],
  bundle: true,
  format: "esm",
  platform: "browser",
  outdir: "dist",
  jsx: "automatic",
  external: [
    "react",
    "react-dom",
    "react/jsx-runtime",
    "react/jsx-dev-runtime",
    "react-dom/client",
    // The SDK's node-only code paths (spawning a local CLI) sit behind
    // dynamic imports a webview never reaches — leave the builtins
    // unresolved rather than bundling for node.
    "child_process",
    "os",
    "readline",
    "node:*",
  ],
};

const scripts = {
  entryPoints: ["src/overlay.ts"],
  bundle: true,
  format: "iife",
  platform: "browser",
  outfile: "dist/overlay.js",
  loader: { ".css": "text" },
};

function styles() {
  mkdirSync("dist", { recursive: true });
  copyFileSync("src/home.css", "dist/home.css");
  copyFileSync("src/credential.css", "dist/credential.css");
}

if (watch) {
  const rebuildStyles = {
    name: "styles",
    setup(build) {
      build.onEnd(styles);
    },
  };
  const a = await context({ ...tabs, plugins: [rebuildStyles] });
  const b = await context(scripts);
  await Promise.all([a.watch(), b.watch()]);
  console.log("watching src/ -> dist/");
} else {
  await Promise.all([build(tabs), build(scripts)]);
  styles();
}
