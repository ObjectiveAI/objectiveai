# objectiveai-plugin-scaffold (viewer half)

Starting point for an ObjectiveAI VIEWER plugin: a boot tab, a
channel-request handler, and a browser-tab script. The channel handler
answers the MCP half's `scaffold.credential` requests.

## Where the plugin root is

Everything registers **the directory holding `objectiveai.json`**:

- **this directory**, when this half stands alone — and
  `viewer.development.output` is `dist`;
- **the parent**, when this is the `viewer/` half of a full plugin (what
  the root `scaffold.sh` produces) — and `viewer.development.output` is
  `viewer/dist`. The watch build still runs in HERE either way.

`development.output` is a HOST path resolved against the registered
directory, which is why it differs; `viewer.output` (`/dist`) is a path
inside the built image and never changes.

**Identity is not in this repo.** Owner, name, and version come from the
git tag/path on release, and from the explicit
`development plugins viewer create --owner … --name … --version …`
registration in development. Registering BOTH halves under the SAME trio
AND the same `--path` makes them ONE plugin in development mode. A
RELEASED plugin is one repo whose `objectiveai.json` carries both
halves.

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
can never reach — see `src/capture.ts`). No Tauri IPC, no SDK: the page
shares the JS world, so anything more would be hijackable. The spawning
TAB is the trusted brain and must treat bridge messages as untrusted.

## Build

`pnpm install`, then `pnpm run build` (once) or `pnpm run dev` (watch →
`dist/`, which the manifest points at through `development.output`). The
Containerfile runs the identical build for release; reproduce it with the
commands in its header. Every module/style the manifest declares must
exist in `dist/` — the release build fails otherwise.

## Rename

If you scaffolded with `scaffold.sh`, the name is already yours, taken
from the directory you ran it in. Otherwise `./rename.sh <new-name>`
rewrites the placeholder name here and in `package.json`. Keep the
registration trio in sync with the MCP half.
