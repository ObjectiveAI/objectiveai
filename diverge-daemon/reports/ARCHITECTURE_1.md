# Architecture, draft 1: the daemon, whole

The big picture of `diverge-daemon`, from the socket in to the
containers out. What is already decided is stated as fact; the few
things still open are marked. Everything the daemon says on the wire
is fixed by `diverge_sdk::daemon`; this is about what stands behind
the wire.

## 1. What the daemon is

One process, one directory, one port. It is the thing a client of the
daemon protocol talks to, and it is the thing providers run containers
FOR. It holds the records — accounts, roles, templates, agents, tools,
routes, resources, providers, the database mode — judges every request
by the account's grants, and does the work: running containers on
providers, carrying what those containers ask back in, serving them a
database, moving files between everything. It serves nothing of its own
to the outside but this one protocol.

## 2. The five layers

```
 client ──► FRONT ──► JUDGE ──► RECORDS ──► WORK ──► providers, postgres
            socket    grants    the truth   running   the outside
```

- **Front.** The port, the WebSocket, the SDK's frame-level session, and
  the dispatch over every `ClientRequest`. One task per scope. Exists
  today; every arm refuses.
- **Judge.** The credential at the handshake becomes an account; every
  request is allowed or `Forbidden` by the union of the grants of the
  roles the account holds, judged over the exact things the request
  names. A container's requests, arriving through its proxy, are judged
  as the container's `account`. One module, called by every handler
  first; no handler judges on its own.
- **Records.** The durable truth: what exists, who owns it, what it is
  named, what it is made of. Written before the work and after it, never
  during.
- **Work.** Everything that runs: containers on providers, the channels
  into them, the database connections spliced through, transfers in
  flight, logs being written. All of it is live state that a restart
  discards and the records let the daemon rebuild or sweep.
- **Outside.** Providers, dialled or dialling in, spoken to through the
  SDK's provider client half; the local Postgres supervised by
  `diverge-postgres` or a remote one at a URL.

The rule between layers: the front never reaches past the judge, the
judge never touches work, records never hold anything live, and work
never writes a record except through the records layer.

## 3. The records

Two populations, kept apart by type and by fate:

| | durable records | live state |
|---|---|---|
| what | accounts, roles, grants, templates, agent and tool definitions, routes, resource index, providers, the mode, counters | running containers, their scopes and channels, open database connections, held volumes, transfers, uploads |
| where | the store | memory, under the `Daemon` |
| at restart | reloaded | gone; the records say what SHOULD be running and the daemon converges |

- **The store.** The SAME Postgres the daemon serves to containers, in a
  schema `diverge` of its own that no container role can read. Which
  Postgres is configuration — local, the daemon's own beside it, or a
  remote URL — read at start and never changed while the daemon runs:
  another database is another start, and what the old one holds stays
  there. (Ruling of 2026-10-06; the earlier draft wanted a separate
  store and a live mode swap.)
- **Content lives beside the store, not in it.** Resource bytes under
  `<dir>/resources/<id>` — a file as that path, a directory as that
  tree — with uploads still arriving under `resources/incoming/`; agent
  logs appended under `agents/<id>/log` with an index; both read by
  streaming. The store holds the index of a thing,
  never the thing.
- **Counters are records.** The once-and-for-all identity — template and
  index — is a durable, monotonic counter per template, advanced in the
  same transaction that creates the agent or the tool, so a crash
  mid-create never hands one name to two.
- **One owner in memory.** `Daemon` is one `Arc` every task borrows:
  the store handle, the live registries per kind, the provider
  connections, the mode. Per-kind locks, taken in one fixed order wherever
  a request touches two kinds; never one lock over all.

## 4. Who may ask

- **Accounts are the only principal.** A client is an account by its
  credential; a container is an account by its `account`. Nothing else
  asks.
- **Roles are lists of grants; grants are per kind.** Default deny; the
  union allows; nothing denies. The judge collects the grants of the
  action from every role the account holds and passes the request if any
  one reaches everything the request names.
- **Bootstrap.** A fresh database — one with no accounts table before
  the daemon initialized it — is seeded with a role `root` holding every
  grant and an account `root` holding it, whose key is the word `root`.
  Both are ordinary records: the first client connects with `root`,
  makes the real accounts, and rotates or deletes root. A database that
  has the table is never seeded again.
- **Delegation is bounded.** An account hands out only grants it holds —
  `assign` over an account, `grant` over a role — so nothing escalates
  through a container or a role.

## 5. Containers

- **Where they run.** Every agent and tool runs on a provider: an
  outgoing one the daemon dialled, or an incoming one that dialled in and
  was judged by a credential the daemon minted. The daemon is the caller
  of the provider protocol — `run`, `connect`, volumes, `list_for` — and
  holds one run scope per container for its life.
- **What comes back through the run.** The proxy inside the container
  opens channels on the run scope, and each kind is one module of the
  work layer: `/daemon` pairs, served as requests of the container's
  account through the same front dispatch a client gets; Postgres pairs,
  handshaken and relayed to the served database; MCP, enqueue and
  dequeue, filetree, vault — each as the provider protocol states.
- **Who may see or join a tool from outside is an admission.** A
  provider asks the daemon two questions about a tool it runs there:
  may this lister see the container, and may this connector attach. The
  answers come from ADMISSIONS on the tool, records put down by
  `tools::admit` and taken back by `tools::unadmit`: an identity on the
  provider, an address if one, and what it admits — list, connect, or
  both. A list is yes when an admission names the lister's identity and
  admits a list; a connect is yes when the connector's authorization is
  an admission's key — minted and answered once, as an account's — and
  the admission admits a connect. Default deny. A connected tool is
  somebody else's, and its runner admits.
- **Templates are the definitions.** An agent or a tool is a template
  plus what its create added — mounts, account, provider — and the record
  is that; the container is work made from it, and remade from it after a
  restart if the record says it should run.
- **Delete is refused while held.** A volume a container mounts, a
  resource a transfer reads, a database with a connection open: the live
  state is consulted before the record is touched, under the kind's lock.
- **A loop is the proxy's word.** Whether an agent is ACTIVE — a loop
  running in it — is read off the run's main stream, where the proxy
  says `active` before a loop's first chunk and `inactive` after its
  last; the program's output is chunks and cannot say either. Nothing
  is derived from quiet. (Wire change of 2026-10-06.)
- **A cross-provider mount is served for the run.** The `volumes::serve`
  scope the daemon holds on another provider's volume for a FUSE mount
  is opened when the container's run starts and let go when it ends —
  the run's, not the record's, since the record outlives any run.
- **One tool list, prefixed.** An agent sees the union of its attached
  tools and its dependencies, each MCP tool as `<prefix>_<name>`: the
  prefix is the serve-name folded to `[a-z0-9-]`, escalated to `name-2`,
  `name-3`, … while taken, assigned once per run and kept until the tool
  leaves, so nothing is dropped or renamed while served. A call routes by
  its first `_`.
- **Unused, a container stops.** Every container, agent or tool, has an
  idle clock: it resets on every use — a message delivered, a tool call
  relayed, a file moved in or out, a request the container itself makes
  of the daemon — and it does not run while the container is ACTIVE,
  handling something. When it reaches `idle_seconds`, a key of
  `config.yaml` beside `postgres`, default `10`, the daemon ends the
  container's run. The record stays; the container is work, and the next
  use starts it again from the record and its continuation. Nothing a
  client holds — a logs watch, a filetree watch — counts as use, so a
  watcher never keeps a container alive.
- **Dependencies are templates, and the deployer is a queue.** A
  container declares its tool dependencies when it registers, each a
  tool TEMPLATE on record with instructions; a dependency naming no
  template of the caller's is unmet, and that is a failure of the start.
  A position a route already answers is served from the route. One no
  route answers goes to the container's `deployer_agent` as an INTERNAL
  MESSAGE — the position, the template, the instructions — handled
  like any message: the deployer's output lands in its log, and the
  deployer is an agent like any other. The deployer holds a queue and
  handles one dependency at a time, in order; a second waits. The daemon
  waits on whichever comes first: the dependency ANSWERED — a tool of
  that template attached at the position, by the deployer's attach or
  by a route it set — or the deployer going INACTIVE. Answered, the
  start goes on; inactive first, the dependency is unmet, and the
  container being started is failed then and there: its run ended, its
  create or its message answered with the error. A container with no
  deployer and no route for a dependency fails the same way.

## 6. The database

- **One database, two modes, by configuration.** Local, a cluster
  `diverge-postgres` runs beside the daemon — spawned at start, its ready
  line read, its shutdown line sent at stop; remote, a URL. The daemon is
  the end that holds the database for every container, and keeps its own
  records in it. `postgres::get` answers the mode; nothing on the wire
  changes it.
- **One scope per container.** A login role and a same-named schema,
  made at the container's first connection, searched implicitly,
  inescapable by privilege, dropped at delete — `POSTGRES_1.md` in the SDK
  reports. The daemon performs the handshake for the container and relays
  the rest unread, so no credential ever enters a container.
- **Never swapped while running.** Another database is a config edit
  and a restart.

## 7. Concurrency

- Every scope is a task; every container's run is a task; every channel
  into a container is a task. Tasks own their sockets and are ended by
  dropping them.
- The `Daemon` is shared, its live registries are per kind, and a lock
  is held across no `await` that waits on the outside. Anything that
  waits on a provider or a database waits outside the locks, with the
  record marked held meanwhile.
- A stop is a sweep: the listener drains, every container run is ended
  as the provider protocol ends one, the local Postgres is told to shut
  down, and the records are left saying what to bring back.

## 8. The order of building

1. **Store and accounts.** The store engine, the root key, accounts,
   roles, grants, and the judge made real. The first `Forbidden` and the
   first `Created`.
2. **Providers.** Outgoing added and dialled, incoming judged, `list_for`.
3. **Templates and resources.** The definitions and the content store.
4. **Agents and tools.** Create, run on a provider, the run scope, the
   `/daemon` pair served through the front, logs, message, delete,
   restart convergence.
5. **The database.** The mode, `diverge-postgres` spawned in local mode,
   the handshake and the relay, scopes per container.
6. **Volumes, transfers, filetree.** The file movement across every
   kind, and the held rule.

Each step replaces arms of today's `refuse` and nothing else about the
front changes; the shape that exists is the shape that ships.
