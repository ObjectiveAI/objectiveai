# diverge-viewer — read this first

The Diverge viewer. The product's owner sets the product and its words. This file is committed so every session
and worktree inherits it.

## The seams
- **The page speaks the daemon's protocol itself**, with the TypeScript SDK (`diverge-sdk-js`, `@diverge/sdk`,
  a path dependency of this package through the pnpm workspace, as the Rust crates depend on `diverge-sdk-rs`
  by path). Every daemon endpoint is `daemon.endpoints.<family>.<request>.client.execute(handle, frame)`; every
  list is folded by `shared.listing.Listing` with the frame's `change()`; the answer is the frame. The page's one
  handle, the link's status and the list hook (`useListing`, `useAgents`, `useAccounts`) are `src/lib/daemon.ts`.
  The SDK's `dist` is built by the `sdk` script before `dev`, `build` and `typecheck` read it.
- **Rust holds the one connection** — the daemon's address and credential, the socket, the router — and the page
  never does: the content security policy (tauri.conf.json) connects only to Tauri's IPC. The page's handle is
  `wire.proxy.handle(...)` over four commands Rust implements as thin calls on its `diverge_sdk::wire::client::Handle`:
  `wire_request`, `wire_channel_request`, `wire_channel_response`, `wire_channel_response_finish`. Frames cross as
  bytes, one whole wire frame per channel message, header included (`src-tauri/src/wire/`).
- **The app's own things stay in Rust** (`src-tauri/src/actions.rs`, the action registry): tabs (`tabs.rs`), the
  theme, saved Views (a title and a log request the page shapes), the names you gave machines, and what the app
  knows of its folder. Their view types (`src-tauri/src/view.rs`) generate `src/bindings/*.ts`
  (`cargo test -p diverge-viewer`); the page reaches them through `src/lib/app.ts`.
- The contract is the SDK on this same tree: there is no pin, and nothing of the daemon's is mirrored here.
- **Removed on `viewer-edge-graph`** (2026-10-22): the rooms, the person layer, the agent door, hires, the
  reporter, the stand-in daemon, the image catalog and the browser preview. They were built on provider-protocol
  pieces that no longer exist; what comes back comes back designed on today's wire.
- **Files** (`src-tauri/src/store.rs`): every JSON file the app keeps says what it is and its version
  (`{"file", "version", "data"}`). A write goes to a new owner-only file, synced, then renamed over the old one,
  which becomes `<file>.bak` where the format keeps a last good copy. A file that won't parse, doesn't check, or that
  a newer version wrote, is set aside under a new name beside it and never written over; the app carries on from the
  last good copy, or starts that file empty. Every screen says which file, why, and what the app carried on from
  (`files_set_aside`, words in `src/strings.ts`).
- **One copy of the app per folder** (`store::hold`): the app holds `.in-use` in its folder with the system's file lock
  while it runs. A second copy on the same folder (or one that can't tell) reads the files as they are, writes
  nothing, and the screens say why.
- **The root** is the daemon's rule (`diverge-viewer/config`, crate `diverge-viewer-config`): `--config <dir>`,
  else `DIVERGE_CONFIG`, else `~/.diverge`. `<root>/viewer.yaml` says which daemon and as whom (`daemon.address`,
  `daemon.key`; absent = `ws://127.0.0.1:14980/` as `root`); `diverge config viewer get|set|patch` edits it with
  the same type. `<root>/viewer/` holds every file the app keeps. **Setup** is `AppState::open(folder)`
  (actions.rs); main.rs resolves the root, loads the document, builds the menu.
- **The menu** (main.rs): ⌘N new agent, ⌘W closes a TAB (never the window), ⌘, settings; the page handles ⌘3–9
  and ⌘[ ⌘]. ⌘+ ⌘− ⌘0 (Ctrl on Windows and Linux) zoom the whole window: the window's `zoomHotkeysEnabled` and the
  `core:webview:allow-set-webview-zoom` permission. Keep those keys off the menu (`src/window.test.ts`).
- Never edit the SDK's crates or packages from here; a gap in them is a change there, on its own commit.

## Product laws
- **The app sets nothing for anyone.** Every permission on screen says who set it, and only its setter can change it.
- A claim reaches a screen only once a test proves it.
- Style comes from the protocol site's tokens: `diverge-provider-web/src/layouts/Layout.astro` on origin/main.
  Gold means ONLY "where you are". All colour goes through `src/theme.css`, which is the theme: every
  words-on-background pair keeps 7:1, and every edge of something you can press or type into, and the focus ring,
  keeps 3:1 (`src/theme.test.ts`). Spacing comes only from `--space-N`, font sizes only from `--text-*`, and app.css
  names no colour of its own and styles no class the screens don't name (`src/tokens.test.ts`).
- Every action has a door in the Rust registry (`src-tauri/src/actions.rs`) or is the daemon's own, performed through
  the SDK; no logic only in a click handler.
- No popups, quickstarts, tours, modals, or anything that takes the mouse.
- Never assume who someone is — show a provider only as the daemon names it.
- Don't lead with a feed. Lead with the work: agents, runs, files, machines.
- Nothing times out and a running agent cannot be stopped (`delete` refuses active; a message can only be
  taken back before delivery). No fake stop button.
- No invented numbers. Nothing stands in for the daemon: a screen that cannot reach it says so.
- Words are tabled: every user-facing string lives in `src/strings.ts`.

## Screens (draft five, on the SDK)
The rail: the agents the daemon lists, kept current; Storage, Machines, Views, Settings. New agent · a conversation
· Storage (volumes, across providers) · Machines (the daemon's providers, with the names you gave them) · Views
(saved log requests) · Settings (the daemon's status, the theme, the folder).

## Working on it
Plain words, few of them, tl;dr first. Show screenshots, don't describe. Check the code before asking the product's
owner anything. **GitHub: this work lives on the `viewer-edge-graph` branch**, pushed after every chunk, merged when
the product's owner says. **Nothing private enters this repo**, in files or commit messages.

## Run it
`pnpm install` (from the repo root) then `cd diverge-viewer && pnpm tauri dev`. Every test, in one command:
`pnpm test:all` (the app's Rust tests, which also write the bindings, the typecheck, and vitest). A second copy on its own
root: `DIVERGE_CONFIG=/some/root pnpm tauri dev`. In a plain browser (`pnpm dev`, http://localhost:1430)
nothing answers for the app or the daemon, and the screens say so.
