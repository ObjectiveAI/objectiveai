---
name: plugin-development
description: Develop an ObjectiveAI MCP plugin against a real laboratory host — register the local directory so an agent builds from it instead of a git tag, run the plugin's tools for real, and rebuild after each edit. Use when writing or debugging a plugin's tools, when an edit does not seem to have taken effect, or when preparing a plugin for release.
---

# Developing this plugin

`cargo run` compiles and binds a port. It cannot tell you anything else:
nothing dials the server, and `identity()`, `db::connect()` and
`command_executor()` are all empty outside a laboratory container. The
only way to exercise a plugin is to have an agent call it.

Development mode makes that loop fast by pointing the plugin's
coordinates at THIS directory, so the laboratory host builds from the
working tree instead of fetching a git tag.

## The loop

Read the coordinates first — they must match what an agent declares,
byte for byte:

- **name** — `[package] name` in `Cargo.toml`
- **version** — `[package] version`, `v`-prefixed when declared (`0.1.0`
  in Cargo.toml is `v0.1.0` to an agent)
- **owner** — the GitHub owner the plugin will be published under. It is
  not recorded anywhere in this directory; ask if it is not obvious.

```bash
# Once per machine. Development plugins ALWAYS run on the local host,
# and it is never auto-spawned.
objectiveai laboratories spawn

# Once per daemon lifetime.
objectiveai development plugins mcp create \
  --owner OWNER --name NAME --version vX.Y.Z \
  --path /absolute/path/to/this/directory
```

Then have an agent that declares the plugin call one of its tools. The
first call builds the image, which is slow — a full dependency compile.

After every edit:

```bash
objectiveai development plugins mcp reset --owner OWNER --name NAME --version vX.Y.Z
```

Then call again. Rebuilds reuse the caches declared in
`objectiveai.json`, so this is far cheaper than the first build.

## The one that will bite you

**An edit does nothing until you `reset`.** A registered plugin still
takes the image-exists fast path, so a call after an edit runs the OLD
image and looks like your change did not work. If a tool behaves like
code you already deleted, you skipped the reset.

## Everything else that fails at runtime rather than at build time

- **Registrations are in memory.** They live in the resident daemon and
  nowhere else — no file, no table. `daemon kill`, a crash, or a
  reinstall drops them, and the plugin silently goes back to fetching
  its git tag. Re-run `create`. Check with
  `development plugins mcp list`.
- **`--path` must be absolute.** A relative path is rejected: the
  laboratory host resolves it, and its working directory is not yours.
- **The version is exact.** `v0.1.0` and `0.1.0` are different keys, and
  a mismatch is silent — the plugin just builds from GitHub as though
  nothing were registered. This is the most common reason a registration
  "does not work".
- **No local host is an error, not a spawn.** If `laboratories spawn`
  has not been run for this machine and state, the call fails rather
  than starting one.
- **The port appears three times** — `mcp.port` in `objectiveai.json`,
  `PORT` in `src/main.rs`, and `EXPOSE` in the `Containerfile`. They must
  agree; a mismatch looks like a dead plugin rather than a misconfigured
  one.

## Build caches

`mcp.development.caches` in `objectiveai.json` lists CONTAINER paths
kept between rebuilds. They are bound in only in development mode and
ignored entirely for a released plugin.

Two rules, both of which fail in ways that do not look like their cause:

- **Copy anything the image must ship OUT of a cache within the same
  `RUN`.** A build mount exists only while that instruction runs and is
  never committed — the mount point is not even present in the finished
  image. Build into the cache, `cp` the product to a normal path, same
  `RUN`. Otherwise the build SUCCEEDS and the image is missing its
  binary.
- **Never cache a directory holding programs the build needs.** Caching
  `/usr/local/cargo` hides `/usr/local/cargo/bin/cargo` behind an empty
  mount and the first build dies with "cargo: not found". Name the
  subdirectories that accumulate — `registry`, `git`.

Add `--caches` to `reset` only when a cache is actually suspect (a
corrupt target dir, a changed toolchain). It makes the next build cold,
which is the thing development mode exists to avoid.

## Reading failures

- `no laboratory host is running for this machine/state` — run
  `laboratories spawn`.
- `requires the resident daemon` — the daemon is not running.
- `plugin development directory ...` — the registered path is gone, is
  not a directory, or is not absolute. Re-register.
- `plugin declares no MCP server` — `objectiveai.json` has no `mcp`
  block, or is not where the registration points.
- A tool that is absent from the agent's tool list — the plugin serves a
  tool set decided at startup; see `served_routes` in `src/main.rs`.

## Shipping

Plugins are built from a git tag. Commit, tag `vX.Y.Z`, push, then drop
the registration:

```bash
objectiveai development plugins mcp delete --owner OWNER --name NAME --version vX.Y.Z
```

Forgetting is survivable but worth avoiding: the image records the
directory it was built from, so the next run detects the mismatch and
rebuilds from the tag rather than serving uncommitted work under a
released version.

Before tagging, delete the `scaffold_*_deleteme` tools if any survive.
They are named to be impossible to ship by accident, and an agent that
can see one is looking at a plugin whose author never finished.
