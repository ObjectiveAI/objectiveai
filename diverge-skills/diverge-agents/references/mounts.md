# Agent mounts: provider pin, volume mounts, FUSE mounts

The same four members appear on `agents create` and `standalone-tools create`, and in the editable document
of `agents edit` and `standalone-tools edit`. Volumes themselves (create, list, modes, resize) are in
`diverge-storage`.

## Two ways a volume reaches a container

| | volume mount | FUSE mount |
|---|---|---|
| where it is stated | `provider.volume_mounts` on the create | `fuse_file_mounts` / `fuse_directory_mounts` on the create |
| whose volume | only the provider the container is pinned to | any provider the daemon knows; each mount names its own |
| how it is served | mounted directly by that provider | every read and write is forwarded by the daemon to the volume's provider, live, nothing buffered |
| cost | none | one round trip per file operation |
| pins the container | yes, to that provider | no |
| granularity | the volume or a subdirectory | one file, or one directory tree |

Use a volume mount for working data. Use a FUSE mount to reach a volume on another provider, or to mount a
single file (for example a credential file a vendor CLI rewrites).

## Pinning and volume mounts

```json
"provider": {
  "identity": {"issuer": "http://127.0.0.1:14981", "subject": "provider"},
  "volume_mounts": [
    {
      "volume_name": "notes",
      "volume_relative_path": [],
      "volume_mode": "persistent",
      "container_path": ["data"]
    }
  ]
}
```

- `identity`: the provider as the daemon knows it, `{"issuer","subject"}` (see `diverge run providers list`,
  `diverge-daemons`). A provider the daemon does not know is the create's error.
- `volume_mounts` may be empty: that pins the agent and mounts nothing.
- `volume_name`: the volume's name in that provider's listing. Needs the `mount` grant over the volume.
- `volume_relative_path`: components into the volume; `[]` is the volume itself.
- `volume_mode`: `persistent`, `ephemeral` or `read_only`. It states the mode the mount **expects** the
  volume to be in. It does not set it. A volume in another mode refuses the run, which is the agent's start
  error (in its log), not the create's. Only a `volumes edit` changes a volume's mode.
- `container_path`: components from the container's root. Never the root (`[]` is refused).
- Without `provider`, the daemon chooses a provider at each start and the agent mounts no volume. Its state
  is then only its conversation, which the daemon keeps.

## FUSE mounts

```json
"fuse_file_mounts": [
  {
    "provider": {"issuer": "http://127.0.0.1:14981", "subject": "provider"},
    "volume_name": "creds",
    "volume_relative_path": ["claude", "credentials.json"],
    "volume_mode": "persistent",
    "container_path": ["home", "agent", ".claude", "credentials.json"]
  }
],
"fuse_directory_mounts": [
  {
    "provider": {"issuer": "http://127.0.0.1:14981", "subject": "provider"},
    "volume_name": "shared",
    "volume_relative_path": [],
    "volume_mode": "ephemeral",
    "overlay_disk": 268435456,
    "container_path": ["shared"]
  }
]
```

- Which list a mount is on says whether it is a file or a directory.
- The volume path must exist when the agent is created and be of the right kind (a regular file for a file
  mount, a directory for a directory mount), else the create's error.
- `overlay_disk` (bytes): required exactly when `volume_mode` is `ephemeral`, forbidden otherwise. It caps
  the layer the container's changes land in; the layer goes when the agent's run does.
- A **file mount** can be read and overwritten in place, but never deleted, moved, or replaced by a rename.
  A program that saves by writing a temporary file and renaming it over the original fails at the rename:
  give it a **directory mount** instead.
- A `read_only` volume is served read-only; writes fail inside the container as a read-only filesystem.

## Rules for every mount

- No mount's `container_path` may be the root, equal another mount's, or lie inside another mount's, across
  all three lists.
- Path components are names: not empty, not `.` or `..`.
- Volumes are held for the run's life: from container start until the run ends, the volume's provider
  refuses to read, write, walk, resize or delete it through the volume endpoints. Between runs it is free.
- Any number of containers may mount one `ephemeral` or `read_only` volume at once. A `persistent` volume has
  one holder at a time: a second agent that mounts it fails at its start while the first runs.
- What the create refuses: a provider the daemon does not know, a FUSE mount whose volume or volume path
  does not exist or is the wrong kind, `overlay_disk` present or absent the wrong way round, overlapping
  container paths. What only a start reveals (kept in the agent's log as an `error`, with the message
  answered `{"error":…}`): a mode mismatch, a persistent volume already held, a provider that cannot serve
  the volume. A create that succeeds is therefore not proof that the mounts work: send a message and read
  the log.

## Editing mounts

The editable document is:

```json
{"name": "echo", "volume_mounts": [], "fuse_file_mounts": [], "fuse_directory_mounts": [], "secrets": {}}
```

```bash
# append a FUSE directory mount (the one change a running agent accepts)
diverge run agents edit '{"agent":{"name":"echo"},"patch":[
  {"op":"add","path":"/fuse_directory_mounts/-","value":{
    "provider":{"issuer":"http://127.0.0.1:14981","subject":"provider"},
    "volume_name":"shared","volume_relative_path":[],"volume_mode":"read_only","container_path":["shared"]}}
]}'

# replace all volume mounts (agent must be stopped, and must be pinned to a provider)
diverge run agents edit '{"agent":{"name":"echo"},"patch":[
  {"op":"replace","path":"/volume_mounts","value":[
    {"volume_name":"notes","volume_relative_path":[],"volume_mode":"persistent","container_path":["data"]}]}
]}'

# remove the first FUSE file mount; drop the name
diverge run agents edit '{"agent":{"name":"echo"},"patch":[
  {"op":"remove","path":"/fuse_file_mounts/0"},
  {"op":"replace","path":"/name","value":null}
]}'
```

- Operations: `add`, `remove`, `replace`, `move`, `copy`, `test`, each with a JSON Pointer `path`
  (`/list/-` appends; `~1` escapes `/`). The first failing operation fails the whole patch.
- A running container accepts only FUSE mounts appended; the edit answers once the provider has mounted
  them. Anything else answers `"active"`.
- The pinned provider cannot be changed or added by an edit. An agent created without `provider` can never
  get `volume_mounts`; create a new agent instead.
- A mount named anew needs the `mount` grant over its volume, beside `edit` over the agent.
