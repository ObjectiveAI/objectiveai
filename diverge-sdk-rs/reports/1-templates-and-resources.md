# Report 1: templates and resources in the daemon protocol

Covers the seven commits of 2026-09-30 and 2026-10-01 on
`p2p-provider-binary`, `57b71317d` through `6ab52a61d`, all in
`diverge-sdk-rs/src/daemon`. The uncommitted tag and grant work is
not covered.

| commit | what |
|---|---|
| `57b71317d` | agent templates: create, list, delete at tags 6–8; `agents::create` names one; tools shift to 9–15 |
| `6e476d184` | resources: upload, list, delete, by dirhash; a template mounts them over FUSE |
| `3db587f61` | a template names no provider; the pin moves to the agent's create |
| `c56c3acc0` | resource mounts split into file and directory; the mode is an enum |
| `6781ff250` | tool templates at tags 16–18, typed `"agent"` / `"tool"`, one shared definition; `tools::create` names one; resources shift to 19–21 |
| `bc791d060` | the shared template shape moves from `endpoints::template` to `daemon::template` |
| `6ab52a61d` | one doc link follows the move |

## 1. Agent templates (`57b71317d`)

Before, `agents::create` carried the image, the limits, the mounts
and the arguments inline. Now an agent is made FROM a template, and
the create names it by id.

- `agents::templates::create` (tag 6): the template; answers
  `Created(id)` 0, `Exists(id)` 1, `Error` 2. Two identical templates
  are one; a create of one that exists answers the same id.
- `agents::templates::list` (tag 7): streams `Listed { id, created,
  template }`.
- `agents::templates::delete` (tag 8): `{ id }`; answers `Deleted`,
  `NotFound`, `InUse` (some agent of the caller's was made from it),
  `Error`.
- `agents::create` (tag 0) becomes `{ template, …, name }`; the list
  item `Agent` gains `template`.
- The id is the lowercase hex SHA-256 of the template's compact JSON,
  members in declared order, absent members omitted, no whitespace:
  the hash Go's `dirhash` writes for one file. Sixty-four characters.
- Tools shift: create 9, edit 10, connect 11, attach 12, detach 13,
  delete 14, list 15.

## 2. Resources (`6e476d184`)

A resource is a file or a directory the caller uploads once and the
daemon holds by its hash, served over FUSE into every agent made from
a template that mounts it. The daemon serves these mounts itself, on
the agent's run scope, as the caller's FUSE server; no provider
volume stands behind them.

- `resources::upload`: request tagged `kind`, `File` or
  `Directory { files }` (a manifest). The daemon asks for each file's
  bytes on a server channel, `Content { path: Option<String> }`; the
  client answers on the provider protocol's `volumes::write` client
  channel response shape, in chunks of at most 2 MiB. Answers
  `Uploaded(id)`, `Exists(id)`, `Error`.
- `resources::list`: streams each resource's id and kind.
- `resources::delete`: `{ id }`; refused while any agent mounts it.
- Ids: a file is the hex SHA-256 of its bytes; a directory is `h1:`
  plus the base64 SHA-256 of its summary lines, `<hex>  <path>\n`
  sorted bytewise, which is Go's `dirhash` exactly.
- `Kind { File, Directory }`.
- A template gains `fuse_file_mounts` and `fuse_directory_mounts`,
  each naming a resource, a mode and a `container_path`.

## 3. No provider on a template (`3db587f61`)

A template is meant to travel: handed from one caller to another and
hashed the same anywhere. A provider is one daemon's acquaintance, so
the pin leaves the template. `agents::create` gains
`provider: Option<Provider>`, the one provider the agent runs on with
the volumes it mounts there; absent, the daemon chooses and mounts
nothing. `agents::edit` wording follows.

## 4. Resource mounts, typed (`c56c3acc0`)

- `ResourceMode`, tagged `mode`, flattened into the mount:
  `ReadOnly`, or `Ephemeral { overlay_disk }`. The cap exists exactly
  when the mode is ephemeral; a read-only mount cannot carry one.
- `ResourceFileMount { resource, mode, container_path }`: one file at
  one path, overwritten in place only.
- `ResourceDirectoryMount { resource, resource_relative_path, mode,
  container_path }`: a directory resource or a subtree of it; the
  relative path is components from the resource's root, empty for the
  whole, and names a directory some file's path passes through.
- A file resource in a directory mount, or the reverse, is the
  agent create's error.

## 5. Tool templates, one shared definition (`6781ff250`)

Tools get the same treatment as agents, and the two families share
one shape.

- `Template<Type> { type, image, memory, disk, fuse_file_mounts,
  fuse_directory_mounts, arguments }`, `type` first so the hashed
  JSON leads with it. `AgentType { Agent }` serializes as `"agent"`,
  `ToolType { Tool }` as `"tool"`; neither template decodes as the
  other, and the two never hash the same.
- `agents::templates::Template` and `tools::templates::Template` are
  aliases of the one shape.
- `tools::templates::create` 16, `list` 17, `delete` 18, with the
  agents' answers reworded for tools.
- `tools::create` (tag 9) becomes `{ template, provider,
  fuse_file_mounts, fuse_directory_mounts, name }`, member for member
  the agents create's. The tools list's `Origin::Created` carries
  `template` in place of `image`.
- Resources shift: upload 19, list 20, delete 21. Twenty-two tags.

## 6. Where the shape lives (`bc791d060`, `6ab52a61d`)

The shared vocabulary — `Template`, `ResourceMode`,
`ResourceFileMount`, `ResourceDirectoryMount` — serves nothing
itself, so it moved from `daemon::endpoints::template` to
`daemon::template`, a sibling of `endpoints`. Every path and doc link
follows; the aliases still resolve to it.

## The tag table at `6ab52a61d`

| tag | request |
|---|---|
| 0–5 | `agents::{create, delete, message, logs, list, edit}` |
| 6–8 | `agents::templates::{create, list, delete}` |
| 9–15 | `tools::{create, edit, connect, attach, detach, delete, list}` |
| 16–18 | `tools::templates::{create, list, delete}` |
| 19–21 | `resources::{upload, list, delete}` |

## Verification across the seven

Every commit passed `cargo check -p diverge-sdk` and
`cargo doc --no-deps -p diverge-sdk` with zero warnings, plus a
throwaway round-trip example asserting the tag values and the JSON
shapes, deleted after. Nothing was run live; the daemon SDK is
protocol-only, and no daemon binary exists yet.
