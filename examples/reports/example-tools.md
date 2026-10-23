# Example tool containers

Four tool containers to build as the first Diverge examples. Each is
one image whose program is an MCP server with several tools, and each
leans on a different part of what a tool container can do:

| example | what it shows | mode |
|---|---|---|
| `courier` | a tool acting on the daemon as an account: messaging other agents | transient |
| `taskboard` | one shared instance used by many agents at once, told apart by who calls | service |
| `memory` | the caller's own Postgres database, kept per agent | transient |
| `workspace` | a volume mounted into the container as files | transient |

None of this has been built. The design rests on what the SDK states
today, cited below, and on two facts worth repeating up front: no tool
container has yet run live, and the daemon has no image-build
endpoint, so these images are built through the provider protocol's
`images.build` until it does.

## What every tool container has

These come from `diverge-sdk-rs/src/container_proxy/inside/` and the
daemon's `standalone_tools` family.

- **An MCP server.** The program is an HTTP server on the container's
  loopback at the port `inside::port()` names. The proxy beside it is
  its only caller: `POST /register` once (the arguments and the
  template id, fixed for the container's life), `POST /secrets` and
  `POST /secrets/collect` around every activation, and MCP over
  Streamable HTTP at `/mcp` (`inside::tool`).
- **The caller, attested.** Every `list_tools`, `list_resources`,
  `call_tool` and `read_resource` the tool receives carries, under
  `_meta`, the daemon's own word on who is calling
  (`shared::mcp::attestation::Caller`): an agent of the daemon's
  (`kind: agent`, its image, template, index and name) or a client
  over a connect (`kind: unbrokered`, the identity the daemon
  authorized). It is never what the caller says of itself, so a tool
  can key state and permissions on it.
- **The daemon.** A tool container reaches the whole daemon protocol
  through the proxy, at `/daemon` on the loopback
  (`inside::daemon`). It acts as the named account the tool was
  created under (`standalone_tools.create`'s `account`), and the
  daemon judges each request by that account's roles. An agent
  container has no such access; a tool does.
- **Postgres.** The proxy is a pgwire listener on the loopback that
  carries every connection to the caller's own database
  (`inside::postgres`); the program's driver dials it like any
  Postgres.
- **Volumes and secrets.** A tool is made of what an agent is made
  of: a provider with volume mounts, FUSE mounts of other providers'
  volumes, and the secrets its template names, fed in at activation
  and never shown to the agents that call it.
- **Two ways to run.** A tool template says `transient` (the
  container runs while something uses it) or `service` (kept running
  while any agent is attached or any connect is open, retired after
  `idle_seconds` with no holds). A service is one container shared
  by every agent attached to it.

## `courier`: messaging other agents

A tool through which an agent writes to another agent. It is the one
example that acts on the daemon itself.

**How it works.** The tool is created under an account whose roles
let it list agents and message them. When an agent calls `send`, the
courier reads the attested caller from `_meta`, puts a sender line in
front of the text, and calls `agents.message` on the daemon for the
recipient. The message lands in the recipient's conversation as user
content, so the recipient sees it on its next turn, from a sender it
can trust: the line is written from the daemon's attestation, not
from anything the calling agent claimed.

**Tools.**

| tool | does |
|---|---|
| `directory()` | lists the agents the courier's account can reach: name, template and index, whether each is active, when each was last active |
| `send(to, text)` | messages one agent, prefixed with the attested sender; answers whether the message was delivered or is waiting |
| `broadcast(tag, text)` | messages every agent carrying a tag, and reports each delivery |
| `status(to)` | whether an agent is working or idle, and when it was last active, without messaging it |

**Arguments.** Optional: a list of tags the courier may broadcast to,
and whether to deliver to agents that are not active (otherwise the
message queues as the daemon queues it).

**Open questions.**
- The account's roles are the whole permission model: which grants
  the example's setup creates (an `agents` list and `message` grant,
  narrowed by tag or not) should be written down in the example.
- A reply is just another `send`. Whether the courier should also
  offer an `inbox` of messages received is a design choice; the
  simple answer is no, since the recipient's own conversation already
  holds them.

## `taskboard`: one tool, shared by the agents that use it

A shared board of tasks. It runs as a service, so every agent
attached to it talks to the same container and sees the same state.
The attested caller is what tells the agents apart.

**How it works.** The board lives in the container's memory, or in
Postgres for a board that survives the service being retired. Every
mutating call records the attested caller, so `claim` knows who holds
a task and `complete` can refuse anyone but the holder.

**Tools.**

| tool | does |
|---|---|
| `post(title, detail)` | adds an open task; answers its id |
| `list(state?)` | the board, or the open, claimed or done tasks, each with who claimed or finished it |
| `claim(id)` | takes an open task; atomic, so two agents can never hold the same one |
| `complete(id, result)` | marks the caller's task done with its result |
| `release(id)` | gives the caller's task back to open |
| `comment(id, text)` and `thread(id)` | a running discussion on one task |

**Arguments.** Optional: whether only the poster may delete a task,
and whether the board persists to Postgres.

**Why this one.** It shows the thing a service tool is for:
coordination among agents through one shared object. Two alternatives
for the same slot were considered: a `ballot` (propose, vote, tally)
and a two-player `chess` game between agents. The task board shows
the most, since it uses ownership as well as shared state.

## `memory`: a notebook on the caller's database

A persistent notebook. It uses Postgres through the proxy, and keys
every note by the attested caller, so each agent has its own notes,
with an area the agents may share on purpose.

**How it works.** On registration the program creates its table if
absent. Each note holds its owner (the attested caller), its text,
its tags and when it was made. `recall` is a text search (Postgres
full-text search to start; embeddings later, if ever).

**Tools.**

| tool | does |
|---|---|
| `remember(text, tags?)` | keeps a note for the caller; answers its id |
| `recall(query, tags?, shared?)` | the caller's notes that match, best first, and the shared ones if asked |
| `forget(id)` | deletes one of the caller's notes |
| `share(id)` | moves one of the caller's notes into the shared area, still marked as theirs |

**Open question.** Whether a connect client (`kind: unbrokered`)
gets its own notebook keyed by its identity, or only agents do. The
simple answer is yes: the attestation gives either kind a stable key.

## `workspace`: files on a mounted volume

A file desk. The tool is created with a volume mounted into its
container, and gives the agents that use it a small, safe set of file
operations on it.

**How it works.** The mount path is a fixed directory in the
container; every path a tool takes is resolved under it, and one that
would leave it is refused. Whether writes stick depends on the
volume's mode (persistent, ephemeral or read only), which the
volume, not the tool, decides; `write` on a read-only volume answers
that it cannot.

**Tools.**

| tool | does |
|---|---|
| `tree(path?, depth?)` | the directory tree under a path |
| `ls(path)` | one directory's entries, with sizes and times |
| `read(path, from?, to?)` | a file's text, or a range of lines |
| `write(path, text)` | writes a file whole, making its directories |
| `grep(pattern, path?)` | lines matching a pattern, with file and line number |

**Open question.** As a transient tool each agent's use starts the
container; as a service, several agents share one desk. Both are
useful; the example ships transient and says how to make it a
service.

## How the examples are laid out

Each example is a directory under `examples/`: a Rust crate whose one
dependency from this repository is the SDK, a `Containerfile`, and a
`DIVERGE.md` stating the arguments, as the agent containers have
theirs. They sit in the repository so the SDK is a path dependency and
the build context can hold it.

What is unsolved is building from a volume. A provider build sees one
directory of one volume as its context and nothing above it, so the
SDK has to be inside that context: uploaded beside the example, a git
dependency, or a published crate. That is decided before the first
image is built.

## Order

1. `memory`: the smallest surface (Postgres and the attestation), and
   the first live run of a tool container.
2. `workspace`: adds a volume mount.
3. `taskboard`: adds service mode and several agents at once.
4. `courier`: adds the tool's own account and the daemon access, the
   most moving parts.
