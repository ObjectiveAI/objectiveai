# The filesystem under `.diverge/`: provider, daemon, postgres

Written 2026-10-08, from the code as it stands in `diverge-provider`,
`diverge-daemon` and `diverge-postgres`. Every path below is relative
to one parent, written `<diverge>`, and every file and directory is
named as the code names it. Nothing here has been observed on a live
host; the three programs have never been run.

## 1. The parent, and the three directories under it

Each program has ONE directory, found the same way by all three:

1. `--config <dir>`, the one argument any of them takes;
2. else an environment variable — `DIVERGE_PROVIDER_CONFIG`,
   `DIVERGE_DAEMON_CONFIG`, `DIVERGE_POSTGRES_CONFIG`;
3. else `.diverge/<program>` under the home directory, `HOME` or,
   on Windows, `USERPROFILE`.

The path is made absolute against the working directory without
following links, and the directory is made if absent. So with no
argument and no variable the three live side by side:

```text
~/.diverge/                         <diverge>
├── provider/                       the provider's directory
├── daemon/                         the daemon's directory
└── postgres/                       diverge-postgres run ON ITS OWN only
```

Two things to hold onto before the trees:

- **The provider and the daemon share nothing on disk.** They share
  the parent and the shape — `config.yaml` at the top of each, the
  program's state under it — and talk to each other over the wire.
  Neither reads the other's directory. A volume's bytes are the
  provider's; a resource's bytes and an agent's log are the daemon's.
- **The daemon's Postgres lives inside the daemon's directory.** The
  daemon runs `diverge-postgres --config <diverge>/daemon/postgres`,
  so `~/.diverge/postgres/` is only ever made by a `diverge-postgres`
  started by hand. Under the daemon, the whole postgres tree in §4 is
  `<diverge>/daemon/postgres/`.

Beside the directories there are two binaries each program expects
NEXT TO ITS OWN EXECUTABLE, not under `<diverge>`:

| Program | Looks for | Why |
|---|---|---|
| `diverge-provider` | `diverge-container-proxy` | bind-mounted into every container it runs; missing is a start error |
| `diverge-daemon` | `diverge-postgres` (`.exe` on Windows) | spawned when `postgres` in `config.yaml` is `{kind: local}` |

## 2. The provider: `<diverge>/provider/`

```text
<diverge>/provider/
├── config.yaml                     read at start; absent = defaults
├── hooks/                          made at start, even when empty
│   └── <name>/                     one folder per hook, named in config.yaml
│       ├── hook.yaml               {windows|macos|linux: [argv…]}, no shell
│       └── …                       the program argv[0] names, resolved in this folder
├── run/                            made at start; the provider's own scratch
│   ├── auth.json                   podman's auth file, 0600: registries' credentials
│   ├── mounts/                     where stored volumes are loop-mounted
│   │   └── <uuid>/                 one mount point per attached volume image
│   └── known_hosts                 macOS/Windows only: the podman machine's ssh key
└── podman_data/                    containers.podman.storage_path, DEFAULT location
    ├── ephemeral/                  scratch of every ephemeral volume serve, swept at start
    │   └── <uuid>                  a sparse file; unlinked at once on Unix
    └── …                           podman's own store (Linux: --root); the machine (macOS/Windows)
```

And, wherever `config.yaml` points them — absolute paths, never under
the provider's directory unless the operator puts them there:

```text
<store.path>/                       one per volumes.stores[]; made at start
└── <identity>/                     made on an identity's first create
    ├── <name>                      the volume: a sparse ext4 image file
    └── .<name>                     its mode: {"mode":"persistent"|"ephemeral"|"read_only"}

<fixed.path>/                       one per volumes.fixed[]; must exist already
└── …                               the directory IS the volume; writes land as
                                    .<name>.<uuid> temporaries renamed into place
```

### 2.1 `config.yaml`

The whole file is optional, every section is optional, and an unknown
key is refused. Every relative path in it resolves against the
provider's directory, never the working directory.

```yaml
port: 14979                         # OUTSIDE_PORT; what peers dial
auth:                               # who may dial in; absent = nobody
  unbrokered:
    - key: {key: "…", identity: "…", address: 203.0.113.7}   # address optional
    - hook: {authorize_hook: "<name>"}                       # hooks/<name>/
clients:                            # whom the provider dials; absent = nobody
  unbrokered:
    - {address: "host:port", key: "…", identity: "…"}
containers:                         # always present; absent = defaults
  podman:
    registries:                     # default: docker.io, ghcr.io, quay.io, anonymous
      - {host: docker.io, credential: {username: "…", password: "…"}}
    storage_path: podman_data       # relative → <provider dir>/podman_data; made at start
    image_cache_disk: 34359738368   # 32 GiB
    container_overlay_disk: 34359738368
    memory: 8589934592              # 8 GiB
  server_images:                    # images the provider holds itself; default none
    - {name: acme/tools, digest: "sha256:…"}
volumes:                            # absent = no volume can exist
  stores:                           # ABSOLUTE paths; capacity 0 refused; made at start
    - {path: /srv/volumes, capacity: 1099511627776}
  fixed:                            # ABSOLUTE paths; must be existing directories
    - {name: datasets, path: /srv/datasets, bytes: 0, mode: read_only, authorize_hook: "<name>"}
```

What each path rule is, in the loader's words: `storage_path` joined
onto the directory when relative and made; a store's or a fixed
volume's path refused when relative; a store made if absent; a fixed
volume refused if not an existing directory.

### 2.2 Who writes what, when

| Path | Made | Written by | Removed |
|---|---|---|---|
| `hooks/` | `config::dir`, every start | the operator | never by the provider |
| `run/mounts/` | deployer start | `Volume::attach` makes `<uuid>/` and loop-mounts an image on it | swept at start (an earlier life's mounts), unmounted when the last container detaches |
| `run/auth.json` | deployer start | rewritten every start from `registries[].credential` | — |
| `run/known_hosts` | tunnel open (macOS/Windows) | emptied every start | — |
| `<storage_path>/` | `config::load` | podman, as `--root` on Linux; the machine's disk elsewhere | a changed path leaves the old store as it was |
| `<storage_path>/ephemeral/` | `Scratch::sweep` at start | one sparse file per ephemeral serve | every entry removed at start |
| `<store>/<identity>/` | first `volumes::create` by that identity | `<name>` image + `.<name>` mode | `volumes::delete` removes both |

The provider also labels every container it runs
`diverge.provider=<provider dir>`, which is how it finds and sweeps
the containers of an earlier life. The podman machine on macOS and
Windows is made under `storage_path` and is handed every store path,
every fixed path, the provider's directory and the proxy's directory
as shares.

## 3. The daemon: `<diverge>/daemon/`

```text
<diverge>/daemon/
├── config.yaml                     read at start; absent = defaults
├── postgres/                       ONLY when postgres is {kind: local}; see §4
├── resources/                      every resource's bytes, by content hash
│   ├── incoming/                   uploads still arriving
│   │   └── <uuid>/                 one per upload; a file resource lands as <uuid>/file
│   └── <id>                        a held resource: a file, or a directory tree
├── agents/                         every agent's log, by record id
│   └── <id>/
│       ├── log                     one JSON ItemWrapper per line, appended
│       └── index                   16 bytes per item: offset u64 BE, length u64 BE
└── overlays/                       an ephemeral mount's own layer, for the run's life
    ├── agent-<id>/                 one per running agent that has one
    │   └── r<n>                    a copy of resource mount n (a file, or a tree)
    └── tool-<id>/                  the same for a running tool
        └── r<n>
```

`resources/`, `resources/incoming/`, `agents/` and `overlays/` are
made at every start. The records themselves — accounts, roles,
providers, agents, tools, templates, routes, resources' metadata,
tags, grants — are not files: they are rows in Postgres, local or
remote, under the daemon's own schema. Volumes have no files here at
all; the daemon mirrors each provider's listing in memory and serves
FUSE mounts of them through the provider.

### 3.1 `config.yaml`

```yaml
port: 14980                         # OUTSIDE_PORT + 1, one above a provider's
postgres: {kind: local}             # or {kind: remote, url: "postgres://user:pw@host:port/db"}
idle_seconds: 10                    # a container unused this long is stopped
```

Three keys, all optional, unknown keys refused. `postgres` is read
once; another database is another start. With `remote` nothing of §4
exists and the daemon dials the URL.

### 3.2 Who writes what, when

| Path | Made | Written by | Removed |
|---|---|---|---|
| `resources/incoming/<uuid>/` | each upload or transfer-in | the receive, hashing as it lands | renamed to `resources/<id>` on success, or removed; an id already held discards the arrival |
| `resources/<id>` | the rename | never after placing | `resources delete` |
| `agents/<id>/{log,index}` | first append (the `Active` item of the first run) | every chunk, every loop word, every start error, under the agent's live lock | `agents delete` |
| `overlays/{agent,tool}-<id>/r<n>` | `Mounts::build` at run start, for `ephemeral` resource mounts only | the FUSE serve answering the container's writes | `Mounts::stop` at run end |

A `read_only` resource mount is served from `resources/<id>` in
place; only `ephemeral` ones get a copy under `overlays/`.

## 4. The Postgres supervisor: `<daemon>/postgres/` (or `<diverge>/postgres/`)

What `diverge-postgres` makes in the directory it is given:

```text
<dir>/
├── config.yaml                     {max_connections: 1024}; absent = default
├── bin/                            the embedded PostgreSQL, extracted once
│   ├── locks/
│   │   ├── install.lock            held while extracting
│   │   └── init.lock               held across password → initdb → stop → start → ready
│   └── <version>/                  postgresql_embedded appends the baked-in version
│       ├── complete                the marker: extraction finished and proven
│       └── bin/{postgres,pg_ctl,initdb,…}
├── password                        the superuser password, read by initdb --pwfile
├── data.ready                      the marker: initdb finished; data/ without it is discarded
└── data/                           the cluster (PGDATA); postmaster.pid while it runs
```

The postmaster listens on `127.0.0.1` on a port chosen at start, with
no Unix socket; the supervisor prints one `Ready {url}` line on stdout
and the daemon takes the URL from it. The daemon does not leash the
supervisor: the next daemon start has the supervisor stop whatever
the last one left, under `init.lock`.

## 5. The whole picture, on one host with everything local

```text
~/.diverge/
├── provider/
│   ├── config.yaml
│   ├── hooks/<name>/hook.yaml
│   ├── run/{auth.json, known_hosts, mounts/<uuid>/}
│   └── podman_data/{ephemeral/<uuid>, …podman's store…}
├── daemon/
│   ├── config.yaml
│   ├── postgres/{config.yaml, password, data.ready, data/, bin/<version>/, bin/locks/}
│   ├── resources/{incoming/<uuid>/, <id>}
│   ├── agents/<id>/{log, index}
│   └── overlays/{agent,tool}-<id>/r<n>
└── (postgres/ — only if diverge-postgres is run by hand)

elsewhere, named by provider/config.yaml:
<store.path>/<identity>/{<name>, .<name>}
<fixed.path>/…

beside the executables:
<provider exe dir>/diverge-container-proxy
<daemon exe dir>/diverge-postgres
```

## 6. What is NOT on disk, for the diagram's sake

- The daemon keeps no provider path, volume path, or container path.
  Everything of a provider's it knows arrives on the wire and lives in
  memory or in Postgres.
- The provider keeps no record: its volumes are read off the stores
  on an identity's first ask and held in memory; its containers are
  podman's, found again by label.
- Neither program writes a log file, a pid file or a socket of its
  own. The supervisor's `postmaster.pid` is Postgres's.
- The container proxy, inside a container, writes nothing to the
  host: its mounts are FUSE over the wire, its database is a socket to
  the daemon.
