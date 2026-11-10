# Mounting volumes into agents and tools

A container reaches a volume through a mount stated on the **create** of the agent or the standalone
tool (or a later edit of it; see `diverge-agents` / `diverge-tools` for those commands). A template
never names a volume or a provider: templates are shareable, and a provider is one daemon's
acquaintance. Naming a volume in any mount takes the `mount` grant over that volume, beside the grant
over the container.

There are two kinds of mount. Choose by where the volume lives and what you need.

| | volume mount | FUSE mount |
|---|---|---|
| stated in | `provider.volume_mounts` | `fuse_file_mounts`, `fuse_directory_mounts` |
| which volumes | only volumes of the **one provider the container is pinned to** | a file or a directory of **any** provider's volume |
| how it is served | the provider mounts the volume's filesystem directly | every read, write, listing, rename is one ask, forwarded by the daemon to the volume's provider |
| cost | native filesystem speed | a round trip per operation |
| typical use | data, workspaces, caches | a credential file or directory a vendor CLI rewrites; state shared across providers |

## Volume mounts (`provider`)

```json
{
  "template": "<template id>",
  "name": "worker",
  "provider": {
    "identity": { "issuer": "http://127.0.0.1:14981", "subject": "provider" },
    "volume_mounts": [
      { "volume_name": "data",   "volume_relative_path": [],          "volume_mode": "persistent", "container_path": ["data"] },
      { "volume_name": "models", "volume_relative_path": ["llama"],   "volume_mode": "read_only",  "container_path": ["opt", "models"] }
    ]
  }
}
```

| member | meaning |
|---|---|
| `provider.identity` | the one provider the container runs on, as `diverge run providers list` reports it. A provider the daemon does not know is the create's error |
| `volume_name` | a volume of **that** provider, by the name its listing gives |
| `volume_relative_path` | components from the volume's root; `[]` mounts the volume itself, anything else a subdirectory |
| `volume_mode` | the mode the mount **expects** the volume to be in (`persistent`, `ephemeral`, `read_only`). It does not set the mode |
| `container_path` | components from the container's root. `[]` (the root) is refused |

- With `provider` present and `volume_mounts` empty, the container is pinned to that provider and
  mounts nothing.
- With `provider` absent, the daemon chooses a provider at each start (the one with the most room
  first) and the container mounts no volume.
- Mounts are applied in order.

## FUSE mounts (`fuse_file_mounts`, `fuse_directory_mounts`)

```json
{
  "template": "<template id>",
  "name": "worker",
  "fuse_directory_mounts": [
    { "provider": { "issuer": "http://127.0.0.1:14981", "subject": "provider" },
      "volume_name": "creds", "volume_relative_path": ["codex"],
      "volume_mode": "persistent", "container_path": ["root", ".codex"] }
  ],
  "fuse_file_mounts": [
    { "provider": { "issuer": "http://127.0.0.1:14981", "subject": "provider" },
      "volume_name": "config", "volume_relative_path": ["settings.json"],
      "volume_mode": "ephemeral", "overlay_disk": 16777216,
      "container_path": ["etc", "app", "settings.json"] }
  ]
}
```

| member | meaning |
|---|---|
| `provider` | the provider that holds the volume (each mount may name a different one; it may be the provider the container runs on) |
| `volume_name` | the volume, by its listed name. A name the provider holds no volume by is the create's error |
| `volume_relative_path` | where in the volume. A file mount names an existing **regular file**, a directory mount an existing **directory**; anything else is the create's error |
| `volume_mode` | the mode the mount expects, as for a volume mount |
| `overlay_disk` | bytes the serve's own layer may hold. **Present exactly when `volume_mode` is `ephemeral`**, absent otherwise; either way round is the create's error |
| `container_path` | where it appears in the container |

- **File mount**: one regular file that can be read and overwritten in place but never deleted,
  moved or replaced by a rename. A program that saves by writing a temporary file and renaming it
  over the original fails at the rename. Use a **directory mount** for such programs.
- **Directory mount**: the whole tree; every entry can be created, overwritten, renamed and deleted.
  Only its root is fixed.
- The mount point is made if absent, with every missing parent directory. Nothing is copied in or
  buffered: what the container sees is the volume as it is.
- The daemon opens one serve per distinct provider and volume for the container's run. The volume is
  held from the container's start until its run ends; between runs it is free.

## Rules for every mount

- No component of any path is empty, `.` or `..`.
- No `container_path` is the root, equals another mount's, lies inside another mount's, or has
  another mount's inside it — across all three lists.
- **The mode must match.** A mount states the mode it means the volume to have; a volume in another
  mode refuses the run (or the FUSE serve) and that is the container's start error. Nothing changes
  a volume's mode but `diverge run volumes edit`. A request that names one volume at several paths
  names one mode for it.
- What the container's writes become follows the volume's mode: kept (`persistent`), discarded with
  the container or the serve (`ephemeral`), refused as a read-only filesystem (`read_only`).
- A volume is not deleted while any agent or tool names it in a mount, running or not
  (`volumes delete` answers `in_use`).
- While a container runs with a volume, `volumes stat`, `edit`, `filetree`, `download`, `upload` and
  `transfer` on that volume answer `held`.

## What the provider's capabilities mean

A provider states, per mode, what it does with a volume of that mode. See them in
`diverge run providers list`, inside a connected record's `status` (its `info`'s `capabilities`), as `volumes_persistent`,
`volumes_ephemeral` and `volumes_read_only`, each an object of booleans (an absent member is
`false`; an object with every member `false` is a mode the provider keeps no volumes in).

| capability | `true` means | when `false` |
|---|---|---|
| `mount` | a container may mount a volume of the mode | a run naming one is refused: the provider is passed over, or, pinned, the start fails ("the provider mounts no volume whose mode is …") |
| `fuse_serve` | a FUSE serve may hold a volume of the mode | a FUSE mount of one fails at start ("the provider FUSE serves no volume whose mode is …") |
| `mount_many` | more than one running container may mount one volume of the mode at once | a second container's start is refused while the first runs |
| `fuse_serve_many` | more than one FUSE serve may hold one volume of the mode at once | a second serve is refused |
| `mount_and_fuse_serve` | a running container and a FUSE serve may hold one volume of the mode at once | whichever comes second is refused |

The most a mode allows, whatever the provider:

| mode | `mount_many` | `fuse_serve_many` | `mount_and_fuse_serve` |
|---|---|---|---|
| `persistent` | may | never | never |
| `ephemeral` | may | may | may |
| `read_only` | may | may | may |

So a persistent volume can be shared by many containers on its own provider as volume mounts (one
filesystem, every writer visible to the others), but has at most **one** FUSE serve and never a
serve beside a running container. Since the daemon opens one serve per container run, two running
containers cannot both FUSE-mount one persistent volume; use an ephemeral or read-only volume, or a
volume mount on the volume's own provider.

Other capabilities that matter here: `containers` (the provider runs containers at all),
`images_registries` and `images_store` (see `diverge-images`). `volumes create` answers
`unsupported` for a provider that keeps no volume of the mode asked.

## Start errors from mounts, and what to do

A mount problem that the create cannot see surfaces when the container starts, as a start error in
the agent's or tool's log (`diverge run logs`, see `diverge-logs`), and the message that caused the
start is not answered `delivered`.

| cause | fix |
|---|---|
| The volume is in another mode than the mount states | Edit the mount's `volume_mode` on the agent or tool, or `volumes edit` the volume's mode |
| The volume is held exclusively (a stat, download, upload, transfer, filetree, edit or delete is on it) | Wait for the operation to end and message again |
| Another running container has the volume and the provider's `mount_many` is false | Stop the other container (it stops by itself after `idle_seconds`), or use another volume |
| A FUSE serve has the volume and the mode or provider does not allow a mount beside it | Wait for the serving container's run to end |
| A persistent volume is asked to be FUSE-served while served or mounted by anyone | Same; or switch that mount to a volume mount |
| The provider does not mount, or does not FUSE serve, volumes of that mode | Use another mode or another provider |
| The volume no longer exists, or the provider is not connected | Recreate it under the same name, or reconnect the provider |
| An ephemeral FUSE serve's `overlay_disk` cannot be set aside | Ask for fewer bytes, or free overlay disk on the provider |

## A worked example: a workspace volume and a shared read-only dataset

```bash
P='{"issuer":"http://127.0.0.1:14981","subject":"provider"}'

diverge run volumes create "{\"provider\":$P,\"name\":\"ws-alice\",\"bytes\":2147483648,\"mode\":\"persistent\"}"
diverge run volumes create "{\"provider\":$P,\"name\":\"dataset\",\"bytes\":1073741824,\"mode\":\"persistent\"}"

# fill the dataset at rest, then freeze it
diverge run volumes upload --path ./data "{\"kind\":\"directory\",\"volume\":{\"provider\":$P,\"name\":\"dataset\"},\"path\":[],\"files\":[\"train.csv\",\"README.md\"]}"
diverge run volumes edit "{\"volume\":{\"provider\":$P,\"name\":\"dataset\"},\"patch\":[{\"op\":\"replace\",\"path\":\"/mode\",\"value\":\"read_only\"}]}"

# an agent pinned to the provider, mounting both
diverge run agents create "{\"template\":\"<id>\",\"name\":\"alice\",\"provider\":{\"identity\":$P,\"volume_mounts\":[
  {\"volume_name\":\"ws-alice\",\"volume_relative_path\":[],\"volume_mode\":\"persistent\",\"container_path\":[\"workspace\"]},
  {\"volume_name\":\"dataset\",\"volume_relative_path\":[],\"volume_mode\":\"read_only\",\"container_path\":[\"data\"]}]}}"

# later, with the agent idle (stopped), look at what it wrote
diverge run volumes filetree "{\"volume\":{\"provider\":$P,\"name\":\"ws-alice\"},\"path\":[]}"
```

## Not verified

- The exact wording of each start error as it appears in a log; the causes above are the provider's
  refusal variants (`VolumeMode`, `VolumeHeld`, `VolumeMounted`, `VolumeFuseServed`,
  `VolumeMountUnsupported`) and the daemon's capability checks.
- Which capability flags the stock provider reports for each mode on each platform; read them from
  `providers list` rather than assuming.
