# diverge-desktop — read this first

The Diverge desktop app, draft two. Maya (co-founder/CPO, designer, not a coder) owns the product;
Ronald owns everything below the daemon seam. This file is committed so every session and worktree
inherits it.

## The seams
- `src-tauri/src/daemon/mod.rs` — `Daemon`, one method per daemon verb (`agents::{create, delete, message, logs,
  list, edit}`). `StubDaemon` now; `WireDaemon` when Ronald ships a daemon.
- `src-tauri/src/machines.rs` — `Machines`: the provider protocol's volume verbs, addressed to one machine. Since
  `87015ef92` a volume is its provider's own; the daemon has none. How the app reaches a machine's verbs is an open question for Ronald. The
  stand-in daemon implements both seams (it knows every agent's mounts, so it keeps every machine's holds).
- Meeting up with Ronald = writing `WireDaemon` and `WireMachines`, nothing else.
- Everything returns **Ronald's real `diverge_sdk` types** (one crate since `d238949ba`, no features). Every
  conversion to our view types is an exhaustive `match` (never `_ =>`) so a new variant from him fails our build.
- The contract is `diverge-sdk-rs/src/{daemon,provider/endpoints/volumes,shared}/**` at `origin/p2p-provider-binary` —
  never a doc. `CONTRACT_PIN` holds the commit we built against. Start every session with the drift check in it and
  tell Maya in one line whether anything moved.
- `providers::{list, add, remove}` are **ours until the wire has them** (Maya: add a machine inside the app). Machine
  names are the app's (`machine_names.json`); the daemon's name stays underneath.
- **Mounts are remembered by the app** (`agent_mounts.json`): the daemon never reports an agent's mounts and
  `agents::edit` states them all anew, so the app offers "Mounts" only for agents it made.
- **Spaces** (`src-tauri/src/spaces/mod.rs`): the social layer on the provider protocol's tool rooms —
  `containers::tools::run` (host), `tools::connect` (join by id + invite), the run's `authorize` channel (the door), and
  the MCP exchanges (a room's verbs, resources, notifications). Ronald's types throughout (`Container`, `Connect`,
  `Id`, `Authorize`, rmcp's model). `StubSpaces` keeps rooms in process until his daemon relays them.
- **The agent door** (`src-tauri/src/door.rs`): the MCP server this app is for its agents — the Space verbs and
  `ask_person` (question / choice / credential by meaning; the value never reaches the agent). Cards wait on the person;
  nothing times out. The stand-in daemon's scripts reach it through `Piece::Door` steps.
- **The reporter** (`src-tauri/src/reporter.rs`): when an agent's run ends it `report`s to the person's Home.
- **The menu** (main.rs): ⌘N new agent, ⌘W closes a TAB (never the window), ⌘1 Home, ⌘2 Inbox; the page handles ⌘3–9
  and ⌘[ ⌘].
- The webview never holds a daemon connection, address or credential. Rust owns them.
- View types in `src-tauri/src/view.rs` generate `src/bindings/*.ts` (`cargo test -p diverge-desktop`).
- Never edit Ronald's crates. Blocked? Note it for Ronald in the private questions list (kept outside this repo) and
  route around it on the stub.

## Product laws (Maya's rulings)
- Style comes from the protocol site's tokens (Maya, 9/25): `diverge-provider-web/src/layouts/Layout.astro`
  on origin/main. Gold means ONLY "where you are". All colour goes through `src/theme.css`; users set themes later.
- Every action has two doors: all actions live in the Rust registry (`src-tauri/src/actions.rs`); no
  logic only in a click handler. The agent-facing door (MCP) comes right after draft one.
- No popups, quickstarts, tours, modals, or anything that takes the mouse.
- Never assume who someone is — show a provider only as the daemon names it.
- Don't lead with a feed. Lead with the work: agents, runs, files, machines.
- Nothing times out and a running agent cannot be stopped (`delete` refuses active; a message can only be
  taken back before delivery). No fake stop button. Ceilings are set at create time.
- No invented numbers. The stand-in daemon is labelled as one on screen.
- Words are tabled: every user-facing string lives in `src/strings.ts`.

## Screens (second draft)
Home (feed across Spaces + fleet; tabs are filters) · Inbox (agents and DM Spaces, one list) · You (receipts attributed
to their Space, shows, agents, machines) · Spaces (the door, hosted, joined, host, join) · a Space (feed, verbs as
generated forms, members, charter, invite; the joiner's door on first entry) · Storage (volumes) · Machines · Views ·
New agent · a conversation (work folds into one line; ask cards inline; Mounts, changed while it's idle).
Storage is per machine: pick a machine, then a volume; each volume is in one of Ronald's three modes (keeps changes /
fresh each run / read only).

## Working with Maya
Layman's terms, few words, tl;dr first. Show screenshots, don't describe. Check the code before asking
her anything; never ask ideological or industry questions — resolve from code or make the call and note
it. **GitHub: drafts live on the `user-experience` branch** of this public repo, rebased onto Ronald's latest when
pushed (Maya, 9/26: "rebase onto the new sdk then push to user-experience"). Push only when she says. Never push to Ronald's branches; no PRs or issues
unless she says so. **Private material never enters this repo**, in files or commit messages: planning docs,
questions lists, quotes from private chats, and wording or ideas that came from them. Seed content is invented and
everyday.

## Run it
`pnpm install` (from the repo root) then `cd diverge-desktop && pnpm tauri dev`.

## Browser preview (reviews, cloud sessions — no Mac window needed)
`pnpm dev` and open http://localhost:1430 in any browser. Outside Tauri the page plays back
`src/preview/fixture.json`, a snapshot of what the stand-in daemon says (regenerate with
`cargo test -p diverge-desktop export_preview_fixture`). Nothing runs there; the rail says so.
