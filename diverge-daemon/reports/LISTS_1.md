# The lists of the daemon wire

Every list the daemon serves, as of 2026-10-08 (resources gone, tags renumbered), in one page: what is
asked, what comes back, where each part of an item comes from, and
what already streams. Written so that a watchable list — a list kept
open, sending what changes — can be decided over it. Decided and
built: as of 2026-10-08 every one of the twelve is a stream kept
open, in the one shape `STREAMS_1.md` states; the rows below say so.

## 1. One shape

Ten lists are over the daemon's records. Every one is the same
exchange:

- **The request** is one frame: a `filter`, flattened, whose every
  member is optional and absent means any; and `count`, the most to
  send, absent meaning all.
- **The answer** is a stream: zero or more item frames, one per record
  the grants reach and the filter admits, in the store's order, then
  the finish. `Forbidden` is sent alone, as the one frame, when the
  caller holds no `list` grant of the kind at all; a record the grants
  do not reach is left out silently. `Error` ends the stream after
  what was sent. Nothing, then the finish, is an answer: no record
  matched.
- **The order** is `created, id` — oldest first, ties by id — for
  every record list. Volumes are in provider-record order, each
  provider's volumes in the order it listed them. A `count` caps what
  is sent after judging and filtering, so it is the first `count`
  matches, not the first `count` records.
- **The handler** (`serve/<kind>/list.rs`) loads every record of the
  kind in one query, takes one snapshot of the live state it needs,
  judges each record by the grants' `within` — which is the same
  filter type as the request's, so a grant reaches exactly what a
  filter would match — tests the request's filter, caps, and sends.
  The connection to the store is let go before the first frame is
  sent.

One list is not over records, and has no filter and no count:
`postgres list` (81) reads the daemon's live state alone.

## 2. The ten

| Tag | Request | Item | Filter members | Live in the item |
|---|---|---|---|---|
| 48 | `accounts list` | `Account` | names, identities, named, credentialed, roles, connected, creators, all_tags, any_tags, created_from, created_to | `connected`; a stream since 2026-10-08 |
| 55 | `roles list` | `Role` | names, accounts, creators, all_tags, any_tags, created_from, created_to | —; a stream since 2026-10-08 |
| 34 | `providers outgoing list` | `Outgoing` | addresses, kinds, connected, creators, all_tags, any_tags, created_from, created_to | `connected`; a stream since 2026-10-07 |
| 41 | `providers incoming list` | `Incoming` | identities, connected, creators, all_tags, any_tags, created_from, created_to | `connected`; a stream since 2026-10-07 |
| 11 | `agents templates list` | `Listed` | ids, creators, in_use, all_tags, any_tags, created_from, created_to | `in_use` (filter only); a stream since 2026-10-08 |
| 28 | `tools templates list` | `Listed` | ids, creators, in_use, all_tags, any_tags, created_from, created_to | `in_use` (filter only); a stream since 2026-10-08 |
| 5 | `agents list` | `Agent` | names, templates, creators, active, all_tags, any_tags, created_from, created_to | `active`, `logs_index`; a stream since 2026-10-08 |
| 23 | `tools list` | `Tool` | names, templates, creators, kind, active, agents, all_tags, any_tags, created_from, created_to | `active`; every dependency tool running now, as an item of its own while it runs (2026-10-09); a stream since 2026-10-08 |
| 68 | `volumes list` | `Volume` | providers, names, modes, mounted, all_tags, any_tags, created_from, created_to | `mounted` (filter only); a stream since 2026-10-08, over the daemon's mirror of each connected provider's listing |
| 81 | `postgres list` | `Connection` | none | all of it; a stream since 2026-10-08 |


## 3. The items

What each item carries, and which members are the record's and which
are read at list time.

- **`Account`** — `name`, `credential` (`identity`, `address`; never the
  key), `description`, `roles` (names), `tags`, `created`, `creator`;
  live: `connected`, whether a client is connected as the account now
  (`Live.connected`, a count per account).
- **`Role`** — `name`, `description`, `grants`, `accounts` (references
  of the accounts holding it, joined at read time), `tags`, `created`,
  `creator`. Nothing live.
- **`Outgoing`** — `address`, `kind`, `created`, `creator`;
  `last_connected` is a record column the dial task writes as a
  connection opens and closes; live: `connected` (`Live.providers`, the
  slot for `Identity::Outgoing`).
- **`Incoming`** — `identity`, `address`, `created`, `creator`; live:
  `connected` (`Live.providers`, the slot for
  `Identity::IncomingUnbrokered`; exactly one connection or none).
- **`Listed`** (templates, both kinds) — `id`, `template` (the
  definition whole), `tags`, `created`, `creator`. `in_use` is not in
  the item: it is a filter member, read from the records that name the
  template (agents or tools made from it).
- **`Agent`** — `name`, `template`, `index`, `creator`,
  `created`, `provider` (pinned, if any),
  `last_active` (record), `tools` (keys of the attached tools, joined),
  `tags`; live: `active` (`Live.agents`, the run's loop), `logs_index`
  (the log's length on disk).
- **`Tool`** — `name`, `origin` (created from a template, connected
  through a provider, or a dependency deployed for an agent — each
  with its index, the dependency with its agent, its template's id
  and the template whole, its provider and its container id — a
  dependency has no `name`), `creator`,
  `created`, `last_active`, `agents` (keys of the agents it is attached
  to), `admissions`, `tags`; live: `active` (`Live.tools`). A
  dependency tool is no record: it is an item while its run is in
  `Live.tools` and gone after — always `active`, attached to the one
  agent it was deployed for, no tags, no admissions, made by itself as
  `Creator::Tool(Dependency)` when it was deployed (2026-10-09).
- **`Volume`** — `provider`, `name`, `bytes`, `mode`, `created` (the
  provider's listing), `agents` and `tools` (keys of the records that
  mount it, running or not), `tags` (the daemon's, kept in
  `volume_tags` beside the listing). `mounted` is a filter member:
  either list non-empty. The whole item is read from the provider at list time; a
  provider not connected contributes nothing.
- **`Connection`** (postgres) — `container` (as the database names
  it), `opened`; entirely `Live.connections`.

## 4. What changes an item

A watchable list sends a change when an item enters, leaves, or
differs. Each of those has two sources today, and neither announces
itself.

**The record changed.** Every create, edit, tag, untag, delete of the
kind, and — for items that join other records — an attach or detach
(agents' `tools`, tools' `agents`), an admission (tools'
`admissions`), a role assigned or
unassigned (accounts' `roles`, roles' `accounts`), a template's use
(templates' `in_use`), a mount named (volumes'
`agents`/`tools`), a dial opening or closing (`last_connected`), a
run (`last_active`). The store is written by the handler of each
request, in its transaction; nothing is told afterwards but the
kind's word.

**The live state changed.** A client connecting or leaving
(`connected`), a provider's slot taken or given back (`connected`), an
agent's or a tool's run starting, its loop going active or inactive,
or ending (`active`), a dependency deployed or ended with its agent
(a tools item added or removed), a line appended to a log (`logs_index`), a
database connection opening or closing (`Connection`), a provider's
listing of its volumes changing under it (`Volume`). `Live` is a set
of maps behind mutexes, written in place; the only things watched in
it today are each log's `latest` (a `watch`), each run's `loop_active`
(a `watch`), and the mounts' change feed (a `broadcast`), none of them
a list.

## 5. What streams already

Three exchanges are kept open now, and are the precedents a watchable
list would be shaped after.

- **`agents logs` (4)** — the request carries `watch: Option<bool>`.
  The daemon sends every item that matches the filter from the log on
  disk, then, when `watch` is `true`, stays open and sends each item
  appended after, as the log's `latest` moves, until the client's
  cancel channel or the agent's deletion. The cursor is `logs_index`;
  a client that reconnects asks from the index it last saw.
- **`agents filetree` (79), `tools filetree` (80)** — a `Snapshot`
  frame of the whole tree first, then `Inserted`, `Modified`, `Removed`
  frames, each naming a path, until cancel, the run's end, or an
  error. The daemon merges the provider's stream with its own mounts'
  changes, and sends the tree whole again when it lost track.
- **The cancel** — one channel the client opens on the scope, carrying
  one frame `Cancel`, read and never answered; the inbox closing is
  not a cancel. Every streaming endpoint shares it.

## 6. What is not here

`volumes list` is a provider's answer, not the daemon's: the
provider's `volumes list` is a stream since 2026-10-07, with a volume
changed beside added and removed. (`tools list_for`, the provider's
other listing, was removed from both wires on 2026-10-09; a tool's
connectors are told to its runner on the run's own stream instead.)
The daemon holds one `volumes list` of each connected
provider for the connection's life, mirrored in `Live`, which every
reading of a provider's volumes on the daemon reads — no scope per
call. `get` endpoints answer one item as a
list would, and are not lists.
