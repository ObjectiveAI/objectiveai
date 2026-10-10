# The lists that become streams

Which of the daemon wire's twelve lists ([`LISTS_1.md`](LISTS_1.md))
become streams kept open, in what order, and what each needs. The
purpose is a visual client: one that opens a list once and is told
every change as it occurs, rather than asking again.

## 1. The one shape

Every list that becomes a stream takes the shape the provider wire's
two listings took on 2026-10-07 (`containers::tools::list_for`,
`volumes::list`):

- **The request** is what it was: the filter and the count where the
  list has them. A count caps what is sent before the word that the
  listing is whole, and nothing after it.
- **The answer** is a stream: every item the grants reach and the
  filter admits, each as `Added`; then exactly one bare `Listed`,
  the word that the listing as it stood is sent; then, for the
  scope's life, `Added` for an item that comes to be listed, `Changed`
  for one whose item differs from what was last sent, `Removed` for
  one that ceases to be listed — the filter and the grants applied to
  every change, so an item that stops matching is `Removed` and one
  that starts matching is `Added`. `Forbidden` alone, and `Error`
  last, as today.
- **The end** is the client's `Cancel` channel — the one-byte
  channel every watch on the daemon wire already has — the client's
  connection ending, or the error. An empty listing is `Listed` at
  once and watched.
- **The daemon's own `Changed`** is by comparison: the daemon keeps
  what it last sent per item and tells only a difference, so a change
  that leaves the item as it was is nothing.

## 2. The twelve, in the order they are built

| Order | Tag | List | What tells the daemon something changed |
|---|---|---|---|
| 1 | — | `tools list_for` | **Removed 2026-10-09** with the provider's listing: a tool's connectors are told to its runner on the tools run stream instead. |
| 2 | 78 | `postgres list` | `Live.connections`: a connection opened or closed. **Built 2026-10-08.** |
| 3 | 67 | `volumes list` | Each connected provider's own stream, merged; a provider connecting or leaving; the records that mount a volume (an agent or tool created, edited, deleted). **Built 2026-10-08: the daemon mirrors one listing per connected provider (`volumes::Mirror`, `volumes::watch`).** |
| 4 | 5 | `agents list` | The record (create, edit, tag, untag, delete, attach, detach, a tool renamed); `Live.agents` (a run starting or ending, its loop active or inactive, a start failed). **Built 2026-10-08.** The log's length is read at those changes and not per line: see §3. |
| 5 | 23 | `tools list` | The record (create, connect, edit, tag, untag, delete, attach, detach, admit, unadmit, a route set or deleted, an agent renamed); `Live.tools` (a run inserted or removed). **Built 2026-10-08.** |
| 6 | 28 | `tools routes list` | The record (set, delete; a tool deleted); a tool renamed. **Built 2026-10-08.** |
| 8 | 11, 31 | `agents templates list`, `tools templates list` | The record (create, tag, untag, delete; an agent or tool made from one for `in_use`). **Built 2026-10-08.** |
| 9 | 47 | `accounts list` | The record (create, edit, tag, untag, delete); a role renamed or deleted; `Live.connected` (a client connecting or leaving). **Built 2026-10-08.** |
| 10 | 54 | `roles list` | The record (create, edit, tag, untag, delete); an account made, edited or deleted. **Built 2026-10-08.** |
| 11 | 37, 42 | `providers outgoing list`, `providers incoming list` | The record (add, edit, delete); `Live.providers` (a connection taken or given back); `last_connected` written by the dial. **Built 2026-10-07, first of the record lists.** |

## 3. What the record lists need, once

The word exists since 2026-10-07: `daemon::Kind`, one variant per
kind, and `Live::changed(kind)` / `Live::changes()`, a broadcast of
kinds. The two provider kinds send it after each add, edit and
delete commit, at a slot taken or given back, and at each write of
`last_connected`; `serve::stream::listing` is the loop every record
list runs over a `Source`, with `diff` telling added, changed and
removed by key. Each remaining list needs its word sent — by every
handler of the kind after its commit, and by the live state at
every transition that reaches an item (`Live.enter`/`leave`,
`connect_provider`/`disconnect_provider`, a run inserted or removed,
its loop's `watch`, a log appended, a database connection opened or
closed). A stream of a kind subscribes to its word before it reads
the records, reads them again on every word, and tells the
difference — the shape the provider's `volumes::list` has, where the
word says when to look and the store is the truth. Which kinds a
write touches is part of the handler's knowledge: an attach touches
agents and tools; a mount named touches volumes; a route set
touches routes and tools.

**Not per log line.** A log grows by one item per chunk, and chunks are
token-level deltas: hundreds per reply. The agents word is sent at an
agent's changes of state — the run starting and ending, the loop
beginning and ending, a start that failed — and never on an append, so
`logs_index` on the item is the length as of the last change of state.
The log as it grows is `agents logs`, which has its own per-agent watch.

## 4. What stays as it is

`get` endpoints answer one item once. `agents logs` and the two
`filetree` watches stream already. The provider wire is done: both
of its listings are streams; `volumes list` is mirrored, one per
connected provider, and `tools list_for` is gone (2026-10-09).

## 5. Done

All twelve are built as of 2026-10-08 (the resources list went with the resources feature the same day), in the one shape of §1: a
word per kind (`daemon::Kind`, `Live::changed`), one loop
(`serve::stream::listing` over a `Source`, the difference told by
key), the client's cancel as the end, and the agents rule of §3 —
never a word per log line.
