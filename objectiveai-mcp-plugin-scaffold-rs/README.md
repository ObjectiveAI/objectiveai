This is an [ObjectiveAI](https://objectiveai.dev) MCP plugin, scaffolded
from `objectiveai-mcp-plugin-scaffold-rs`.

## Getting Started

First, give it a name:

```bash
./rename.sh my-plugin
```

That rewrites the base name in the four places it appears — the
package, the binary, the `cp` in the `Containerfile`, and the `NAME`
constant. Do it before anything else; a half-finished rename fails at
runtime rather than at build time.

Then run it:

```bash
cargo run
```

The server starts on port `8080`. Nothing will call it — inside
ObjectiveAI the laboratory host is the client and dials in — but a
clean start means the build is good.

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

The two tools already in there are called
`scaffold_greet_deleteme` and `scaffold_whoami_deleteme`. The names are
deliberately unmissable: they are there to be read once and removed,
and an agent that can see a `..._deleteme` is looking at a plugin whose
author never got to writing their own. Delete them as soon as you have
one tool of your own.

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

Plugins are built from a git tag. Push one, then have an agent install
and call it — that is the real test loop, and the only one where
`identity()` is populated, since the host stamps it at container
create.

The port appears in three places — `objectiveai.json`, `PORT` in
`src/main.rs`, and `EXPOSE` in the `Containerfile`. They have to agree;
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

## Learn More

- [ObjectiveAI](https://objectiveai.dev) — what all of this is for.
- [`objectiveai-mcp-plugin-framework`](https://docs.rs/objectiveai-mcp-plugin-framework)
  — the API reference for everything above.
- [Model Context Protocol](https://modelcontextprotocol.io) — the
  protocol your plugin speaks.
- [`rmcp`](https://docs.rs/rmcp) — the MCP implementation, including
  the `#[tool]` and `#[tool_router]` macros.
