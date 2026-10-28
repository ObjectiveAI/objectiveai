# diverge-world — read this first

Diverge World: a Tauri app beside the viewer, empty up-front. The product's owner sets the product and its words.

## The seams
- **The page speaks the daemon's protocol itself**, with the TypeScript SDK (`@diverge/sdk`, a path dependency through
  the pnpm workspace, built by the `sdk` script before `dev`, `build` and `typecheck`). Its one handle is
  `src/lib/wire.ts`: `wire.proxy.handle(...)` over the app's Rust. Every daemon endpoint is
  `daemon.endpoints.<family>.<request>.client.execute(handle, frame)`.
- **Rust holds the one connection** — the address, the identity, the socket — through the SDK's own `tauri_proxy`
  runner (the `tauri-proxy` feature): main.rs manages one `TauriProxy` and registers its six commands, the four sends
  and `wire_status`/`wire_reset`. The content security policy connects only to Tauri's IPC.
- **The root and the config are the viewer's**: `--config <dir>`, else `DIVERGE_CONFIG`, else `~/.diverge`, and
  `<root>/viewer.yaml` read with the viewer's own type (`diverge-viewer/config`, crate `diverge-viewer-config`):
  `daemon.address`, `daemon.issuer`, `daemon.subject` — one of `<root>/identities/`, signed in by
  `diverge identities login` — and `daemon.redial`. A document naming no daemon stops the start.
  `diverge viewer config get|set|patch` edits it.
- **The link dials again by itself**: as `daemon.redial` says — every 5 seconds when absent, as the daemon redials a
  provider, a whole number of seconds, or `never` — after a failed dial and after a connection that ended, dialling at
  start. `wire_reset` dials at once. The rules are the SDK's (`diverge-sdk-rs/src/wire/proxy/link.rs`).
- **The app keeps no files.**

## Run it
`pnpm install` (from the repo root) then `cd diverge-world && pnpm tauri dev`. The page is on port 1440 (HMR 1441), so
it runs beside the viewer (1430). A second copy on its own root: `DIVERGE_CONFIG=/some/root pnpm tauri dev`.

## Working on it
Plain words, few of them, tl;dr first. **GitHub: this work lives on the `viewer-edge-graph` branch**, pushed after
every chunk. **Nothing private enters this repo**, in files or commit messages.
