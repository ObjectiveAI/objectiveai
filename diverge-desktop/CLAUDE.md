# diverge-desktop — read this first

The Diverge desktop app, draft four. Maya owns the product and its words; Ronald owns everything below the
daemon seam. This file is committed so every session and worktree
inherits it.

## The seams
- `src-tauri/src/daemon/mod.rs` — `Daemon`, one method per daemon verb (`agents::{create, delete, message, logs,
  list, edit}`, `tools::{create, edit, connect, attach, detach, delete, list}`). `StubDaemon` now; `WireDaemon` when
  Ronald ships a daemon. The app doesn't use the tools verbs for rooms: a daemon-attached tool bypasses the app, which
  seals agents' calls and asks the person first.
- `src-tauri/src/machines.rs` — `Machines`: the provider protocol's volume verbs, addressed to one machine. Since
  `87015ef92` a volume is its provider's own; the daemon has none. The wire doesn't say yet how the app reaches a machine's verbs.
  The stand-in daemon implements both seams (it knows every agent's mounts, so it keeps every machine's holds).
- Meeting up with Ronald = writing `WireDaemon` and `WireMachines`, nothing else.
- Everything returns **Ronald's real `diverge_sdk` types** (one crate since `d238949ba`, no features). Every
  conversion to our view types is an exhaustive `match` (never `_ =>`) so a new variant from him fails our build.
- The contract is `diverge-sdk-rs/src/{daemon,provider/endpoints/{volumes,containers/tools},shared,container_proxy/inside}/**` at `origin/p2p-provider-binary` —
  never a doc. `CONTRACT_PIN` holds the commit we built against. Start every session with the drift check in it and
  tell Maya in one line whether anything moved.
- `providers::{list, add, remove}` are **ours until the wire has them** (Maya: add a machine inside the app). Machine
  names are the app's (`machine_names.json`); the daemon's name stays underneath.
- **Mounts are remembered by the app** (`agent_mounts.json`): the daemon never reports an agent's mounts and
  `agents::edit` states them all anew, so the app offers "Mounts" only for agents it made.
- **Spaces** (`src-tauri/src/spaces/mod.rs`): the social layer on the provider protocol's tool rooms —
  `containers::tools::run` (host), `tools::connect` (join by id), the run's `authorize` channel (the host's yes), the
  MCP exchanges (a room's verbs, resources, notifications), the container's files (a room's table) and `transfer`.
  Ronald's types on the wire; what rides inside them is ours (below). `StubSpaces` runs the real room program in
  process until rooms are hosted on the wire.
- **The person layer is ours, not Ronald's** — the wire gives a room no way to tell members apart (a tool program sees
  one MCP client; a runner sees an address and an opaque string), and nothing on the wire names a person:
  - `diverge-desktop/room` (crate `diverge-desktop-room`): the room program.
    - A room's settings (`Args`) are signed by its host, and its id is a label plus the host's mark (`room_id`), so
      no other host can run a room by that id.
    - Every call is sealed under the `_meta` key `network.diverge.desktop/seal` for that id, and checked against keys
      the host admitted. Someone let in unlisted is a mark of their key (`key_mark`) until they first act.
    - Every move is chained and countersigned by the room's own key, which the host's app made and named in the
      settings. What a move says must be what its sealed verb and arguments make.
    - A `Record` (settings, the record it continues, moves) is checked only by replaying it under the rules a live
      call meets (`Room::check`, `Room::from_record`). Restarts, successors and copies all take that path;
      `tests/attacks.rs` keeps the attacks it has to refuse.
    - With `--features image` it builds the tool container program (`/register`, `/schema`, `/mcp` over Streamable
      HTTP, as `container_proxy::inside::tool` states). `/register` takes the settings, the room's key
      (`room_secret`) and any record, and refuses a damaged one. rmcp lifts `_meta` into the request context, so the
      program re-attaches it before checking the seal. It never waits on its host while holding the room.
  - `src-tauri/src/identity.rs`: your usual self and fresh personas, and your agents' keys tethered to the persona they
    act for, named plainly in a fresh name's rooms. One owner-only file (`identity.json`), written whole and swapped
    in, with a backup kept beside it; never the keychain (it can prompt in a popup). If it and its backup are both
    unreadable, the app signs nothing and says so; it never makes new keys over them. Counters live in
    `counters.json` and never fall behind the clock. One call at a time per key per room (`Identity::turn`).
  - A knock's opaque authorization carries a `Knocking`, signed by the key it names, for one room, good for a day,
    with a mark of the invite it came with. A vouch names the room it vouches someone into and runs out after a week.
    An invite is text (`diverge-invite:…`) carrying the room's rules and verbs, checked against the room on entry.
  - `records/`: the app keeps a replayed copy of every room's record, named for a digest of the room's id. An
    unreachable room shows from it, and someone still in it can continue it.
- **Hires** (`src-tauri/src/hires.rs`): a visitor asks one of your agents for something through your profile room; the
  room calls its host (on the wire, its own `mcp-call-tool`); you decide on a card. A hire waits until its agent is
  free, reaches it quoted as the visitor's words, and its result is read from its own run alone
  (`reporter::run_after`). It lands on the profile's table. A hire asked while the app was closed is picked up from
  the profile's record.
- **The agent door** (`src-tauri/src/door.rs`): the MCP server this app is for its agents — the Space verbs, the table,
  open asks, and `ask_person` (question / choice / credential by meaning; the value never reaches the agent). Every
  room move is sealed as the agent. Reading a room and acting in one both ask its person first, unless the person set
  a daily allowance for that agent in that room, for that kind of move (`Reach`: read, talk, work, pledge;
  `allowances.json`). An allowance is spent only on what the room accepts. A card carries the whole call
  (`CardCall`, `CardHire`), worded by the screen from `src/strings.ts`. Cards wait on the person; nothing times out.
- **The reporter** (`src-tauri/src/reporter.rs`): when an agent's run ends it `report`s to the person's Home, with the
  run's words only while Home is theirs alone.
- **The stand-in keeps its rooms** between launches (`tables/.stand-in-rooms.json`), rebuilt by replaying their
  records.
- **The menu** (main.rs): ⌘N new agent, ⌘W closes a TAB (never the window), ⌘1 Home, ⌘2 Inbox; the page handles ⌘3–9
  and ⌘[ ⌘].
- The webview never holds a daemon connection, address or credential. Rust owns them.
- View types in `src-tauri/src/view.rs` generate `src/bindings/*.ts` (`cargo test -p diverge-desktop`).
- Never edit Ronald's crates. Blocked? Note it for Ronald in the private questions list (kept outside this repo) and
  route around it on the stub.

## Product laws (Maya's rulings)
- **The app sets nothing for anyone.** Every permission on screen says who set it (you over your own agents, a host
  over their room, whoever runs a machine over it, the network's own rules as plain facts), and only its setter can
  change it. The app ships starting positions for your own settings and says so; it adds no rules of its own.
- A claim reaches a screen only once a test proves it.
- Style comes from the protocol site's tokens: `diverge-provider-web/src/layouts/Layout.astro` on origin/main.
  Gold means ONLY "where you are". All colour goes through `src/theme.css`, which is the theme, and every
  words-on-background pair keeps 7:1 (`src/theme.test.ts`).
- Every action has two doors: all actions live in the Rust registry (`src-tauri/src/actions.rs`); no
  logic only in a click handler. The agent-facing door is the MCP server in `src-tauri/src/door.rs`.
- No popups, quickstarts, tours, modals, or anything that takes the mouse.
- Never assume who someone is — show a provider only as the daemon names it.
- Don't lead with a feed. Lead with the work: agents, runs, files, machines.
- Nothing times out and a running agent cannot be stopped (`delete` refuses active; a message can only be
  taken back before delivery). No fake stop button. Ceilings are set at create time.
- No invented numbers. The stand-in daemon is labelled as one on screen.
- Words are tabled: every user-facing string lives in `src/strings.ts`.

## Screens (draft four)
Home (feed across Spaces + fleet; tabs are filters; an ask goes to several rooms and is followed as one thread; take
an offer, or close the ask everywhere) · Inbox (agents and direct rooms, one list) · You (your names, your profile
room's hires and notes, receipts once each, credited to the room that issued them) · Spaces (knocks that check, with
notes and vouches; hosted, joined; host with an open door or not; paste an invite) · the door (an invite read before
knocking: rules and who set them, verbs, what the host learns about you; appear as your usual name or a fresh one; a
vouch) · a Space (what's happened and the table; verbs in plain words; the host's own tools: edit the rules,
recommend a room, remove someone quietly or not, restart; members with whose agent each is; your agents' allowances
by kind; the people from before, in a room you continued; an unreachable room from your copy, with how old it is and
"continue it") · Storage (volumes) · Machines · Views · New agent · a conversation (work folds into one line; cards
show the whole call; Mounts, changed while it's idle, with scratch space you choose).
Storage is per machine: pick a machine, then a volume; each volume is in one of Ronald's three modes (keeps changes /
fresh each run / read only).

## Working on it
Plain words, few of them, tl;dr first. Show screenshots, don't describe. Check the code before asking Maya
anything; settle protocol and industry questions from the code, or make the call and note it. **GitHub: drafts live
on the `user-experience` branch** of this public repo, rebased onto Ronald's latest, and are pushed only when Maya
says. Never push to Ronald's branches; no PRs or issues unless asked. **Nothing private enters this repo**, in files
or commit messages: planning docs, lists of questions, quotes from outside conversations, and wording or ideas that
came from them. Seed content is invented and everyday.

## Run it
`pnpm install` (from the repo root) then `cd diverge-desktop && pnpm tauri dev`.

## Browser preview (reviews, cloud sessions — no Mac window needed)
`pnpm dev` and open http://localhost:1430 in any browser. Outside Tauri the page plays back
`src/preview/fixture.json`, a snapshot of what the stand-in daemon says (regenerate with
`DIVERGE_EXPORT_PREVIEW=1 cargo test -p diverge-desktop export_preview_fixture`; a plain `cargo test` leaves it alone). Nothing runs there; the rail says so.
