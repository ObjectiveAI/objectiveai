# diverge-world — read this first

Diverge World: a Tauri app beside the viewer, empty up-front. The product's owner sets the product and its words.

## The seams
- **The page is a Phaser game, no React**: `src/main.ts` makes one `Phaser.Game` filling the window, one scene per
  file under `src/scenes/`. `world` draws the map and starts `ui` over itself; `ui` draws the menus — game menus,
  drawn in the engine — with a camera of its own. Text panels are the exception, HTML over the canvas: clicking an
  agent opens its bottom sheet (`src/sheet/`), the agent's log followed live (`agents logs` with `watch`) and folded
  into messages (`sheet/fold.ts`): assistant chunks join into one message (text contiguous, an image breaking it), each
  tool call a card holding its response, user parts join per sender and are labelled by the sender. A listed tool
  opens its own log the same way (folded by `sheet/tool_fold.ts`); both are read through the one root `logs` endpoint,
  each entity carrying its `Subject`:
  every call and read the daemon routed into it, a card each with its answer, joined per caller, and each
  notification under the caller whose MCP session it came on (the proxy opens one session per caller). Clicking another
  agent or tool swaps the sheet to it; clicking it again, or ✕, closes it.
- **The world is the daemon drawn once** (`scenes/world.ts`): six lists read to their `listed` (`world/snapshot.ts`),
  retried every 5 seconds until a read succeeds, never read again; then each tool's image is asked its
  `images info` once (`world/names.ts`), and a tool is named by its image's DIVERGE.md `name` where it said one. Every account, agent, standalone tool and
  dependency tool is an entity; every event a directed link (`world/model.ts`). Each connected group is an island
  (`islands.ts`). The grid is pointy-top hexes (`hex.ts`). `place.ts` puts an island's entities on slots 3 hexes apart
  whose rings join into one island, and anneals them so lines avoid entities, plates, each other and tight fans.
  `land.ts` is every hex within 2 of an entity, nothing more; `pack.ts` packs the islands; `map.ts` puts
  it together. `render/` draws it procedurally, no art files: hexes of sea and land with sand coasts, then links (one
  arrowed line per direction, a two-way pair two parallel lines), then the entity tiles (each filling its hex) and name plates. The rules are
  numbered beside each piece. The sea has no edge: `scenes/sea.ts`, the first scene, draws one tiling sprite of sea
  hexes (`render/sea.ts`, the map's own grid) under the world, moved and scaled to the world camera every frame;
  the map itself draws only land.
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
