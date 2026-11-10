# diverge-world — read this first

Diverge World: a Tauri app beside the viewer, empty up-front. The product's owner sets the product and its words.

## The seams
- **The page is a Phaser game, no React**: `src/main.ts` makes one `Phaser.Game` filling the window, one scene per
  file under `src/scenes/`. `world` draws the map and starts `ui` over itself; `ui` draws the menus — game menus,
  drawn in the engine — with a camera of its own: one menu, the outliner
  (`scenes/outliner.ts`) top-right — Arrange and one row per line type at its top, the status row beneath, then
  every node by kind, each kind's header an eye that shows or hides it (`KINDS`), a row's click (`FOCUS`) flying the board's camera to it and
  opening its sheet, a row's hover (`HOVER`) focusing it; the board tells it the nodes (`OUTLINE`) and the open one
  (`SELECTED`) through the registry, and pans and zooms for nothing over its bounds (`OUTLINER`). Text panels are the exception, HTML over the canvas: clicking an
  agent opens its bottom sheet (`src/sheet/`), the agent's log followed live (`agents logs` with `watch`) and folded
  into messages (`sheet/fold.ts`): assistant chunks join into one message (text contiguous, an image breaking it), each
  tool call a card holding its response, user parts join per sender and are labelled by the sender. Under an agent's
  log, its composer (`sheet/composer.ts`) writes to it (`agents message`): text, then files attached by 📎 or a paste,
  each known by its bytes (`file-type`, `sheet/attach.ts`) and sent as an MCP image, audio or resource block in base64;
  a message sent waits greyed at the log's bottom, one request and one answer with no cancel, in the page's outbox
  (`sheet/outbox.ts`, by route and agent, outliving every sheet); `delivered` (the program accepted it) turns it white
  until the log shows an account's user part matching it (first text block, else first block), which takes it out; a
  refusal leaves it red until clicked. A listed tool
  opens its own log the same way (folded by `sheet/tool_fold.ts`); both are read through the one root `logs` endpoint,
  each entity carrying its `Subject`. A tool's sheet also holds the commands its container put to the daemon, and
  clicking an account opens its command log — every request it put to the daemon, marked allowed or refused (an
  administrator the daemon's `admins` names is listed as an account too, its log the `admin` subject — the world's own):
  every call and read the daemon routed into it, a card each with its answer, joined per caller, and each
  notification under the caller whose MCP session it came on (the proxy opens one session per caller). Clicking another
  agent or tool swaps the sheet to it; clicking it again, or ✕, closes it. The sheet has tabs, `logs` and
  `filesystem` (`sheet/files.ts`, `sheet/filetree.ts`): an agent's, tool's or dependency tool's container tree watched
  live (`<family> filetree`), a volume's asked once (`volumes filetree`, ↻ asks again) — columns of name, size, created
  and modified, directories first, sortable headers, changed rows glowing. Right-clicking a row opens a menu that downloads it — a file, or a folder whole — where the
  system's dialog says; files dragged onto the tree upload into the directory they are aimed at (its rows glow, a line
  says where), a volume re-read after. The app's Rust moves the bytes (`src-tauri/src/transfer.rs`), disk to wire in
  2 MiB pieces one frame ahead, never through the page; the strip at the window's corner (`sheet/strip.ts`) shows
  each transfer and stops it. A third tab, `console` (`sheet/console.ts`), is one interactive shell in an agent's,
  tool's or dependency tool's container (`<family> terminal`), drawn by xterm (`@xterm/xterm`, `@xterm/addon-fit`):
  keys go to its stdin one write in flight, the rest held and sent as one; the pane's size, settled, is its size;
  closing or swapping the sheet closes it, which kills the shell. A fourth tab, `frontend` (`sheet/frontend.ts`), shows
  an agent's or a tool's own web frontend (its image's DIVERGE.md says `frontend: true`; `Entity.frontend`, from `world/names.ts`'s
  `imageInfo`) in an `<iframe>`: the app's Rust (`src-tauri/src/frontend.rs`, commands `frontend_open`/`frontend_close`)
  serves it from `127.0.0.1:<fresh port>` (axum), each request one daemon `<family> frontend` scope over the page's
  connection, bodies streamed, a `101` piped both ways; gated by Host, `Sec-Fetch-Site` and a key on the first load;
  closed with the sheet, on a swap, and on every page load. An account
  and a connected standalone tool (another daemon's, its own kind: `connected_standalone_tools list`, the
  `connected_standalone_tool` log subject) have only logs, a volume only its files, a provider (`providers list`, the
  record the daemon keeps of one; no log subject) no tab at all, and a daemon (`daemons list`, the record of one account
  of ours on another daemon; no log subject; `Entity.daemon` its record's ID) one tab of its own, `daemon`, a single
  button "Open <name>" that opens that daemon's tab of the window — a provider's header's second line its direction and
  status, a daemon's `as <our subject> · connected|disconnected`; each tab starts reading the first time it is shown, and
  the tab last chosen carries to the next sheet.
- **A board per daemon, by its route** (`src/tabs/tabs.ts`, `scenes/board.ts`): the tab strip, HTML along the top
  (`TABS` tall; the outliner starts beneath it), is Home — the `board` scene, route `[]` — then one tab per daemon
  opened (`OPEN` from a daemon's sheet, with `{ route, labels }`): a `Board` added at runtime as `board:<id>/<id>…`,
  given the route, with its own follow, nodes, camera and sheet, and `ui` brought back to the top. Switching sleeps the
  board shown (its sheet closes; its lists stay open), sets `ACTIVE` and wakes the target; ✕ stops and removes the
  board. Only the `ACTIVE` board writes `STATUS`, `OUTLINE` and `SELECTED` (`say` keeps the last of each and writes it
  again on wake) and takes `ARRANGE`, `FOCUS`, `HOVER` and pulses; `KINDS` and `LINES` are shared, set by the first;
  only Home launches `ui`; the backdrop follows the `ACTIVE` board's camera. **The route is on every request**: each
  frame is spread with `relayed(route)` (`lib/wire.ts`) — `{}` for `[]`, so Home's frames carry no `relay`, else
  `{ relay: route }` — in `world/live.ts`, `world/names.ts` and every sheet tab (`Opened.route`); our daemon takes the
  first ID off and relays the rest. The hex world (`world/snapshot.ts`, `world/watch.ts`) stays Home's.
- **The board is what opens** (`scenes/backdrop.ts`, `scenes/board.ts`, `src/grid/`): a dark Tron grid, every agent,
  standalone tool, connected standalone tool, dependency tool, account, volume, provider and daemon a glowing node in its kind's neon, first standing in a column
  per kind (`grid/place.ts`) and then wherever it is dragged — the links have no say, and positions live only for the
  page. Each way an event ran is a glowing line in the sender's neon with a chevron at the receiver, a mount a dashed
  green line (`grid/links.ts`); a provider is attached to every agent and tool deployed on it (`grid/edges.ts`, rule 2d), a daemon to every connected tool whose `daemon` (first hop) it is (rule 2f); a working agent or tool breathes a halo; a node is bright only while deployed (an account: connected; a connected tool: its connect held; a provider: `connected`; a daemon: `connected`; a volume: mounted into a deployed container), and the sheet's dot is that brightness; a container
  that is down dims its node (`grid/live.ts`); every rise in an event's count runs a pulse of light along its link
  (`grid/pulses.ts`, the events list kept open). The backdrop tiles the grid under the board's camera, as the sea does
  for the hex map. The hex map below (`Sea`, `World`) is kept, unlisted in `main.ts`.
- **Live lights** (`render/lights.ts`, `world/watch.ts`): the accounts, agents, tools, providers and daemons lists stay open after the
  snapshot, and each entity's state lights it: a pulsing green halo and a plate dot while an agent or tool is `active`
  (working), a steady halo for a `connected` account. The map itself is not rebuilt.
- **The board follows the daemon** (`world/live.ts`, `follow`): the eleven lists (agent templates, connected standalone tools, providers and daemons among them) kept open for the page's life, a list
  that reopens reconciled against what it held, the world handed to `scenes/board.ts` at most once a frame once every
  list was whole once. The board draws the first world, then diffs every later one by key (`apply`): new entities are
  nodes at `placeOne`, gone ones are removed (their sheet closed), kept ones relabelled; the edges are replaced
  (`links.setEdges`), the lights reapplied (`light`), the outliner's counts reset; pulses take the same events stream
  (`pulses.told`) after the world holding their line is drawn.
- **The hex world is the daemon drawn once** (`scenes/world.ts`, unlisted): eleven lists read to their `listed` (`world/snapshot.ts`),
  retried every 5 seconds until a read succeeds, never read again; then each tool's image is asked its
  `images info` once (`world/names.ts`), and a tool is named by its image's DIVERGE.md `name` where it said one. Every account, agent, standalone tool,
  connected standalone tool, dependency tool, volume, provider and daemon is an entity; every event a directed link, and every volume an agent mounts a dashed
  mount between them (`world/model.ts`). Each connected group is an island
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
  runner (the `tauri-proxy` feature): main.rs manages one `TauriProxy` and registers its eight commands, the four sends,
  `wire_took`/`wire_dropped` (a frame crosses to the page only after it took the last) and
  `wire_status`/`wire_reset`. The content security policy connects only to Tauri's IPC. The app's own commands that
  make daemon requests — `frontend_open`, `files_download`, `files_upload` — take `relay: Option<Relay>`
  (`diverge_sdk::shared::relay::Relay`), absent at Home, and put it on every frame they send (a frontend keeps it on
  `Served`).
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
**`FEATURES.md` lists every feature, in plain words, and is kept current:** a change that adds, removes or
alters one updates it in the same commit.
Plain words, few of them, tl;dr first. **GitHub: this work lives on the `viewer-edge-graph` branch**, pushed after
every chunk. **Nothing private enters this repo**, in files or commit messages.
