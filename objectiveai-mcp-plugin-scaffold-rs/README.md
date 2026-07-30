This is an [ObjectiveAI](https://objectiveai.dev) MCP plugin — the
server half.

## Where the plugin root is

Everything below registers **the directory holding
`objectiveai.json`**, which is either:

- **this directory**, when this half stands alone; or
- **the parent**, when this is the `mcp/` half of a full plugin (what
  the root `scaffold.sh` produces, laying down `mcp/` + `viewer/` under
  one manifest). Both halves then register that same path.

The manifest's `mcp.containerfile` is resolved from the root, and a
containerfile's own directory is its build CONTEXT — so this half's
`Containerfile` sees this directory either way, and its `COPY` steps
never carry an `mcp/` prefix.

## Getting Started

If you scaffolded with `scaffold.sh`, the name is already yours — taken
from the directory you ran it in — and you can skip to registering.
Otherwise give it one first:

```bash
./rename.sh my-plugin
```

That rewrites the base name where it appears — the package, the binary,
the `cp` in the `Containerfile`, and the `NAME` constant. Do it before
anything else; a half-finished rename fails at runtime rather than at
build time.

Then point ObjectiveAI at it:

```bash
objectiveai laboratories spawn
objectiveai development plugins mcp create \
  --owner you --name my-plugin --version v0.1.0 \
  --path /absolute/path/to/the/plugin/root
```

Now an agent that declares `you/my-plugin@v0.1.0` gets THIS working
tree instead of the git tag — no pushing, no tagging. After each edit:

```bash
objectiveai development plugins mcp reset --owner you --name my-plugin --version v0.1.0
```

Rebuilds are explicit, so `reset` is what makes an edit take effect.
It is fast because the `mcp.development.caches` in `objectiveai.json`
keep the cargo registry and `target/` between builds; drop `--caches`
into that command only when a cache is actually suspect.

`cargo run` also works and is the quickest check that the thing
compiles and binds. It just cannot tell you much: nothing dials it, and
`identity()`, the database and the command executor are all empty
outside a container.

You can start adding tools by editing `src/main.rs`. Each `#[tool]`
becomes a tool an agent can call, and its parameter struct becomes the
schema the agent sees:

```rust
#[rmcp::tool(description = "Greets someone by name.")]
async fn greet(&self, Parameters(args): Parameters<GreetArgs>) -> String {
    format!("Hello, {}!", args.name)
}
```

Put anything your tools share — clients, handles, configuration — on
`Plugin`. Every tool receives it as `&self`.

The seven tools already in there are named `scaffold_*_deleteme`. The
names are deliberately unmissable: they are there to be read once and
removed, and an agent that can see a `..._deleteme` is looking at a
plugin whose author never got to writing their own. Delete them as soon
as you have one of your own.

## Using the database

The database is an OPT-IN: `"postgres": true` in the manifest's `mcp`
block (this scaffold opts in). Only then does the host give the
container a Postgres — relayed through a proxy on the container's own
loopback — so a plugin never configures one, it asks:

```rust
let pool = db::connect(Default::default()).await?;
```

That connects at most once no matter how many tools call it, shares one
attempt between overlapping callers, and does not cache a failure — a
proxy that was not up yet will not poison the process. A plugin whose
manifest says `"postgres": false` gets a distinct `db::Error::NoDatabase`
from `connect`, naming the opt-in it is missing.

`scaffold_note_write_deleteme` and `scaffold_note_read_deleteme` are a
round trip through it. Three things in them are worth keeping when you
delete the rest:

- **The database is the daemon's, not yours alone.** You share it with
  ObjectiveAI's own tables and with every other plugin. So own a
  distinctly named table, and scope rows by
  `identity().agent_instance_hierarchy` — the next container over is a
  different agent reading the same table.
- **`sqlx::query`, not the `sqlx::query!` macro.** The macro validates
  SQL against a live database at COMPILE time, which would make your
  plugin unbuildable without one. The database it will talk to does not
  exist until a host creates its container.
- **Cast timestamps to text in the query.** Decoding a `TIMESTAMPTZ`
  into a real time type needs sqlx's `chrono` or `time` feature, which
  the framework does not enable; `written_at::text` works with the sqlx
  you have.

The table is created on first use, guarded by a `OnceCell`. A plugin
container is ephemeral and there is nowhere to run a migration.

## Asking a person something

`scaffold_credential_call_deleteme` is the longest example, and the one
worth reading if your plugin ever needs a human in the loop. It calls a
URL with a credential, and gets the credential by asking — through a
viewer channel — but only when it has to:

1. Look in the database, scoped to this agent. If a credential is
   there, skip to step 5. **This is what makes it conditional**: the
   first call may wait on a person, and no later call does.
2. `channels publish` offers a channel and BLOCKS until a viewer
   accepts. Uncapped without a timeout, so it has one. Both waits use
   the base `--timeout`, which the daemon turns into a whole-stream
   deadline over any command — a plugin never needs its own clock.
3. `channels logs subscribe` waits for the reply — one call, no
   cursor. A publisher's reads are scoped to `reply` entries, so the
   offer it just published never comes back as its own answer.
4. Entries are envelopes; the content is behind `channels logs open`.
   Then the channel is closed, on the success and failure paths alike —
   a channel left open is a user surface left waiting.
5. Call the endpoint with the credential as `Authorization`, and return
   the status and body.

**Failures come back as a tool RESULT, not a protocol error**, carrying
which step broke:

```rust
CallToolResult::structured_error(json!({ "step": "subscribe", "error": ... }))
```

That distinction is worth keeping. A protocol error means "this call
was malformed". Everything above is the call working correctly and the
world not cooperating — nobody accepted, the person cancelled, the
endpoint rejected the credential — which is something an agent can
reason about and retry. The HTTP status is reported rather than
enforced for the same reason: a 401 is a real answer, and turning it
into an error would hide the one result that says the credential has
gone stale.

## Tools that come and go

A plugin does not have to serve the same tools for its whole life, or
the same tools as the next copy of itself. `served_routes` in
`src/main.rs` is the whole mechanism, and the two example tools past
`greet` and `whoami` exist to demonstrate it:

- **`scaffold_switch_deleteme` only exists if the agent asked for it.**
  It is served only when `arguments()` has `switch` set to the JSON
  boolean `true`. Absent, `null`, `0`, the string `"true"` — all off,
  because a plugin that guesses what someone meant will one day guess
  a feature on that should have stayed off. Nothing at runtime can
  talk it into existing either: arguments are stamped at container
  create and never rewritten.
- **`scaffold_switched_deleteme` starts off**, and appears when the
  first tool is called. `Tools::replace` swaps the served set and sends
  `notifications/tools/list_changed`, so a client that re-lists on that
  notification sees the new set.

Both are declared with `#[tool]` exactly like any other. "Conditional"
is not a property of a tool — it is a property of the list you build:

```rust
fn served_routes(switched_on: bool) -> Vec<ToolRoute<Plugin>> {
    let has_switch = arguments()
        .get(SWITCH_ARGUMENT)
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    Plugin::tool_router()
        .into_iter()
        .filter(|route| /* decide, by name */)
        .collect()
}
```

Filtering the full router rather than assembling routes by hand keeps
the macros as the single declaration of what a tool IS. And the served
list is the only state — asking `tools.routes()` what is currently
served beats keeping a flag that can disagree with it.

## What you get for free

The framework handles everything that is the same for every plugin, so
none of it is in your source:

- **`identity()`** — who the host says this plugin is running as.
- **`arguments()`** — what the agent configured for this plugin.
- **`db::connect()`** — the Postgres your container gets, connected
  once no matter how many callers ask.
- **`command_executor()`** — run ObjectiveAI CLI commands, executed by
  the host on your behalf.
- **`Tools::replace()`** — change which tools you expose while you are
  serving; the agent is told to re-list.

## Shipping it

Plugins are built from a git tag. Push one, drop the development
registration (`development plugins mcp delete`), and the next run
fetches the tag instead — the image records which directory it was
built from, so it rebuilds on its own rather than serving your
uncommitted work under a released version.

The port appears in three places — `mcp.port` in the root
`objectiveai.json`, `PORT` in `src/main.rs`, and `EXPOSE` in the
`Containerfile`. They have to agree;
the host publishes whichever one the manifest names, so a mismatch
looks like a dead plugin rather than a misconfigured one.

Commit `Cargo.lock`. The image builds `--locked`, because a plugin that
resolves differently on two days is not really a plugin build.

## A note on the name

Your plugin's name is not decoration. ObjectiveAI derives a routing
prefix from it and prepends `<prefix>_` to every tool an agent sees, so
`greet` would show up as `my-plugin_greet`. Two plugins sharing a name and
version fall back to a positional index, which is legal and unreadable.
Pick something distinctive.

## Adding a viewer

This is the MCP half. A plugin can also contribute tabs and components
to the ObjectiveAI viewer by declaring a `viewer` half in the same
`objectiveai.json` — its own containerfile, built in its own directory.
The two are independent; neither build sees the other.

## For agents

`.agents/skills/plugin-development/SKILL.md` teaches a coding agent this
whole loop — the coordinates to read, the reset that makes an edit take
effect, and the failures that look like something else. It travels with
this directory, so an agent working in your plugin picks it up with no
setup.

## Learn More

- [ObjectiveAI](https://objectiveai.dev) — what all of this is for.
- [`objectiveai-mcp-plugin-framework`](https://docs.rs/objectiveai-mcp-plugin-framework)
  — the API reference for everything above.
- [Model Context Protocol](https://modelcontextprotocol.io) — the
  protocol your plugin speaks.
- [`rmcp`](https://docs.rs/rmcp) — the MCP implementation, including
  the `#[tool]` and `#[tool_router]` macros.
