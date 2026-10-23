# Example tool containers

Four tool containers to build as the first Diverge examples. Each is
one image whose program is an MCP server with several tools, and each
leans on a different part of what a tool container can do:

| example | what it shows | mode |
|---|---|---|
| `courier` | an inbox between agents: Postgres for the mail, the daemon to notify | service, global |
| `taskboard` | one shared board used by many agents at once, told apart by who calls | service, template |
| `memory` | a notebook in the tool's Postgres database | transient |
| `workspace` | a volume mounted into the container as files | transient |

`courier`, `memory` and `taskboard` are built; `workspace` is not. The design rests on what the SDK states
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

## `courier`: an inbox between agents

**Built:** `examples/courier/` (crate `diverge-example-courier`, binary `courier`), with its `Containerfile` and `DIVERGE.md`. It is a **dependency tool** declared by agents' images (a `service` with `scope: global`, grants `self` and `never_active` over agents), keeps mail per template, has a fifth tool `list_agents` listing whom the caller can write to (attached, then never-run), and a send whose notification cannot be sent fails and keeps nothing.

Mail between agents. Every agent that uses the courier has an inbox
of conversations; an agent opens a conversation with another agent on
a topic, or answers inside one it already has. The courier never puts
a message's words into an agent's own conversation. It only tells the
recipient that something arrived, and the recipient opens its inbox
when it chooses: the agent pulls its mail, nothing is pushed.

**The notification.** Whenever a message is added to a conversation,
the courier sends every other participant, through the daemon's
`agents.message`, exactly this and nothing else:

```text
<courier>
You have a new message
</courier>
```

It names no sender, no topic and no conversation. To learn more the
agent calls the courier's listers. The courier acts on the daemon as
the account the tool was created under, so that account needs a
grant to message the agents that use it.

**Conversations.** A conversation is between two agents, the one who
opened it and the one it was opened with. It has an id, the topic it
was opened on, and its messages in order. The topic is set when the
conversation is opened and never changes. Each message records its
sender, which is the attested caller of the call that sent it (the
daemon's word, never what the sending agent claims), its text and
when it was sent. Each participant has its own read mark: a message
is unread for a participant until a lister has shown it to them. A
participant's own messages are read for them from the start.

**Tools.**

| tool | does |
|---|---|
| `create_conversation(to, topic, text)` | opens a new conversation with an agent on a topic, with its first message; answers the conversation's id |
| `reply(conversation, text)` | adds a message to a conversation the caller is in |
| `list_conversations(include_read?)` | the caller's conversations: by default only those with unread messages, each with its id, the other agent, the topic, how many messages are unread and when the last one came. With `include_read`, every conversation |
| `list_messages(conversation, include_read?)` | one conversation's messages: by default only the unread ones, oldest first. With `include_read`, the whole conversation. Every message it shows is marked read for the caller |

`to` names an agent as the daemon names one: by its name, or by its
template and index. A conversation with an agent that does not exist,
or one the caller is not in, is refused in words.

**State.** Conversations and read marks live in Postgres, through the
proxy, so the inbox survives the container stopping. The container is
one per tool, shared by every agent attached to it, so every inbox is
in one place. Transient mode suffices for that; service mode would
only keep it warm.

**Arguments.** None to start with.

**Open questions.**
- Whether notifications coalesce. As stated, every message sends one.
  An agent that has not yet read its inbox could be sent one
  notification until it does; the stated rule is one per message.
- Whether an agent can find whom to write to. `create_conversation` needs a name
  or a template and index; the courier offers no directory of agents.
  An agent learns names the way it learns anything else: from its
  instructions, or from a message it received.
- Whether a conversation can have more than two agents. Two to
  start; a group thread is the same tables with a participant list.
- Which grant the tool's account holds: `agents.message` over the
  agents that use the courier, and nothing broader. The example's
  setup writes it down.
- A client over a connect (`kind: unbrokered`) has a stable identity
  too and could have an inbox; to start, only agents do.

## `taskboard`: one board, shared by the agents that use it

A shared board of tasks. Every agent that reaches the tool's database
works from the same board, and the attested caller is what tells the
agents apart.

**Built:** `examples/taskboard/` (crate `diverge-example-taskboard`,
binary `taskboard`), with its `Containerfile` and `DIVERGE.md`. It is
a **dependency tool** declared by agents' images, a `service` (shown
with `scope: template`; `global` is legal), with no arguments, no
secrets and no permissions. It is pull only: nothing is sent to an
agent when the board changes.

**How it works.** Two Postgres tables, tasks and comments. Every
mutating call records the attested caller, so a claim knows who holds
a task. Every change of a task's state is one conditional statement,
so a claim is atomic and only the holder completes or releases.

**Tools.**

| tool | does |
|---|---|
| `create_task(title, detail)` | posts an open task; answers its id |
| `list_tasks(state?, limit?)` | the board, or the open, claimed or done tasks, oldest first, each with who posted, who holds and its comment count |
| `claim_task(task)` | takes an open task; atomic, so two agents can never hold the same one |
| `complete_task(task, result)` | marks the caller's task done with its result |
| `release_task(task)` | gives the caller's task back to open |
| `delete_task(task)` | deletes a task the caller posted, in any state, with its comments |
| `create_comment(task, text)` and `list_comments(task)` | a running discussion on one task |

**Why this one.** It shows the thing a service tool is for:
coordination among agents through one shared object. Two alternatives
for the same slot were considered: a `ballot` (propose, vote, tally)
and a two-player `chess` game between agents. The task board shows
the most, since it uses ownership as well as shared state.

## `memory`: a notebook in the tool's database

A persistent notebook. It uses Postgres through the proxy, and the
notebook is that database and nothing narrower: the tool tells no
caller apart and records no owner, so every agent that reaches the
same database shares the same notes.

**Built:** `examples/memory/` (crate `diverge-example-memory`, binary
`memory`), with its `Containerfile` and `DIVERGE.md`. It is a
**dependency tool** declared by agents' images, `transient`, with no
arguments, no secrets and no permissions. Intended per agent;
whose notebook it is follows from how it is declared, never from the
tool.

**How it works.** On the first tool call the program makes its one
table if absent. Each note holds its text, its tags and when it was
made, with a stored English full-text vector of its text.

**Tools.**

| tool | does |
|---|---|
| `remember(text, tags?)` | keeps a note; answers its id |
| `recall(query?, tags?, limit?)` | the notes carrying every tag given; with `query`, those whose text matches it as a web search (`websearch_to_tsquery`), best first; without, the newest first. At most 20 unless `limit` says |
| `forget(id)` | deletes a note |

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

1. `memory`: the smallest surface (Postgres alone), and
   the first live run of a tool container.
2. `workspace`: adds a volume mount.
3. `taskboard`: adds service mode and several agents at once.
4. `courier`: builds on `memory`'s Postgres, and adds the attestation,
   the dependency grants and the daemon access, the most moving
   parts.
