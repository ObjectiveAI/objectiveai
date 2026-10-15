# Report 4: senders, optional names, the shared create and edit, self, one maker, and providers

Covers the eighteen commits after report 3 on `p2p-provider-binary`,
from `891246e5f` on 2026-10-03 to `159dc59d1` on 2026-10-04, all in
`diverge-sdk-rs/src/daemon`. Every one of the protocol commits is a
wire change. The daemon's tag table grew from thirty-seven requests to
forty-seven, and the daemon's tools from twenty-nine members to
forty-six.

| commit | what |
|---|---|
| `891246e5f` | a user part in a log carries its sender |
| `80f4b602f` | `name` is optional everywhere; `daemon::key`; a connected tool referenced by provider and id |
| `ccf353d77`, `7fcf0c49c` | `daemon::create::Inner` shared by both creates; `DaemonTools` dissolved into it |
| `1d8eae659`, `22e2c237e` | `daemon::edit::Edit` shared by both edits; every member an optional `Change` |
| `ed52d6dcc` | daemon tools gain the four gets |
| `edf444b4a`, `6c6efe36d`, `52d65b9af` | routes `set`; the `"self"` reference; the `agents_self` and `tools_self` permissions |
| `276a94aeb` | a creator is one maker, not a chain |
| `4b2298cb6`, `ffb18243d`, `18fafb5d4` | a template made again keeps its creator and its created time |
| `a02e77a17`, `42c3081f3`, `a9a8e4a60`, `159dc59d1` | providers, outgoing and incoming, at 37–46 |

## 1. The sender of a message (`891246e5f`)

A log's `Item` gains a first variant, `User`: a user part — one of the
five `user_*` chunks — with `sender` beside its flattened members.
Every other chunk stays a `Chunk`, and `sender` is what tells the two
apart on the untagged wire. The `agents::message` request names no
sender; the daemon knows who sent from the scope the request arrived
on and records it. The provider's chunk types are untouched.

## 2. Optional names, and keys (`80f4b602f`)

- `name` is optional on `agents::create`, `tools::create` and
  `tools::connect`, on the `Agent` and `Tool` list items, and on both
  creator structs. A create with no name is never refused for one.
- `reference::Tool` gains a third form, `{"provider":…,"id":…}`, so a
  nameless connected tool, which has no template, is still reachable.
- `daemon::key`: `Agent { template, index, name? }`,
  `Tool { origin, index, name? }` and `Origin`, `created { template }`
  or `connected { provider, id }`. `Agent.tools` and `Tool.agents` are
  lists of keys rather than names, each with the name beside when it
  has one.

## 3. The shared create and edit (`ccf353d77` through `22e2c237e`)

- `daemon::create::Inner`, flattened into both creates beside `name`:
  `template`, `provider`, the two FUSE mount lists, the daemon's tools
  and `deployer_agent`. `DaemonTools` is gone; its members are members
  of `Inner`, each `#[serde(default)]`, always present and `disabled`
  when not held, beside the mounts as the mounts are. A sparse create
  decodes with every tool disabled.
- `daemon::edit::Edit`, flattened into both edits beside the
  reference: `name`, `volume_mounts`, the two FUSE lists, every daemon
  tool, `deployer_agent`. Every member is `Option<Change<T>>`:
  absent keeps, `"delete"` takes away, `{"set":…}` replaces whole.
  `Change` is externally tagged because a name is itself a string.
  What `delete` leaves is what a create that left the member out would
  have made. Mounts wait for the container to be inactive, `Active`
  refusing the whole request; the rest change live. `InUse` for a name
  another holds, at byte 3 for agents and byte 4 for tools after
  `NotOwned`.

## 4. Gets, routes `set`, and `self` (`ed52d6dcc` through `52d65b9af`)

- `agents_get`, `agents_templates_get`, `tools_get` and
  `tools_templates_get` join the daemon's tools, each after its list,
  reaching by the list's filter.
- `tools::routes::add` is `tools::routes::set`, with `Set` at byte 0;
  a position is routed once and not again until its route is deleted.
- A reference may be the string `"self"`, the fourth form of both
  `reference` enums, wrapping a one-value `Itself` so it stays a
  string where the other forms are objects. It is for a get alone: an
  agent's `"self"` in `agents_get` when the caller is an agent, a
  tool's in `tools_get` when the caller is a tool. Anywhere else it
  names nothing. The two gets answer `NoSelf` at byte 2, `Error`
  moving to 3.
- The permission is `agents_self: bool` on `agents::create` and
  `tools_self: bool` on `tools::create`, each on its own family's
  frame rather than on `Inner`, with the same two as optional
  `Change<bool>` on the edits. Without it `"self"` names nothing. The
  docs say it is about naming, not names.

## 5. One maker (`276a94aeb` through `18fafb5d4`)

- Every `creator` is one `Creator`, not a chain: on the `Agent`,
  `Tool`, both `Listed` and `Route` items, and as a user part's
  `sender`. Only the direct maker is carried; the maker's own maker is
  on the maker's item. The five `creators` filters match the direct
  maker.
- A template made again — answered `Exists`, or made anew after a
  delete — keeps its creator and its first `created`; a create of one
  that exists changes nothing.

## 6. Providers (`a02e77a17` through `159dc59d1`)

`endpoints::providers`, two sub-families, spelled as the provider
server spells its own `auth`.

- **Outgoing**, the providers the daemon dials, by address, which is
  the identity. `add` 37 takes `{ address, mode }` with the mode keyed
  by name, `{"unbrokered":{"authorization":…}}`; `get` 38; `list` 39
  narrowing by addresses, kinds, connected, creators and a created
  span, its item carrying `kind` without the credential, `connected`,
  `last_connected`, `created` and `creator`; `delete` 40 refusing
  with `InUse` while a container is pinned to it or uses a volume of
  its; `edit` 41 replacing the mode whole, which is how a credential
  rotates.
- **Incoming**, judges of providers that dial in, tried in the order
  added, the first that accepts deciding. A `Judge` is untagged and
  told apart by its members as the server's are: `{"key":…,"identity":…}`
  with an optional `address`, or `{"authorize_hook":…}`, a
  pre-existing directory resource with `hook.yaml` at its root. The
  family doc states the hook contract: the per-platform argv manifest,
  one line of JSON on stdin with `credential` and `address`, and the
  `{"authorized":…}` document it answers. `add` 42 appends and answers
  `Exists` or `NoResource`; a judge is referenced afterwards by
  `{"identity":…}` or `{"authorize_hook":…}` for `get` 43, `delete` 45
  with `InUse` while a provider is connected through it, and `edit` 46
  replacing a judge with one of its form and keeping its place,
  `Mismatch` otherwise; `list` 44 sends them in the order tried, each
  a `Told`, the judge without its key, with the identities connected
  through it.
- Secrets are given and never answered: no list or get carries an
  authorization or a key.
- Ten daemon tools, `providers_outgoing_*` and `providers_incoming_*`:
  add as a `Switch`, the rest a `Reach` by the matching filter.

## The daemon's tag table at `159dc59d1`

| tags | requests |
|---|---|
| 0–8 | `agents::{create, get, delete, message, logs, list, edit, tag, untag}` |
| 9–14 | `agents::templates::{create, get, list, delete, tag, untag}` |
| 15–24 | `tools::{create, get, edit, connect, attach, detach, delete, list, tag, untag}` |
| 25–27 | `tools::routes::{set, delete, list}` |
| 28–33 | `tools::templates::{create, get, list, delete, tag, untag}` |
| 34–36 | `resources::{upload, list, delete}` |
| 37–41 | `providers::outgoing::{add, get, list, delete, edit}` |
| 42–46 | `providers::incoming::{add, get, list, delete, edit}` |

## Open

- `tools::connect` has no daemon tool, by ruling; every other endpoint
  has one, forty-six in all.
- What each daemon tool says to its caller, and how the daemon knows
  which container is calling, is not yet stated.
- A `"self"` reference outside a get falls under `NotFound`; only the
  two gets answer `NoSelf`.

## Verification across the eighteen

Every commit passed `cargo check -p diverge-sdk` and
`cargo doc --no-deps -p diverge-sdk` with zero warnings, plus a
throwaway round-trip example asserting the JSON shapes and the tag
values, deleted after. Nothing was run live. One commit in this span,
`ffb18243d`, dropped the tool template item's `created` member while
rewriting its doc; cargo's unused-import warning caught it and
`18fafb5d4` restored it.
