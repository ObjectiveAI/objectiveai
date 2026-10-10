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
providers, the database mode — judges every request
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
| what | accounts, roles, grants, templates, agent and tool definitions, providers, daemon records, the mode, counters, volume tags | running containers, their scopes and channels, open database connections, daemon connections, open exposures, held volumes, transfers, uploads |
| where | the store | memory, under the `Daemon` |
| at restart | reloaded | gone; the records say what SHOULD be running and the daemon converges |

- **The store.** The SAME Postgres the daemon serves to containers, in a
  schema `diverge` of its own that no container role can read. Which
  Postgres is configuration — local, the daemon's own beside it, or a
  remote URL — read at start and never changed while the daemon runs:
  another database is another start, and what the old one holds stays
  there. (Ruling of 2026-10-06; the earlier draft wanted a separate
  store and a live mode swap.)
- **Logs live beside the store, not in it.** Agent logs appended
  under `agents/<id>/log` with an index, read by streaming. (The
  content store of resources went on 2026-10-08: a container's content
  is a provider's volume.) The store holds the index of a thing,
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
- **One connection per incoming credential.** A provider that dials in
  holds, for its connection's life, both the identity its credential
  names and the credential itself; a second connection presenting that
  credential, or admitted as that identity, is closed without a word,
  as a refused credential is. The slot is taken before the version is
  asked and given back by the connection's own task, on every path. An
  edit of the credential ends the connection it holds and the next is
  judged by the new key; a delete while connected is refused. The
  provider server keeps the same rule for the peers that dial it, by
  identity and by credential. (Ruling of 2026-10-07.)
- **Daemons reach one another through providers, and accept by
  default.** A daemon holds, under `providers::daemons`, the other
  daemons it has an account on — a name here, the mode it
  authenticates to the remote in, and one link per provider the remote
  is reachable through, each a provider of the caller's and the
  identity the remote is known by there — and reaches each only
  through a provider both are connected to: the provider protocol's
  `daemons::connect`, the remote judging the credential as it judges
  any client's. On every provider connection it holds, a daemon opens
  one `daemons::accept` scope, unless `daemon: accept_daemons: false`
  in `config.yaml`, and is known there by the identity the provider
  answers; each connection the provider announces is judged by the
  credential in its mode and the address the provider saw, as a
  socket's handshake is, and served by the same session a socket
  gets. A daemon record is deleted only while no connected tool names
  it; a provider only while no record links through it. (Ruling of
  2026-10-09; built 2026-10-10.)

## 5. Containers

- **Where they run.** Every agent and tool runs on a provider: an
  outgoing one the daemon dialled, or an incoming one that dialled in and
  was judged by a credential the daemon minted. The daemon is the caller
  of the provider protocol — `run`, `connect`, volumes — and holds one
  run scope per container for its life.
- **What comes back through the run.** The proxy inside the container
  opens channels on the run scope, and each kind is one module of the
  work layer: `/daemon` pairs, served as requests of the container's
  account through the same front dispatch a client gets; Postgres pairs,
  handshaken and relayed to the served database; MCP, enqueue and
  dequeue, filetree, vault — each as the provider protocol states.
- **Who may join a tool from outside is an exposure.** A provider
  asks the daemon one question about a tool it runs there: may this
  connector attach. The answer comes from EXPOSURES, held in memory
  and on no record: a `tools::expose` scope starts the tool's
  container if it does not run, holds it while the scope is open, and
  answers once — the provider the container runs on, the identity the
  daemon is known by there, the container's id, and a key minted for
  that exposure alone — then stays open until the tool's run ends or
  the client cancels. A connect is yes when the connector's
  authorization is an open exposure's key and the exposure is this
  tool's; the key is spent by that one connection, and dies with the
  scope. Default deny. A connected tool is another daemon's, and that
  daemon exposes it. (Admissions gone 2026-10-10.)
- **A connected tool is another daemon's tool, reached through a
  provider.** `tools::connect` names a daemon record and a tool as
  that daemon names it. While an attached agent is active, the daemon
  connects to that daemon through a linked provider — the one
  connection reused while it lasts, opened through the connected
  links in random order when none is held — opens that daemon's
  `tools::expose` naming the tool, and joins the container the expose
  answers by `containers::tools::connect` on the provider the expose
  names, through the link whose identity the expose answered. The
  join and the expose are let go together with the last attached
  agent; either ending ends the run. (2026-10-10.)
- **Templates are the definitions.** An agent or a tool is a template
  plus what its create added — mounts, account, provider — and the record
  is that; the container is work made from it, and remade from it after a
  restart if the record says it should run.
- **Held, and in use.** A volume is HELD while a running container
  mounts it — the record names it in its mounts and the run is up — or
  a download, an upload or a transfer of the daemon's is on it; an
  edit, a stat, a filetree, a download, an upload and a transfer answer
  `Held` and are asked again once it is free, and a download, an upload
  or a transfer takes the volume for its own length. A volume is IN USE
  for a delete while any record names it in its mounts, running or
  not, or an operation is on it; `mounted` in a listing is the record
  rule alone. A database scope is in use while a connection is open. The live state is consulted before
  the record is touched, under the kind's lock. (2026-10-07.)
- **Parents are made, and an offline provider lists nothing.** A write
  into a container makes the missing parents, as a write into a volume
  does, so an upload or a transfer lands as the daemon wire promises;
  the proxy's own wire says nothing of parents, since what the proxy
  does internally is no one's business. A provider on record but not
  connected contributes nothing to a volumes listing; only a connected
  provider that could not be asked is the listing's error, after what
  was sent. (Rulings of 2026-10-07.)
- **A loop is the proxy's word.** Whether an agent is ACTIVE — a loop
  running in it — is read off the run's main stream, where the proxy
  says `active` before a loop's first chunk and `inactive` after its
  last; the program's output is chunks and cannot say either. Nothing
  is derived from quiet. (Wire change of 2026-10-06.)
- **Every list is a stream kept open.** Each of the thirteen lists sends
  what matches as added, the word that the list is whole, and from then
  on each item added, changed or removed, until the client's cancel.
  A word per kind (`daemon::Kind`, said by every handler after its
  commit and by the live state at every transition that reaches an
  item) tells a listing to read again and tell the difference by key;
  nothing is announced per log line. `STREAMS_1.md` is the whole of
  it. (2026-10-08.)
- **The daemon attests under `_meta`; the proxy and the provider relay.**
  On every MCP call an agent sends outward the daemon sets the agent's
  image and key (`diverge.network/image`, `diverge.network/agent`)
  before it forwards the call; on everything a tool's server answers —
  the result, each tool and resource of a list, each notification — the
  tool's key (`diverge.network/tool`) and its image when the daemon ran
  it, as it relays the answer back; on every chunk an agent says, the
  agent's, as it keeps the chunk in the log. The proxy and the
  provider set nothing under `_meta` and send it as sent; the begin
  request carries no image. `diverge_sdk::shared::mcp` is the whole
  of it. (2026-10-08.)
- **A provider's volumes are mirrored for the connection.** The daemon
  opens one `volumes::list` on every provider as it attaches and keeps
  it for the connection's life; what the provider's volumes are is read
  from that mirror, never asked per request, and every change the
  provider streams is a word to the lists kept open. (2026-10-08.)
- **A cross-provider mount is served for the run.** The `volumes::serve`
  scope the daemon holds on another provider's volume for a FUSE mount
  is opened when the container's run starts and let go when it ends —
  the run's, not the record's, since the record outlives any run. A
  dependency's mounts are the agent's own paths, served into the
  dependency's container by a `containers::serve` scope on the agent's
  provider for the dependency's life, spliced into the dependency's
  tree at `tool_path` and changed as the agent's serve streams.
- **One tool list, prefixed.** An agent sees the union of its attached
  tools and its dependencies, each MCP tool as `<prefix>_<name>`: the
  prefix is the serve-name folded to `[a-z0-9-]`, escalated to `name-2`,
  `name-3`, … while taken, assigned once per run and kept until the tool
  leaves, so nothing is dropped or renamed while served. A call routes by
  its first `_`.
- **A tool runs while held.** A record tool's container is started
  for the first container of the daemon's that uses it — an agent's
  loop beginning, a call outside a loop, a file operation — and
  stopped when nothing holds it: no user of the daemon's, no expose
  scope open on it, and no connector attached from outside. The
  provider tells the runner of every connector coming and going on
  the run's own stream (`Connected`/`Disconnected`, 2026-10-09), and
  the daemon counts them; the last of the three leaving is the one
  test. An expose on a tool already running is a touch of it. A tool
  has no idle clock.
- **Unused, a container stops.** Every agent has an
  idle clock: it resets on every use — a message delivered, a tool call
  relayed, a file moved in or out, a request the container itself makes
  of the daemon — and it does not run while the agent is ACTIVE, which
  is exactly two things: a loop running in it, between the proxy's
  Active and Inactive, or an MCP exchange in flight for it or for any
  of its dependency tools. The countdown runs only when neither holds. When it reaches `idle_seconds`, a key of the
  daemon's block of `config.yaml` beside `postgres`, default `10`, the daemon ends the
  container's run. The record stays; the container is work, and the next
  use starts it again from the record and its continuation. Nothing a
  client holds — a logs watch, a filetree watch — counts as use, so a
  watcher never keeps a container alive.
- **Dependencies are deployed then and there, for the agent's life.**
  An AGENT declares its dependencies when it registers, each a
  dependency tool template — image, limits, arguments, the agent's
  paths to serve into it, which database scope it gets, the grants its
  requests are judged by — and the provider asks the daemon to deploy
  them, the agent's container id with the ask, before the id is out;
  a tool container declares none (2026-10-09). The daemon deploys
  every one AT ONCE, each a tool container of the agent's own on any
  connected provider, the candidates tried in random order and the
  next tried when one will not run it; the first that cannot be
  deployed is the run's error, and every one started is stopped. A
  dependency is no record: it is keyed by a number minted for the
  deploy, listed as `kind: dependency` while it runs, reachable by the
  reading tool requests — get, filetree, download, transfer out — and
  refused by every changing one, since it is its agent's. Its
  `/daemon` connections are served under its template's grants, fixed
  at the deploy; it is named by its template's id — the hash of the
  template's canonical bytes; a template has no name — and its
  database scope by its agent, once and for all, or by its agent's
  template, as the template's `database` says, the same scope on
  whatever provider it is deployed. When the agent's run ends, every dependency and every attached
  tool is told to stop at the same time, and the daemon waits for all.
  An unpinned agent or tool is placed the same way: every connected
  provider, shuffled, the next tried on a failed run.

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
  reports. The role names its OWNER first — `diverge_`, twenty hex of
  the owner's canonical hash, twenty of the scope's part within it —
  so that everything an owner has shares a prefix: an agent's own
  scope and the scope of every `per_agent_instance` dependency ever
  deployed for it are swept from the catalog by the agent's prefix at
  its delete, logs and all; a `per_agent_template` scope is the agent
  template's, never swept by an agent's delete, and swept by the
  template's delete, which only happens once no agent is left of it;
  a tool's own scope goes with the tool, and a tool template owns
  none (2026-10-09).
  Every id that is a hash — a template's, an owner's — is over
  canonical bytes: compact JSON, every object key sorted at every
  depth, `arguments` included (`diverge_sdk::shared::canonical`). The daemon performs the handshake for the container and relays
  the rest unread, so no credential ever enters a container.
- **Never swapped while running.** Another database is a config edit
  and a restart.
- **The handshake is the daemon's.** The container's driver is asked
  no password; the daemon reads its startup packet, answers
  `AuthenticationOk`, and authenticates toward the database as the
  container's role through the `postgres-protocol` crate — SCRAM, md5,
  or cleartext over TLS alone — then relays whole messages unread.
  (Ruling of 2026-10-07: no credential in the container, no hand-rolled
  protocol.)
- **TLS toward a remote database** by the URL's `sslmode` as libpq
  reads it, `prefer` when absent, through rustls; `verify-ca` is held to
  `verify-full`.
- **The privilege check is per connection.** A URL's role lacking
  `CREATEROLE` or `CREATE` on the database refuses that container's
  connection, with an `error` item in an agent's log; the daemon starts
  regardless.

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
2. **Providers.** Outgoing added and dialled, incoming judged.
3. **Templates.** The definitions. (Resources, the content store, were
   built here and removed on 2026-10-08.)
4. **Agents and tools.** Create, run on a provider, the run scope, the
   `/daemon` pair served through the front, logs, message, delete,
   restart convergence.
5. **The database.** The mode, `diverge-postgres` spawned in local mode,
   the handshake and the relay, scopes per container.
6. **Volumes, transfers, filetree.** The file movement across every
   kind, and the held rule.

Each step replaced arms of the front's refusal and nothing else about
the front changed; the shape that exists is the shape that ships. All
six are built (2026-10-07): every one of the eighty-nine requests has its
handler, and the refusal is gone.
