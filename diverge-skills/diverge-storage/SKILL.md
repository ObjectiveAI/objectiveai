---
name: diverge-storage
description: Storage in Diverge through the `diverge` CLI — volumes (`diverge run volumes create|get|list|edit|delete|stat|tag|untag|filetree|download|upload|transfer`; modes persistent/ephemeral/read_only, sizes in bytes and the provider's 4 MiB floor, stored vs fixed volumes, mounting a volume into an agent or tool as a volume mount or a FUSE mount, what the provider's volume capabilities mean, `held` and `in_use` answers), the vault (`diverge run vault insert|get|list|delete|tag|untag`; keys, slots, who holds a slot, how a template's `secrets` names bind to vault keys through the agent's or tool's `secrets` map, fixed vs rotating secrets, what happens at activation and at inactive), and postgres (`diverge run postgres get|list`; the one database the daemon serves, each container's own schema, listing open container connections). Load it when you create or resize a volume, put files into or take files out of a volume, mount storage into a container, store an API key or a login for an agent, debug a start error about a secret or a volume, or need to know where a container's database is.
---

# Diverge storage: volumes, the vault, postgres

Three kinds of state outlive a container:

- A **volume** is a directory a *provider* keeps on its disk, mounted into containers. It belongs to
  the provider, is named by `{provider, name}`, and outlives every container and connection.
- The **vault** is the *daemon's* store of secrets: named keys, each holding one or more JSON values
  (slots). Containers are handed values at every activation; they never ask for one and are never
  told a key.
- **Postgres** is the one database the *daemon* serves into its containers. Each container reaches a
  schema of its own with no credential.

Every command is `diverge run <family> <leaf> [BODY]`, the body one JSON object. `request-schema` and
`response-schema` beneath any leaf print the exact shapes and dial nothing. Global flags
(`--daemon`, `--account`, `--jq`, `--timeout`) go right after `run`. Every body also accepts `relay`
(see `diverge-daemons`). Every leaf can answer `"forbidden"` (no grant; see `diverge-access`) and
`{"error":…}`.

Two shapes recur:

```json
{"issuer": "https://identity.diverge.network", "subject": "github.12345.provider"}                               // a provider's identity
{"provider": {"issuer": "https://identity.diverge.network", "subject": "github.12345.provider"}, "name": "data"}  // a volume reference
```

Get a provider's identity from `diverge run providers list` (see `diverge-daemons`). Paths inside a
volume or a container are **arrays of components** (`["site","index.html"]`), never strings; no
component is empty, `.` or `..`, or holds `/`.

---

# Volumes

## What a volume is

- **Stored volume** — one a client created. On the provider it is an ext4 filesystem in a sparse
  image file; its size is *reserved*, not taken. It can be resized, re-moded and deleted.
- **Fixed volume** — a directory the provider's own configuration names (`volumes.fixed` in
  `provider.yaml`). It appears in listings like any other, with a declared size and mode, but the
  provider refuses every edit and never deletes it. A provider may list it only to some identities.
- A provider creates volumes only if its configuration has `volumes.stores`; with none, a create is
  answered `unsupported` or `insufficient_capacity`.

**Mode** is a fact of the volume, set at create and changed only by an edit:

| mode | what a container's writes become | who may hold it at once (at most) |
|---|---|---|
| `persistent` | kept in the volume | any number of running containers sharing one filesystem, **or** one FUSE serve; never both |
| `ephemeral` | discarded with the container; every container starts from the volume as it is | any number of containers and FUSE serves |
| `read_only` | refused inside the container | any number of containers and FUSE serves |

Uploads at rest land in all three modes; the mode governs containers, not `volumes upload`.

**Size** is `bytes`. The provider refuses fewer than **4 MiB (4194304)**: less cannot hold the
filesystem's journal. A too-small size comes back as `{"error":…}`.

**At rest, or held.** A volume is read, written, walked, resized and deleted only *at rest*. While a
running container has it, or while a download, upload or transfer is on it, those requests answer
`"held"`: wait and ask again. Agents are stopped after the daemon's `idle_seconds` (default 10) of
disuse, so a volume an idle agent mounted frees itself shortly.

## Create

When: before the first agent or tool that needs disk state.

```bash
diverge run volumes create '{
  "provider": {"issuer":"https://identity.diverge.network","subject":"github.12345.provider"},
  "name": "data", "bytes": 1073741824, "mode": "persistent"}'
```

All four members are required. `name` is the handle forever (it cannot be renamed): one path
component, not starting with `.`, with no `/`, `\` or NUL, and not a fixed volume's name.

| answer | meaning |
|---|---|
| `"created"` | the volume exists and a list shows it |
| `"no_provider"` | no provider on record has that identity |
| `"exists"` | that provider already lists a volume of the name; nothing changed |
| `"insufficient_capacity"` | the provider cannot reserve that many bytes; ask smaller |
| `"unsupported"` | the provider keeps no volumes, or none of that mode; ask another provider |
| `{"error":…}` | could not ask (provider not connected) or the provider refused (bad name, under 4 MiB) |

Grant: `{"volumes":["create"]}`.

## Get, list, stat

```bash
diverge run volumes get '{"volume":{"provider":{…},"name":"data"}}'
diverge run volumes list                                   # every volume the grants reach
diverge run volumes list '{"modes":["persistent"],"mounted":false,"count":20}'
diverge run volumes list --watch '{"all_tags":["prod"]}'   # keep the stream open
diverge run volumes stat '{"volume":{"provider":{…},"name":"data"}}'
```

A volume as `get` (`{"found":{…}}`) and `list` (`{"added":{…}}`) report it:

```json
{"provider":{"issuer":"…","subject":"github.12345.provider"},"name":"data","bytes":1073741824,"mode":"persistent",
 "created":"2026-11-10T09:00:00Z",
 "agents":[{"template":"…","index":0,"name":"worker"}],
 "tools":[{"record":{"template":"…","index":0,"name":"notes"}}],"tags":["prod"]}
```

`agents` and `tools` are the records that name the volume in their mounts, running or not (absent
when empty; a tool is `{"record":{…}}` for a standalone tool or `{"dependency":{…}}` for a dependency
tool). There is no creator: a volume is the provider's.

`list` filter members, all optional: `providers` (identities), `names`, `modes`, `mounted`
(true/false: whether any agent or tool names it in its mounts), `all_tags`, `any_tags`,
`created_from`, `created_to` (RFC 3339), and `count` (a cap). It prints one `{"added":…}` per volume,
provider by provider and by name, then `"listed"`, and ends; with `--watch` it stays open and prints
`{"added":…}`, `{"changed":…}`, `{"removed":…}` as things change. Only volumes of **connected**
providers appear: a provider on record but not connected contributes nothing.

`stat` walks the volume (so it needs the volume at rest) and answers what a listing leaves out:
`{"stat":{"bytes_used":123456,"dirhash":"h1:…"}}` — bytes in use and a content hash (Go's `dirhash`
`h1:` form; equal hashes mean equal content). Other answers: `"not_found"`, `"held"`.

Grants: `get`, `list`, `stat` actions over the volume.

## Edit (size, mode)

```bash
diverge run volumes edit '{"volume":{"provider":{…},"name":"data"},
  "patch":[{"op":"replace","path":"/bytes","value":2147483648}]}'
diverge run volumes edit '{"volume":{…},"patch":[{"op":"replace","path":"/mode","value":"read_only"}]}'
```

`patch` is a JSON Patch (RFC 6902) applied to the document `{"bytes":…,"mode":…}`; those two members
are the whole of what can change. Both in one patch are one edit: a size the provider refuses changes
the mode no more than the size. An empty patch, or one that changes nothing, is `"edited"` with
nothing asked.

| answer | meaning |
|---|---|
| `"edited"` | the volume is as the patch states |
| `"not_found"` | no such volume |
| `"insufficient_capacity"` | the provider cannot reserve that many bytes |
| `"content_too_large"` | the volume holds more than the smaller size asked |
| `"held"` | a running container has it, or a download/upload/transfer is on it |
| `{"error":…}` | a patch that does not apply, a fixed volume, under 4 MiB, a provider failure |

Changing the mode changes what every existing mount of the volume means: a mount states the mode it
expects, and a mismatch is a start error (see mounting). Grant: `edit`.

## Delete

```bash
diverge run volumes delete '{"volume":{"provider":{…},"name":"data"}}'
```

Irreversible: the content is gone. Answers: `"deleted"`, `"not_found"`, `"in_use"`, `"forbidden"`,
`{"error":…}`. **`in_use`** means some agent or tool of the daemon's names the volume in its mounts —
whether or not it is running — or a download, upload or transfer is on it now. Find the mounters with
`volumes get` (`agents`, `tools`), take the volume out of their mounts (an edit of the agent or tool)
or delete them, and ask again. To get space back and keep the name, delete and create. Grant:
`delete`.

## Tag, untag

```bash
diverge run volumes tag   '{"volume":{…},"tags":["prod","team-a"]}'     # "tagged"
diverge run volumes untag '{"volume":{…},"tags":["team-a"]}'            # "untagged"
```

Tags are the daemon's, kept beside the provider's listing and dropped when the provider stops listing
the volume. Tagging a tag already there, or removing one not there, is not a failure. Tags are what
grants narrow by (`within`), so tag volumes you want to delegate. Answers also include `"not_found"`.
Grant: the tagging shape, `{"volumes":{"actions":["tag","untag"],"within":"any","tags":"any"}}`.

## Files on a volume: filetree, download, upload, transfer

All four need the volume **at rest** (`"held"` otherwise) and hold it for their length.

```bash
# what is in it: one snapshot of the tree (path [] is the whole volume)
diverge run volumes filetree '{"volume":{…},"path":[]}'
diverge run volumes filetree '{"volume":{…},"path":["site"]}'

# a file or a directory out of it, to a local path
diverge run volumes download --path ./index.html '{"volume":{…},"path":["site","index.html"]}'
diverge run volumes download --path ./backup     '{"volume":{…},"path":[]}'

# one local file in, at a path
diverge run volumes upload --path ./config.json '{"kind":"file","volume":{…},"path":["etc","config.json"]}'

# several files of a local directory in, under a path ([] is the volume's root)
diverge run volumes upload --path ./site '{"kind":"directory","volume":{…},"path":["site"],
  "files":["index.html","css/main.css"]}'

# copy from the volume into an agent, a tool, a dependency tool or a volume; bytes never reach you
diverge run volumes transfer '{"volume":{…},"path":["site"],"to":{"agent":{"name":"worker"},"path":["srv","site"]}}'
diverge run volumes transfer '{"volume":{…},"path":["site"],"to":{"volume":{"provider":{…},"name":"backup"},"path":["site"]}}'
```

- **filetree** answers `{"tree":[…]}`: nodes with `type` `file` (`name`, `size`, times), `directory`
  (`name`, `children`) or `symlink` (`name`, `path`). `"not_directory"` when a file is at the path;
  `"not_found"` when nothing is. It is a snapshot; nothing is watched.
- **download** `--path` is required: the local file a file download is written to, or the local
  directory a directory download is written under (parents made). It prints one
  `{"path":[…],"bytes":N}` per file written, or `"not_found"` / `"held"`. A directory with no file
  prints nothing.
- **upload** `--path` is required: the local file, or the local directory each name in `files` is
  read under. `files` are `/`-separated paths relative to that directory, at least one, none twice;
  **the CLI uploads only the files you list**, it does not walk the directory. Each file lands whole,
  parents made, replacing what was at its path and touching nothing else. Answers `"uploaded"`,
  `"not_found"`, `"held"`.
- **transfer** `to` is exactly one of `{"agent":…,"path":[…]}`, `{"tool":…,"path":[…]}`,
  `{"dependency":{"template":…,"owner":…},"path":[…]}`, `{"volume":…,"path":[…]}`; the destination
  `path` has at least one component. A file lands at `path`; a directory lands *as* `path`, nothing
  already there removed. An agent or tool that is not running is started for the copy and stopped
  after; a dependency tool must already be running. Answers `"transferred"`, `"not_found"`,
  `"no_destination"`, `"held"` (either end).

Grants: `filetree`, `download`, `upload`, `transfer` over the source volume; a transfer's
destination is judged by its own grant (`upload` over a destination volume).

## Mounting a volume into an agent or a tool

Mounts are stated on the **create** of an agent or a tool (or a later edit), never on a template.
Naming a volume in a mount takes the `mount` grant over it. Two ways:

1. **Volume mount** — the container is pinned to the volume's provider and the volume is mounted
   directly:

   ```json
   {"template":"<id>","name":"worker",
    "provider":{"identity":{"issuer":"…","subject":"github.12345.provider"},
      "volume_mounts":[{"volume_name":"data","volume_relative_path":[],
                        "volume_mode":"persistent","container_path":["data"]}]}}
   ```

2. **FUSE mount** — a file or a directory of *any* provider's volume, served live across the daemon:

   ```json
   {"template":"<id>","name":"worker",
    "fuse_directory_mounts":[{"provider":{"issuer":"…","subject":"github.12345.provider"},"volume_name":"creds",
        "volume_relative_path":[],"volume_mode":"persistent","container_path":["root",".config","tool"]}],
    "fuse_file_mounts":[{"provider":{…},"volume_name":"creds","volume_relative_path":["token.json"],
        "volume_mode":"ephemeral","overlay_disk":16777216,"container_path":["etc","token.json"]}]}
   ```

Key rules: `volume_mode` must equal the volume's actual mode or the start fails; `overlay_disk` is
required for an `ephemeral` FUSE mount and forbidden otherwise; no mount path may be the root or lie
inside another mount; a persistent volume cannot be FUSE-served while a container mounts it or
another serve holds it. Full rules, the capability flags and every start error are in
[references/mounting.md](references/mounting.md). The create and edit commands themselves are in
`diverge-agents` and `diverge-tools`.

---

# The vault

## What it is

A **vault key** is a name and one or more values, each any JSON, numbered from `0` in the order they
were appended: its **slots**. Secrets reach a container through three statements by three parties:

1. The **template** lists the secret *names* the image needs: `"secrets":["OPENROUTER_API_KEY"]`.
2. The **agent or tool** made from it binds every name to a vault key on its create:
   `"secrets":{"OPENROUTER_API_KEY":"or-key"}`.
3. The **program**, at registration, declares which names it takes and each one's **mode**.

| mode | at every activation | at inactive |
|---|---|---|
| `fixed` | a **random** slot's value is read and handed over; nothing is locked; many containers may read one value | nothing |
| `rotating` | a random **free** slot is **locked** to the record (the agent, standalone tool or dependency tool) and its value handed over | the daemon collects what the program made of it, writes that into the same slot, and drops the lock |

So an API key is `fixed`; a login whose refresh token is single-use (an OAuth state document, an
`auth.json`) is `rotating`, and the refreshed login is what the next activation finds. A lock is on a
*value*, never on a key: a key with three values serves three rotating holders at once. Which secrets
an image declares, and in which mode, is in `diverge-images`.

Binding rules (checked at the agent's or tool's create or edit):

- The `secrets` map must bind **every name the template declares and no other**.
- Each key must exist in the vault **with at least one value**, and binding it takes the `use` grant
  over it. Otherwise the create answers its secrets error.
- A dependency tool's secrets are resolved through the binding of the **agent** that spawns it, so a
  dependency's secret names must be names the agent's template declares.

What can go wrong at a start (shown as the start error in the agent's or tool's log):

- "the program takes the secret X, which the template does not declare" — the image's arguments
  select a secret the template omits. Make a template that lists it.
- "…bound to the vault key K, which holds no value" — the key was emptied.
- "…bound to the vault key K, no value of which is free" — every slot of a rotating secret's key is
  held by a running container. Add a slot (`vault insert` with no `slot`) or wait for one to go
  inactive.

## Insert (make a key, append a value, replace a value)

```bash
diverge run vault insert '{"key":"or-key","value":"sk-or-…"}'                       # new key: slot 0
diverge run vault insert '{"key":"codex-logins","value":{"auth_mode":"chatgpt","tokens":{…}}}'
diverge run vault insert '{"key":"codex-logins","value":{…}}'                       # existing key: appends slot 1
diverge run vault insert '{"key":"or-key","slot":0,"value":"sk-or-NEW"}'            # replaces slot 0
```

`value` is any JSON, held as given: a string for an API key, an object for a login document — the
shape the image's secret expects. Without `slot` the value is appended (making the key if it is new);
with `slot` that slot's value is replaced.

| answer | meaning |
|---|---|
| `"inserted"` | the key holds the value |
| `"not_found"` | a `slot` was named that the key does not have, or the key does not exist |
| `"locked"` | a running container holds that slot as a rotating secret; wait, or name another slot |

A bound key is inserted into like any other: the next activation may be handed the new value. This
is how a key is rotated by hand. The value in a shell command lands in shell history; prefer reading
it from a file (`"$(cat body.json)"`). Grants: a **new** key needs the make `{"vault":["insert"]}`;
an **existing** key needs the `insert` action over it.

## Get and list

```bash
diverge run vault get '{"key":"or-key"}'
diverge run vault list
diverge run vault list '{"keys":["or-key"],"any_tags":["prod"],"count":50}'
diverge run vault list --watch
```

`get` is **the one request that answers values**:

```json
{"found":{"key":"codex-logins",
  "slots":[{"slot":0,"locked":{"type":"agent","template":"…","index":0,"name":"coder"},"value":{…}},
           {"slot":1,"value":{…}}],
  "agents":[{"template":"…","index":0,"name":"coder"}],
  "standalone_tools":[{"record":{"template":"…","index":0,"name":"notes"}}],"tags":["prod"],
  "created":"2026-11-10T09:00:00Z","creator":{"type":"client","identity":{…}}}}
```

`locked` names the record whose running container holds that slot (absent when free). `agents` and
`standalone_tools` are the records **binding** the key. Use `--jq` to avoid printing values you do
not need, e.g. `--jq '.found.slots[] | {slot, locked}'`.

`list` prints the same per key **without any value** (`{"added":{"key":…,"slots":[{"slot":0,"locked":…}],…}}`),
oldest first, then `"listed"`. Filter members: `keys`, `creators`, `all_tags`, `any_tags`,
`created_from`, `created_to`, and `count`. With `--watch` it stays open and reports locks taken and
dropped as `{"changed":…}`, which is how to watch rotating secrets being held. Grants: `get`, `list`.

## Delete

```bash
diverge run vault delete '{"key":"or-key"}'               # the whole key
diverge run vault delete '{"key":"codex-logins","slot":1}'  # one value
```

| answer | meaning |
|---|---|
| `"deleted"` | the key, or the value named, is gone |
| `"not_found"` | no such key, or no such slot |
| `"in_use"` | (whole key) an agent or a tool binds it in its `secrets`; unbind or delete those first |
| `"locked"` | a running container holds the value named, or one of the key's values; wait for it to go inactive |
| `"last"` | the slot is the key's only one; a key holds at least one value, so delete the key instead |

Deleting a slot does not renumber what `get` reported for the others in a way you should rely on:
read the key again before naming a slot. Grant: `delete`.

## Tag, untag

```bash
diverge run vault tag   '{"key":"or-key","tags":["prod"]}'    # "tagged"
diverge run vault untag '{"key":"or-key","tags":["prod"]}'    # "untagged"
```

Same semantics as volume tags; `"not_found"` for a key the vault does not hold. Grant: the tagging
shape, `{"vault":{"actions":["tag","untag"],"within":"any","tags":"any"}}`.

## A worked example: an agent with an API key

```bash
diverge run vault insert '{"key":"or-key","value":"sk-or-…"}'
# the template (see diverge-images / diverge-agents) lists "secrets":["OPENROUTER_API_KEY"]
diverge run agents create '{"template":"<id>","name":"helper","secrets":{"OPENROUTER_API_KEY":"or-key"}}'
diverge run vault get --jq '.found.agents' '{"key":"or-key"}'      # the agent now binds the key
```

---

# Postgres

The daemon serves exactly **one** database into its containers, and keeps its own records in it too.
Which database is the daemon's configuration (`postgres` in `daemon.yaml`: `{kind: local}`, a
Postgres the daemon runs beside itself, the default; or `{kind: remote, url: …}`), read at start.
Nothing on the wire changes it; see `diverge-config`.

What a container sees: its driver dials its own proxy at `127.0.0.1:81`, user `diverge`, database
`diverge`, **no password**. The daemon authenticates toward the real database for it. What it
reaches is one **schema of its own** (a login role and a schema of the same name, search path pinned
to it): unqualified table names land there, another container's tables are `permission denied`.

Which scope a container gets:

| container | scope |
|---|---|
| an agent | its own, one per agent |
| a standalone tool | its own, one per tool |
| a transient dependency tool | one per parent agent (and per dependency template) |
| a service dependency tool | as far as its `scope` says: per agent (`instance`), per agent template (`template`), or one for all (`global`) |

A scope is made at the container's first connection and dropped only by the delete of what owns it:
a tool's by the tool's delete, a dependency tool's by its dependency template's delete. **An agent's
delete drops no scope.** Two containers never share a scope; share a volume instead.

## `postgres get`: which database

```bash
diverge run postgres get        # {"mode":{"kind":"local"}}  or  {"mode":{"kind":"remote","url":"postgres://user@host:5432/db"}}
```

The body is `{}`. A remote URL is answered with its password taken out. Grant: `{"postgres":["get"]}`.

## `postgres list`: open container connections

```bash
diverge run postgres list
diverge run postgres list --watch
```

One line per connection a container holds open through the database now, oldest first, then
`"listed"`:

```json
{"added":{"id":17,"container":{"agent":{"template":"…","index":0}},"opened":"2026-11-10T09:01:02Z"}}
```

`container` is `{"agent":…}`, `{"tool":…}` (each by template and index) or
`{"dependency":{"template":…,"owner":…}}`. A container holding a pool is listed once per connection,
told apart by `id`. There is no filter and no count. With `--watch`, connections are printed as
`{"added":…}` when they open and `{"removed":…}` when they close. Use it to see which containers are
actually using their database. Grant: `{"postgres":["list"]}`.

---

## Grants at a glance

| family | make (bare array) | over actions (`{"actions":[…],"within":…}`) | tagging |
|---|---|---|---|
| `volumes` | `create` | `get`, `list`, `delete`, `edit`, `stat`, `download`, `upload`, `transfer`, `filetree`, `mount` | `tag`, `untag` |
| `vault` | `insert` (a new key) | `get`, `list`, `insert`, `delete`, `use` | `tag`, `untag` |
| `postgres` | — | bare array: `get`, `list` | — |

`mount` (volumes) and `use` (vault) are not commands: they are what naming a volume in a mount, or a
key in a `secrets` binding, takes. Admins hold everything. Shapes, `within` and diagnosing
`forbidden` are in `diverge-access`.

## Not verified

- No request was run live while writing this (the daemon was down); answers are from the SDK's
  response types, the daemon's handlers and the CLI's `response-schema`.
- What the daemon answers for an edit or delete of a **fixed** volume: the provider refuses it, and
  it is expected to surface as `{"error":…}`.
- Whether remaining slot numbers shift after `vault delete` with a `slot`.
- The exact text of the create's secrets error for an unbound name or an empty key.
- The largest size a volume may have (a resize refuses sizes whose 4 KiB block count exceeds 32
  bits, about 16 TiB; whether create applies the same bound was not traced).

## When this task is done

Load the `diverge` skill again before starting the next Diverge task: it is the index, and it says which
skill the next task needs.
