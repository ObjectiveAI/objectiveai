# objectiveai-plugin-scaffold

An [ObjectiveAI](https://objectiveai.dev) plugin with both halves under
one manifest.

```
.
├── objectiveai.json   the ONE manifest — both halves, at the root
├── mcp/               the MCP server (Rust): tools an agent calls
├── viewer/            the viewer extension: tabs, channel handlers, browser scripts
└── .agents/skills/    skills for the coding agent working on this
```

**One directory is registered for both halves: this one.** The manifest
lives here, `mcp.containerfile` and `viewer.containerfile` are resolved
from here, and each containerfile's own directory is its build context —
so `mcp/` and `viewer/` build independently without either seeing the
other.

## Getting started

The name is already yours: `scaffold.sh` took it from the directory you
ran it in, and wrote it into the Cargo package, the binary, the
lockfile, the `Containerfile`, the MCP server's `NAME`, and
`package.json`. That name is not decoration — ObjectiveAI derives the
routing prefix from it, so a tool called `greet` reaches an agent as
`<name>_greet`.

Pick an owner (the GitHub account this will publish under) and register
BOTH halves against this directory:

```bash
# Once per machine — development plugins always run on the local host,
# and it is never auto-spawned.
objectiveai laboratories spawn

objectiveai development plugins mcp create \
  --owner you --name <name> --version v0.1.0 --path "$PWD"
objectiveai development plugins viewer create \
  --owner you --name <name> --version v0.1.0 --path "$PWD"
```

The trio must match BYTE FOR BYTE between the two, and match what an
agent declares — `v0.1.0` and `0.1.0` are different keys, and a mismatch
is silent: the plugin just builds from GitHub as though nothing were
registered.

Then, in a second terminal, start the viewer's watch build:

```bash
cd viewer && pnpm install && pnpm run dev
```

Now have an agent that declares `you/<name>@v0.1.0` call one of the
tools. The first call builds the MCP image, which is slow; later ones
reuse the caches named in `objectiveai.json`.

## The two loops are different

| | MCP half | viewer half |
|---|---|---|
| after an edit | `development plugins mcp reset …`, then call again | nothing — the watch build writes, the viewer reloads |
| why | the image-exists fast path serves the OLD image until you reset | the viewer watches the built output and hot-reloads open tabs |

**An MCP edit does nothing until you `reset`.** If a tool behaves like
code you already deleted, that is why.

## Shipping

Plugins are built from a git tag. Commit, tag `vX.Y.Z`, push, then drop
both registrations:

```bash
objectiveai development plugins mcp delete --owner you --name <name> --version vX.Y.Z
objectiveai development plugins viewer delete --owner you --name <name> --version vX.Y.Z
```

An agent declaring `you/<name>@vX.Y.Z` then gets the tag. The image
records which directory it was built from, so a forgotten registration
self-heals — it rebuilds from the tag rather than serving uncommitted
work under a released version.

Before tagging, delete the `*_deleteme` tools, tabs and scripts if any
survive. They are named to be impossible to ship by accident.

## Where to read next

- `mcp/README.md` — writing tools, the database, the command executor
- `viewer/README.md` — tab bundles, browser scripts, the mailbox bridge
- `.agents/skills/` — the same ground for a coding agent, plus how to
  drive and inspect agents from the CLI while testing
