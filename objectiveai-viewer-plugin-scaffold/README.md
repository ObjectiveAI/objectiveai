# objectiveai-plugin-scaffold (viewer half)

Starting point for an ObjectiveAI VIEWER plugin: a boot tab, a
channel-request handler, and a browser-tab script. The MCP half lives in
its sibling scaffold (`objectiveai-mcp-plugin-scaffold-rs`); this repo's
channel handler answers that scaffold's `scaffold.credential` requests.

**Identity is not in this repo.** Owner, name, and version come from the
git tag/path on release, and from the explicit
`development plugins viewer create --owner … --name … --version …`
registration in development. Registering BOTH scaffolds under the SAME
trio (one `mcp` half, one `viewer` half) makes them ONE plugin in
development mode. A RELEASED plugin is one repo whose `objectiveai.json`
carries both halves — merge them before tagging.

## The one hard contract

Tab bundles are ESM with `react`, `react-dom`, `react/jsx-runtime`, and
`react-dom/client` left EXTERNAL: the host serves the single React
instance through an import map, and a bundle carrying its own dies on the
first hook. Everything else (`@objectiveai/sdk`, `@tauri-apps/api`,
`canvas-confetti`) bundles in. Scripts are the opposite: classic IIFE,
nothing external, CSS inlined as text.

## What a script can do

Exactly the child-side mailbox toward its spawning tab, nothing else:
`__objectiveai.send / subscribe / list` (a closure-local binding the page
can never reach — see `src/overlay.ts`). No Tauri IPC, no SDK: the page
shares the JS world, so anything more would be hijackable. The spawning
TAB is the trusted brain and must treat bridge messages as untrusted.

## Build

`pnpm install`, then `pnpm run build` (once) or `pnpm run dev` (watch →
`dist/`, the manifest's `development.output`). The Containerfile runs the
identical build for release; reproduce it with the commands in its
header. Every module/style the manifest declares must exist in `dist/` —
the release build fails otherwise.

## Rename

`./rename.sh <new-name>` rewrites the placeholder name here,
package.json, and the skill. Keep the registration trio in sync with the
MCP half.
