---
name: diverge-daemons
description: Connecting a Diverge daemon to providers and to other daemons. Covers provider records (`diverge run providers add|get|list|edit|delete|enable|disable|tag|untag`, incoming vs outgoing direction, ips and address, the `listable` flag, the daemon's description, status, capabilities and capacity), the peers seen through providers (`diverge run providers daemons list|listen|message`, direct messages), remote daemon records (`diverge run daemons add|get|list|edit|delete|tag|untag`, DaemonId, routes by providers / ips / addresses), the `relay` member every request may carry to have another daemon serve it, and connected standalone tools (`diverge run connected-standalone-tools create|get|list|edit|delete|tag|untag|attach-agent-instance|attach-agent-template|detach-agent-instance|detach-agent-template`), which hold another daemon's tool here for this daemon's agents. Load it when a provider shows `disconnected`, when adding or changing a provider, when making two daemons talk, when a request should run on another daemon, or when an agent needs a tool that lives on another daemon.
---

# Diverge daemons: providers, peers, relay, connected tools

A **provider** is where containers run and volumes live; a daemon works with the providers it has records
of. A provider is also the meeting point between daemons: two daemons connected to the same provider can
reach each other through it. A daemon keeps **remote daemon records** (one account of ours on another
daemon), may serve any request **on** such a remote (`relay`), and may hold a tool that runs on a remote
under a local name for its own agents (a **connected standalone tool**).

All commands are `diverge run <family> <request> '<JSON body>'`, one compact JSON line per answer. Run
`<command> request-schema` / `response-schema` for exact shapes. An identity is always
`{"issuer":"…","subject":"…"}`. Grants named below are explained in `diverge-access`.

**What has never been run live:** everything daemon-to-daemon. Remote daemon records connecting, `relay`,
connected standalone tools, and direct messages (`providers daemons listen|message`) are built and
compile-checked only. Provider records with an outgoing provider are live-proven. Expect to debug the
two-daemon flow; see "Not verified" at the end.

## 1. Provider records (`diverge run providers …`)

One record per provider identity. The record says which end dials, whether this daemon lets the provider
list it to others (`listable`), and whether the daemon works with it (`enabled`).

### Add

```bash
# Outgoing: the daemon dials the provider. Dialled at once when enabled.
diverge run providers add '{"identity":{"issuer":"http://127.0.0.1:14981","subject":"provider"},
  "direction":"outgoing","address":"127.0.0.1:14979","listable":true,
  "description":"local podman provider","tags":["local"]}'

# Incoming: the provider dials the daemon; admitted only from these source IPs (absent = any, [] = none).
diverge run providers add '{"identity":{"issuer":"https://issuer.example","subject":"edge-provider"},
  "direction":"incoming","ips":[{"type":"cidr","cidr":"10.0.0.0/24"},{"type":"ip","ip":"203.0.113.7"}]}'
# -> "added" | "exists" | "forbidden" | {"error":…}
```

Members: `identity` (required), `direction` with its own members beside it (`address` for `outgoing`: a
`host:port` dialled as `ws://host:port/`, or a full URL; `ips` for `incoming`), `listable` (default
`false`), `enabled` (default `true`), `description`, `tags`. Grant: `{"providers":["add"]}`.

Rules:

- Exactly one direction per provider. An incoming provider is never dialled; an outgoing provider that
  dials in is refused. An IP is never a dial target, and an address is never an IP filter.
- The mutual handshake establishes the provider's identity; an outgoing dial is refused unless the
  identity that answers is the record's.
- The other side must agree: the provider's own `provider.yaml` must accept this daemon's identity (and,
  for an incoming provider, dial this daemon), and each must have its own `identity` in its document, signed in
  under its root. The two need not be at the same issuer. See `diverge-config`.
- The daemon holds at most one connection per provider identity.

### Read: status, capabilities, capacity

```bash
diverge run providers list                      # {"added":{…}} per provider, then "listed"
diverge run providers list '{"connected":true,"enabled":true}'
diverge run providers get '{"identity":{"issuer":"http://127.0.0.1:14981","subject":"provider"}}'   # {"found":{…}}
diverge run providers list --jq '.added? | {subject: .identity.subject, status: .status.type, enabled, listable}'
```

A provider item: `identity`, `direction` (+ `address` or `ips`), `listable`, `enabled`, `description`,
`tags`, `created`, `creator`, `status`, `capacity`.

- `status.type` is `disconnected`, `connected` (with `info`), or `different_version` (with `info`).
  `info` is what the provider answered: `version` and `capabilities`. A provider counts as `connected`
  only once that answer is cached. **`different_version`** means the socket is held but the provider is
  used for nothing (no container, volume, image, or daemon reached through it) until daemon and provider
  are the same version.
- `capabilities` says which optional parts the provider serves: `containers`, `images_store`,
  `images_registries` (the registries it pulls from), `volumes_persistent` / `volumes_ephemeral` /
  `volumes_read_only` (each an object of `mount`, `fuse_serve`, `mount_many`, `fuse_serve_many`,
  `mount_and_fuse_serve`), and `messages` (present only when the provider relays direct messages). A member
  absent is the capability absent. The daemon asks a provider nothing its capabilities exclude.
- `capacity` is the provider's latest answer, polled (`capacity_poll_seconds` in `daemon.yaml`, default
  10): `volumes_disk`, `images_disk`, `container_memory`, `container_disk`, in bytes. Absent unless
  `connected`.
- List filter members: `identities`, `connected`, `enabled`, plus `count`. `providers list --watch` keeps
  the list open: a status or capacity change arrives as `{"changed":{…}}`.

If a record stays `disconnected`: the provider is down or unreachable at `address`; the daemon's identity
is not in the provider's `incoming`; the daemon or the provider has no `identity` in its document, or it is not signed in; or the record is disabled.
The daemon keeps redialling an enabled outgoing provider, so fix the cause and list again.

### Change

```bash
# Edit: every member given replaces the record's; absent members stay.
diverge run providers edit '{"identity":{"issuer":"…","subject":"provider"},"description":"main provider"}'
diverge run providers edit '{"identity":{"issuer":"…","subject":"provider"},"listable":true}'
diverge run providers edit '{"identity":{"issuer":"…","subject":"provider"},"direction":"outgoing","address":"10.0.0.5:14979"}'
# -> "edited" | "not_found" | "forbidden" | {"error":…}

diverge run providers disable '{"identity":{"issuer":"…","subject":"provider"}}'   # -> "done" | "not_found" | "forbidden"
diverge run providers enable  '{"identity":{"issuer":"…","subject":"provider"}}'   # -> "done"
diverge run providers tag     '{"identity":{"issuer":"…","subject":"provider"},"tags":["gpu"]}'   # -> "tagged"
diverge run providers untag   '{"identity":{"issuer":"…","subject":"provider"},"tags":["gpu"]}'   # -> "untagged"
diverge run providers delete  '{"identity":{"issuer":"…","subject":"provider"}}'
# -> "deleted" | "not_found" | "enabled" | "in_use" | "forbidden" | {"error":…}
```

Pitfalls, all of which stop running work:

- **`edit` that changes `direction` or `listable` closes the provider's connection.** Every agent and tool
  container running on it ends (its log's `undeployed` carries `reason: "provider_edited"`), and the
  provider is reached again as the record now stands. Editing only the description closes nothing.
- **`disable` takes down every run on the provider** (`reason: "provider_disabled"`), stops the dial and
  refuses a dial in. `enable` dials or admits it again; containers start again on their next use.
- **`delete` stops nothing and so refuses to**: it answers `enabled` unless the record is disabled first,
  and `in_use` while any agent or tool is pinned to the provider or mounts its volumes. Deleting also
  drops the tags the daemon kept on that provider's volumes.
- `enabled` is changed only by `enable` / `disable`, never by `edit`. The identity is not editable.
- Grants: `get`, `list`, `edit`, `enable`, `disable`, `delete` over providers, and the tagging shape; a
  providers grant's `within` is `"any"` or a tags test over the record's tags.

### `listable` and the description

`listable: true` on a provider's record means: this daemon lets **that provider** list it to the other
parties connected to that provider. It is what makes this daemon reachable by other daemons through that
provider. With `listable: false` (the default) the daemon uses the provider but no other daemon can find
or reach it there.

When listed, the daemon is shown with its **description**: `description:` in `daemon.yaml`, one line of
plain text, at most 132 characters, no control characters, no leading or trailing whitespace. It is sent
only to providers the daemon is listable to. (The `description` on a provider *record* is something else:
the caller's own note about the provider.) Changing the daemon's description is a config change and a
daemon restart (`diverge-config`).

## 2. Peers through providers (`diverge run providers daemons …`)

```bash
diverge run providers daemons list
# {"added":{"issuer":"http://127.0.0.1:14981","subject":"daemon-b",
#           "providers":[{"provider":{"issuer":…,"subject":"provider"},"ip":"127.0.0.1","address":null,"listening":false}],
#           "descriptions":["daemon B"]}}
# "listed"
```

- One item per identity listable on any **connected** provider, with every route to it: each `providers`
  entry (a "via") names the provider, the `ip` of the identity's connection to that provider, the
  `address` the provider dialled it at (absent if the identity dialled the provider), and `listening`
  (whether it listens for direct messages there).
- **This daemon's own identity is never listed.** An empty listing means no *other* listable party shares
  a connected provider with this daemon.
- The item's `issuer` and `subject` are what a `daemons add` names as `daemon`.
- `--watch` keeps it open (`added` / `changed` / `removed` as peers come and go). `count` caps it.
- Grant: `{"providers_daemons":{"actions":["list"],"within":"any"}}`. Only routes through providers the
  grant reaches are shown.

### Direct messages

A daemon can send another listable party an opaque message through a shared provider. Neither the daemon
nor the provider reads the body. This needs the provider to relay messages at all (`messages:` in
`provider.yaml`; without it both requests are answered unsupported).

```bash
# Receiver: listen. Runs until the command is ended; prints each message as it arrives.
diverge run providers daemons listen
# {"message":{"provider":{…},"from":{"issuer":…,"subject":"daemon-a"},"body":"aGVsbG8="}}
# {"skipped":{"count":3}}       <- this listener fell behind and missed 3

# Sender: body is standard base64.
diverge run providers daemons message \
  '{"to":{"issuer":"http://127.0.0.1:14981","subject":"daemon-b"},"body":"'"$(printf hello | base64)"'"}'
# -> "delivered" | "not_listening" | "not_found" | "forbidden" | {"error":…}
```

- `listen` has no history: it hears messages taken after it opened. Every listener hears every message.
  The daemon listens upstream on a provider only while some client listens, and the peer's `listening`
  flag in `providers daemons list` reflects that.
- `message`: the daemon picks the provider (one where the receiver listens and that the caller's grant
  reaches). `delivered` means the receiver's reader took it. `not_listening`: the identity is listed but
  listens nowhere reachable. `not_found`: no connected provider lists the identity. When every candidate
  provider is over its rate, the message waits; use `--timeout` to bound it.
- Grants: `listen` and `message` under `providers_daemons`.

## 3. Remote daemon records (`diverge run daemons …`)

A record says: *that* daemon (`daemon`, the identity it answers as), reached as *this* account of ours
(`identity`, one of this host's signed-in identities, from any issuer), by *these* routes. It is known by
its **DaemonId**: a base62 string of 1 to 11 characters that is the hash of `daemon`, `identity` and the
three route filters. There are no names.

```bash
diverge run daemons add '{"daemon":{"issuer":"http://127.0.0.1:14981","subject":"daemon-b"},
  "identity":{"issuer":"http://127.0.0.1:14981","subject":"daemon-a"},
  "description":"daemon B","tags":["peer"]}'
# -> {"added":"3kTq9ZbW1xY"} | {"exists":"3kTq9ZbW1xY"} | "forbidden" | {"error":…}

# Capture the ID either way:
ID_B="$(diverge run daemons add --jq '.added // .exists' '{…}' | tr -d '"')"

# With route filters (all optional; absent or empty = any route):
diverge run daemons add '{"daemon":{…},"identity":{…},
  "providers":[{"issuer":"http://127.0.0.1:14981","subject":"provider"}],
  "ips":[{"type":"cidr","cidr":"10.0.0.0/8"}],
  "addresses":["10.0.0.5:14980"]}'

diverge run daemons get    '{"id":"3kTq9ZbW1xY"}'      # {"found":{id, description, daemon, identity, providers?, ips?, addresses?, tags, created, creator, connected}}
diverge run daemons list                                 # {"added":{…}} …, "listed"; --watch supported
diverge run daemons list '{"connected":true,"issuers":["http://127.0.0.1:14981"]}'
diverge run daemons edit   '{"id":"3kTq9ZbW1xY","description":"daemon B (staging)"}'   # "edited" | "not_found"
diverge run daemons tag    '{"id":"3kTq9ZbW1xY","tags":["staging"]}'                    # "tagged"
diverge run daemons untag  '{"id":"3kTq9ZbW1xY","tags":["staging"]}'                    # "untagged"
diverge run daemons delete '{"id":"3kTq9ZbW1xY"}'       # "deleted" | "not_found" | "in_use" | "forbidden"
```

Rules:

- **The ID is the target.** Only the description and tags ever change. To reach elsewhere, as another
  account, or by other routes: delete and add, which gives another ID. `edit` takes `description` only;
  any other member fails to parse.
- Adding the same `daemon` + `identity` + routes again answers `exists` with the same ID. Several records
  may name the same remote (another account of ours there, or other routes).
- `identity` and `daemon` may be from different issuers. `identity` must be signed in on this daemon's
  host (`diverge run identities list`).
- **Nothing connects at `add`**, and nothing is checked: a provider named in `providers` need not be
  connected. The first relay or connected tool naming the record connects; `connected` in a list says
  whether a connection is held now. One connection per record, reused.
- **Routes** are filters over the vias `providers daemons list` shows for the remote: `providers` (the via's
  provider is one of them), `ips` (the via's `ip` is in one of them), `addresses` (the via's `address` is
  exactly one of them; a via with no address passes no `addresses` filter). A route must pass every filter
  given. One passing route is chosen at random; if none passes, the remote is not reached, with no fallback.
- `delete` answers `in_use` while a connected tool names the record; delete those tools first. Deleting
  drops a held connection.
- Grants: `{"daemons":["add"]}`; `get`, `list`, `edit`, `delete`, `connect`, `relay`, `oci` over daemons
  (`within` filter members `ids`, `issuers`, `creators`, `connected`, tags, created range); tagging shape.
- There is no `daemons accept` or `daemons connections` request. Whether a daemon accepts other daemons is
  `accept_daemons` in its `daemon.yaml` (default `true`).

### What daemon B needs before A can reach it

1. A and B are connected to a **common provider**, at the daemon's own version.
2. On **B**, that provider's record has **`listable: true`** (so `providers daemons list` on A shows B).
3. B's `daemon.yaml` has `accept_daemons: true` (the default).
4. B has an **account** for the identity A presents (the record's `identity`). **B's administrators do
   not count here**: a daemon reaching another is admitted only as an account. Give that account roles for
   exactly what A should be able to do on B.
5. That account is not already in use by another daemon connection: an account has at most one daemon
   connecting as it at a time, and a second is refused.
6. A's host holds the `identity` signed in. B keeps no list of trusted issuers: its account (item 4) is what
   admits that identity.

## 4. `relay`: serving a request on another daemon

Every request body may carry `relay`, an array of DaemonIds. Absent or empty, the daemon dialled serves
the request itself.

```bash
diverge run agents list  '{"relay":["3kTq9ZbW1xY"]}'                        # B's agents, as our account on B
diverge run info         '{"relay":["3kTq9ZbW1xY"]}'                        # B's version: the cheapest reachability test
diverge run agents message '{"agent":{"name":"echo"},"content":[{"type":"text","text":"hi"}],"relay":["3kTq9ZbW1xY"]}'
diverge run agents list  '{"relay":["3kTq9ZbW1xY","7Qm2"]}'                 # two hops: A -> B -> C; "7Qm2" is B's record of C
```

How it is served:

1. The daemon that reads a request with `relay` does not serve it. It takes the **first ID** off, finds
   *its own* record of that ID, and checks one thing: the caller holds `relay` over that record
   (`{"daemons":{"actions":["relay"],"within":"any"}}`).
2. It connects to that remote (or reuses the record's connection) **as the account the record names**, and
   puts the request there with the first ID removed.
3. The remote judges what is left: the next ID, if any, as that account's `relay` over *its* record; and
   finally the request itself, **by that account's roles on the remote**. The remote sees an ordinary
   request from that account and nothing about who originally sent it.
4. The relaying daemon is a pure conduit: every answer, channel and stream comes back unchanged, so
   streams, `--watch`, uploads, downloads, exec, terminals and MCP connects all work through a relay.

Consequences:

- **Your identity on the remote is the record's `identity`, not you.** Things created through a relay have
  that account as their creator there. Being an administrator on A means nothing on B.
- A refusal looks like the request's own answer: `"forbidden"` (no `relay` grant here, or the remote
  account lacks the grant for the request) or `{"error":…}` (no record of that ID, the remote not listable
  or not reachable, its handshake refused, or the connection lost mid-request). The same `forbidden` can
  come from either daemon; check the local `relay` grant first, then the remote account's roles.
- IDs are per daemon: the second ID of a two-hop relay is a record held by the *first* remote. List them
  with `diverge run daemons list '{"relay":["<first id>"]}'`.
- An old ID finds nothing after its record is deleted and re-added differently.

## 5. Connected standalone tools (`diverge run connected-standalone-tools …`)

A connected tool is a standalone tool that **another daemon** holds and runs, held here under a local name
so this daemon's agents can call it. This daemon never starts, stops, edits or inspects the container: its
image, limits, files, exec, terminal and frontend all belong to the other daemon. Here it can only be
named, attached to agents, tagged, logged and deleted.

```bash
# Hold B's tool "notebook" here as "b-notebook". Connects nothing yet.
diverge run connected-standalone-tools create \
  '{"daemon":"3kTq9ZbW1xY","tool":{"name":"notebook"},"name":"b-notebook"}'
# -> "created" | "in_use" (name taken) | "forbidden" | {"error":…} (no record of that ID)

# The tool may be named on the remote by template and index instead, and may lie further away:
#   "tool":{"template":"<template id on B>","index":1}
#   "route":["7Qm2"]      <- B's record of C; the tool is on the last daemon of daemon + route

diverge run connected-standalone-tools get  '{"tool":{"name":"b-notebook"}}'     # {"found":{…}}
diverge run connected-standalone-tools list                                       # --watch supported
diverge run connected-standalone-tools list '{"daemons":["3kTq9ZbW1xY"],"deployed":true}'

diverge run connected-standalone-tools attach-agent-instance '{"tool":{"name":"b-notebook"},"agent":{"name":"echo"}}'
diverge run connected-standalone-tools attach-agent-template '{"tool":{"name":"b-notebook"},"template":"<agent template id>"}'
# -> "attached" | "no_tool" | "no_agent" / "no_template" | "forbidden"
diverge run connected-standalone-tools detach-agent-instance '{"tool":{"name":"b-notebook"},"agent":{"name":"echo"}}'
diverge run connected-standalone-tools detach-agent-template '{"tool":{"name":"b-notebook"},"template":"<agent template id>"}'
# -> "detached" | "no_tool" | "no_agent" / "no_template" | "forbidden"

# Rename (JSON Patch over {"name":…}; null = no name)
diverge run connected-standalone-tools edit '{"tool":{"name":"b-notebook"},"patch":[{"op":"replace","path":"/name","value":"notes"}]}'
# -> "edited" | "not_found" | "in_use" | "forbidden"

diverge run connected-standalone-tools tag    '{"tool":{"name":"notes"},"tags":["shared"]}'    # "tagged"
diverge run connected-standalone-tools untag  '{"tool":{"name":"notes"},"tags":["shared"]}'    # "untagged"
diverge run connected-standalone-tools delete '{"tool":{"name":"notes"}}'
# -> "deleted" | "not_found" | "attached" (detach everywhere first) | "forbidden"

diverge run logs '{"of":{"type":"connected_standalone_tool","name":"notes"}}'     # every call routed into it
```

What to know:

- **Naming the tool.** A connected tool is referenced by `{"name":…}` or, once and for all, by
  `{"daemon":"<id>","tool":<the tool as the remote names it>,"index":<n>}` (the list item carries all
  three). Names are one namespace across this daemon's standalone *and* connected tools. A tool created
  without a name is reachable only by the long form.
- **`create` checks nothing on the remote.** A remote that is unreachable, or that has no such tool or does
  not allow it, is not the create's error: the tool simply shows `deployed: false` and the agent's calls to
  it fail. Verify the name first through a relay: `diverge run standalone-tools list '{"relay":["<id>"]}'`.
- **One connect per tool, opened on use.** Attaching connects nothing. When an attached agent first calls
  the tool, this daemon connects to the remote (as the record's account) and opens that daemon's
  `standalone-tools connect` on the tool; the remote starts the container if needed and holds it while
  connected. The connect is let go after `idle_seconds` without use. `deployed` = the connect is held;
  `active` = a call is in flight.
- **Attaching works on a running agent**: the tool appears in its tool list at once and the agent is told
  its tool list changed. An agent attached both directly and through its template has the tool once.
  Attach and detach are idempotent.
- **Fixed at create:** `daemon`, `route` and `tool`. `edit` changes the name only.
- A list item: `name`, `daemon`, `route`, `tool`, `index`, `creator`, `created`, `active`, `last_active`,
  `deployed`, `last_deployed`, `attached_agent_instances`, `attached_agent_templates`, `tags`. Filter
  members: `names`, `daemons`, `creators`, `active`, `deployed`, `attached_agent_instances`,
  `attached_agent_templates`, tags, created range, `count`.
- No request of the `standalone-tools` family finds a connected tool, and none here finds a standalone one.

Grants needed:

| Where | Who | Grant |
|---|---|---|
| This daemon | the caller of `create` | `{"connected_standalone_tools":["create"]}` and `connect` over the daemons record |
| This daemon | the caller of an attach / detach | `attach` / `detach` over the connected tool, and `edit` over the agent (or `attach` over the agent template) |
| Remote daemon | the account the record names | `{"standalone_tools":{"actions":["connect"],"within":…}}` reaching the tool; for a `route`, `relay` over each further record on each hop |

## 6. Worked example: A records B, relays, and holds B's tool

Daemon A is the CLI's default daemon; daemon B is `--daemon b` in `cli.yaml`. Both are connected to one
provider with `listable: true`. A's daemon presents `daemon-a`; B's presents `daemon-b`. You are an
administrator on both. (This is step 10 of `reports/SETUP_INSTRUCTIONS_1.md`.)

```bash
ISS=http://127.0.0.1:14981

# --- On B: a role and an account for the identity A will present. B's admins do not count for A.
diverge run --daemon b roles create '{"name":"peer","description":"daemon A on this daemon","grants":[
  {"agents":{"actions":["get","list","logs","message"],"within":"any"}},
  {"agents_templates":{"actions":["get","list"],"within":"any"}},
  {"standalone_tools":["create"]},
  {"standalone_tools":{"actions":["get","list","logs","connect"],"within":"any"}},
  {"standalone_tools_templates":["create"]},
  {"standalone_tools_templates":{"actions":["get","list"],"within":"any"}},
  {"events":["list"]},
  {"images":["info"]}]}'
diverge run --daemon b accounts create '{"identity":{"issuer":"'$ISS'","subject":"daemon-a"},"roles":["peer"]}'

# --- On A: is B visible through a shared provider?
diverge run providers daemons list --jq '.added? | {subject, via: [.providers[].provider.subject]}'
#   {"subject":"daemon-b","via":["provider"]}      (A itself is never listed)

# --- On A: record B, reached as daemon-a.
ID_B="$(diverge run daemons add --jq '.added // .exists' \
  '{"daemon":{"issuer":"'$ISS'","subject":"daemon-b"},"identity":{"issuer":"'$ISS'","subject":"daemon-a"},"description":"daemon B"}' | tr -d '"')"

# --- Relay: A forwards, B serves as daemon-a's account.
diverge run info        '{"relay":["'$ID_B'"]}'       # {"info":{"version":"…"}}
diverge run agents list '{"relay":["'$ID_B'"]}'       # B's agents (or just "listed"); not "forbidden"
diverge run daemons get '{"id":"'$ID_B'"}' --jq '.found.connected'    # true once the first relay connected

# --- A tool on B, made through the relay (creator there: daemon-a's account).
TPL="$(diverge run standalone-tools templates create --jq '.created // .exists' \
  '{"type":"tool","mode":"transient","image":{"digest":"sha256:<digest>","references":[{"registry":"docker.io","name":"diverge-local/memory"}]},
    "memory":268435456,"disk":268435456,"arguments":{},"relay":["'$ID_B'"]}' | tr -d '"')"
diverge run standalone-tools create '{"template":"'$TPL'","name":"notebook","relay":["'$ID_B'"]}'

# --- Hold B's tool on A, attach it to A's agent "echo", use it.
diverge run connected-standalone-tools create '{"daemon":"'$ID_B'","tool":{"name":"notebook"},"name":"b-notebook"}'
diverge run connected-standalone-tools attach-agent-instance '{"tool":{"name":"b-notebook"},"agent":{"name":"echo"}}'
diverge run agents message '{"agent":{"name":"echo"},"content":[{"type":"text","text":"remember that the launch is on Friday"}]}'

# --- See what happened.
diverge run logs '{"of":{"type":"agent","name":"echo"}}'                               # the agent's side
diverge run logs '{"of":{"type":"connected_standalone_tool","name":"b-notebook"}}'    # every call A routed into B's tool
diverge run connected-standalone-tools get '{"tool":{"name":"b-notebook"}}' --jq '.found | {deployed, active, last_deployed}'
diverge run logs '{"of":{"type":"standalone_tool","name":"notebook"},"relay":["'$ID_B'"]}'   # B's own log of the tool
```

Template and tool bodies are covered in `diverge-tools`; image digests in `diverge-images`.

## Troubleshooting

| Symptom | Likely cause |
|---|---|
| Provider record stays `disconnected` | Provider down or wrong `address`; daemon not in the provider's `incoming`; no `identity` in `daemon.yaml` or `provider.yaml`, or it is not signed in; record disabled. |
| Provider is `different_version` | Daemon and provider builds differ; the provider is held but unused. Reinstall so both match. |
| Containers died after a provider edit | `direction` or `listable` changed: the connection was closed (`undeployed` with `provider_edited`). They start again on next use. |
| `providers daemons list` is empty on A | B's record of the shared provider is not `listable: true`; A and B share no connected provider; A's grant reaches no provider. A never lists itself. |
| Relayed request answers `{"error":…}` | No record of that ID; B not listable on a provider A is connected to by a route the record allows; B has no **account** for the identity presented (admins do not count); B has `accept_daemons: false`; the account is already held by another daemon connection; the identity is not signed in on A's host. |
| Relayed request answers `"forbidden"` | Caller lacks `relay` over the record on A, or the account on B lacks the grant for the request itself. |
| Connected tool never `deployed`, agent's calls fail | Wrong tool name on B; B's account lacks `connect` over that tool; B unreachable. Check with a relayed `standalone-tools get`. |
| `daemons delete` answers `in_use` | A connected tool names the record. |
| `message` answers `not_listening` | Nobody runs `providers daemons listen` on the receiving daemon, or the provider relays no messages. |

## Not verified

- **Daemon-to-daemon has never been run live**: remote records connecting, `relay` (single or multi-hop),
  connected standalone tools (create, attach, calls, logs), and `route` on a connected tool. Everything in
  sections 3 to 6 is from the SDK, the daemon source, the CLI schemas and
  `reports/SETUP_INSTRUCTIONS_1.md` step 10, which itself says to expect debugging.
- **Direct messages** (`providers daemons listen|message`) and incoming-direction providers have not been
  run live either.
- Live-checked while writing this (read-only, as an administrator): `providers list` with one connected
  outgoing provider (the status, capabilities and capacity shapes above are real), `providers daemons list`,
  `daemons list` and `connected-standalone-tools list` answering `"listed"` with nothing held.
- Whether a tool on B created by someone other than the relayed account can be held: the source judges the
  remote `connect` by the account's grant reach over the tool (not by who created it), so a
  `within: "any"` connect grant should reach any standalone tool on B. Not exercised.
- The exact example ID `3kTq9ZbW1xY`, the peer listing output and the `messages` capability shape are
  illustrative.

## When this task is done

Load the `diverge` skill again before starting the next Diverge task: it is the index, and it says which
skill the next task needs.
