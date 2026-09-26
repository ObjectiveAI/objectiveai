# diverge-desktop — read this first

The Diverge desktop app, draft one. Maya (co-founder/CPO, designer, not a coder) owns the product;
Ronald owns everything below the daemon seam. This file is committed so every session and worktree
inherits it.

## The seam
- `src-tauri/src/daemon/mod.rs` — one trait, `Daemon`, one method per daemon verb. `StubDaemon` now;
  `WireDaemon` when Ronald ships a daemon. Meeting up with Ronald = writing `WireDaemon`, nothing else.
- The stub returns **Ronald's real `diverge_daemon_sdk` types**. Every conversion to our view types is
  an exhaustive `match` (never `_ =>`) so a new variant from him fails our build.
- The contract is `diverge-daemon-sdk-rs/src/endpoints/**` at `origin/p2p-provider-binary` — never a doc.
  `CONTRACT_PIN` holds the commit we built against. Start every session with the drift check in it and
  tell Maya in one line whether anything moved.
- `providers::{list, add, remove}` are **ours until the wire has them** (Maya: add a machine inside the app).
- The webview never holds a daemon connection, address or credential. Rust owns them.
- View types in `src-tauri/src/view.rs` generate `src/bindings/*.ts` (`cargo test -p diverge-desktop`).
- Never edit Ronald's crates. Blocked? Add to `QUESTIONS_FOR_RONALD.md` and route around it on the stub.

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

## Working with Maya
Layman's terms, few words, tl;dr first. Show screenshots, don't describe. Check the code before asking
her anything; never ask ideological or industry questions — resolve from code or make the call and note
it. **GitHub: the draft lives on the `user-experience` branch** of this public repo (Maya, 9/25: to show
progress and for cloud sessions). Push only there — never to Ronald's branches — and no PRs or issues unless she
says so. Nothing from the private Discord ideation goes in the repo. Discord read-only with her go-ahead.

## Run it
`pnpm install` (from the repo root) then `cd diverge-desktop && pnpm tauri dev`.

## Browser preview (reviews, cloud sessions — no Mac window needed)
`pnpm dev` and open http://localhost:1430 in any browser. Outside Tauri the page plays back
`src/preview/fixture.json`, a snapshot of what the stand-in daemon says (regenerate with
`cargo test -p diverge-desktop export_preview_fixture`). Nothing runs there; the rail says so.
