# The filesystem under `.diverge/`: provider, daemon, postgres

Written 2026-10-08; revised 2026-10-13, when each program took its
own document. From the code as it stands in `diverge-provider`,
`diverge-daemon`, `diverge-postgres` and `diverge-cli`.
Every path below is relative to one root, written `<root>`, and every
file and directory is named as the code names it. Nothing here has been observed on a live
host; the three programs have never been run.

## 1. The root, one document per program, and the directories under it

Every Diverge program finds ONE root, the same way, each through its
own `config::root`:

1. `--config <dir>`, the one argument any of them takes;
2. else the environment variable `DIVERGE_CONFIG`;
3. else `.diverge` under the home directory, `HOME` or, on Windows,
   `USERPROFILE`.

The CLI alone takes one more turn before the home directory:
`.diverge` in the working directory, where that directory exists. The
path is made absolute against the working directory without following
links, and the root is made if absent. Each program reads ITS OWN
document in the root — `provider.yaml`, `daemon.yaml`, `cli.yaml` —
and keeps its state under a directory of its own:

```text
~/.diverge/                         <root>
├── provider.yaml                   the provider's document
├── daemon.yaml                     the daemon's document
├── cli.yaml                        the CLI's document
├── provider/                       the provider's directory
└── daemon/                         the daemon's directory
    └── postgres/                   the supervisor's, by the daemon or by hand
```

Three rules of a document, each program's `config` stating them for
its own:

- **An absent file is the defaults; an absent key is its default.** A
  key the document does not have, at any level, is refused, so a
  misspelled setting is an error and not a silence. No program reads
  another's document.
- **A relative path resolves against the root**, the directory that
  holds the file, never the working directory. The provider's `load`
  resolves every path as it reads — `containers.podman.storage_path`,
  each store's and each fixed volume's `path`, `images.path` — so it
  holds every path absolute. The default `storage_path` is
  `provider/podman_data`.
- **`config` reads; the program checks its own disk.** What a document
  names on disk — a store to make, a fixed volume that must exist, a
  capacity that must not be `0` — is checked by the program at its
  start, with its own errors.

Two things to hold onto before the trees:

- **The provider and the daemon share the root and nothing else.**
  Neither reads the other's document or the other's directory; they
  talk to each other over the wire. A volume's bytes are the
  provider's; an agent's log is the daemon's.
- **The daemon's Postgres lives inside the daemon's directory, and is
  told its settings by the daemon.** The daemon runs
  `diverge-postgres --root <root> --max-connections <n>`, from the
  local kind of its own `postgres`; the supervisor reads no document
  and keeps its cluster at `<root>/daemon/postgres/`, so one started
  by hand with the same two arguments and one the daemon starts are
  the same cluster. Under the remote kind nothing is started.

Beside the directories there are two binaries each program expects
NEXT TO ITS OWN EXECUTABLE, not under `<root>`:

| Program | Looks for | Why |
|---|---|---|
| `diverge-provider` | `diverge-container-proxy` | bind-mounted into every container it runs; missing is a start error |
| `diverge-daemon` | `diverge-postgres` (`.exe` on Windows) | spawned when `postgres` is `{kind: local}` |

## 2. The provider: `<root>/provider/`

```text
<root>/provider/                    made at start
├── hooks/                          made at start, even when empty
│   └── <name>/                     one folder per hook, named in provider.yaml
│       ├── hook.yaml               {windows|macos|linux: [argv…]}, no shell
│       └── …                       the program argv[0] names, resolved in this folder
├── run/                            made at start; the provider's own scratch
│   ├── auth.json                   podman's auth file, 0600: registries' credentials
│   ├── mounts/                     where stored volumes are loop-mounted
│   │   └── <uuid>/                 one mount point per attached volume image
│   └── known_hosts                 macOS/Windows only: the podman machine's ssh key
└── podman_data/                    provider.containers.podman.storage_path, DEFAULT location
    ├── ephemeral/                  scratch of every ephemeral volume serve, swept at start
    │   └── <uuid>                  a sparse file; unlinked at once on Unix
    └── …                           podman's own store (Linux: --root); the machine (macOS/Windows)
```

And, wherever the block points them — relative to the root when
relative, so `stores/a` is `<root>/stores/a`:

```text
<store.path>/                       one per volumes.stores[]; made at start
└── <identity>/                     made on an identity's first create
    ├── <name>                      the volume: a sparse ext4 image file
    └── .<name>                     its mode: {"mode":"persistent"|"ephemeral"|"read_only"}

<fixed.path>/                       one per volumes.fixed[]; must exist already
└── …                               the directory IS the volume; writes land as
                                    .<name>.<uuid> temporaries renamed into place
```

### 2.1 `provider.yaml`

The file is optional, every section of it is optional, and an
unknown key is refused. Every relative path resolves against the
root.

```yaml
port: 14979                       # OUTSIDE_PORT; what peers dial
auth:                             # who may dial in; absent = nobody
  unbrokered:
    - key: {key: "…", identity: "…", address: 203.0.113.7}   # address optional
    - hook: {authorize_hook: "<name>"}                       # provider/hooks/<name>/
clients:                          # whom the provider dials; absent = nobody
  unbrokered:
    - {address: "host:port", key: "…", identity: "…"}
containers:                       # always present; absent = defaults
  podman:
    registries:                   # default: docker.io, ghcr.io, quay.io, anonymous
      - {host: docker.io, credential: {username: "…", password: "…"}}
    storage_path: provider/podman_data   # relative → <root>/provider/podman_data
    image_cache_disk: 34359738368 # 32 GiB
    container_overlay_disk: 34359738368
    memory: 8589934592            # 8 GiB
volumes:                          # absent = no volume can exist
  stores:                         # made at the provider's start; capacity 0 refused
    - {path: /srv/volumes, capacity: 1099511627776}
  fixed:                          # must be existing directories; names unique
    - {name: datasets, path: /srv/datasets, bytes: 0, mode: read_only, authorize_hook: "<name>"}
images:                           # absent = no client image store
  path: provider/images           # relative → <root>/provider/images; root/ and meta/ inside
  capacity: 274877906944          # 256 GiB over every client image; 0 refused
```

What the provider checks at its start, in `serve::Provider::start`:
`<root>/provider/`, `hooks/`, `run/`, `storage_path` and every store
made if absent; a store whose capacity is `0`, a fixed path that is
not an existing directory, and a fixed name that is not a volume name
or repeats, each refused with the provider's own error.

### 2.2 Who writes what, when

| Path | Made | Written by | Removed |
|---|---|---|---|
| `hooks/` | `Provider::start`, every start | the operator | never by the provider |
| `run/mounts/` | deployer start | `Volume::attach` makes `<uuid>/` and loop-mounts an image on it | swept at start (an earlier life's mounts), unmounted when the last container detaches |
| `run/auth.json` | deployer start | rewritten every start from `registries[].credential` | — |
| `run/known_hosts` | tunnel open (macOS/Windows) | emptied every start | — |
| `<storage_path>/` | `Provider::start` | podman, as `--root` on Linux; the machine's disk elsewhere | a changed path leaves the old store as it was |
| `<storage_path>/ephemeral/` | `Scratch::sweep` at start | one sparse file per ephemeral serve | every entry removed at start |
| `<store>/<identity>/` | first `volumes::create` by that identity | `<name>` image + `.<name>` mode | `volumes::delete` removes both |

The provider also labels every container it runs
`diverge.provider=<root>/provider`, which is how it finds and sweeps
the containers of an earlier life. The podman machine on macOS and
Windows is made under `storage_path` and is handed every store path,
every fixed path, the provider's directory and the proxy's directory
as shares.

## 3. The daemon: `<root>/daemon/`

```text
<root>/daemon/                      made at start
├── postgres/                       ONLY when daemon.postgres is {kind: local}; see §4
└── agents/                         every agent's log, by record id
    └── <id>/
        ├── log                     one JSON ItemWrapper per line, appended
        └── index                   16 bytes per item: offset u64 BE, length u64 BE
```

`agents/` is made at every start. The records themselves — accounts,
roles, providers, agents, tools, templates, tags, grants — are
not files: they are rows in Postgres, local or remote, under the
daemon's own schema. Volumes have no files here at all; the daemon
mirrors each provider's listing in memory and serves FUSE mounts of
them through the provider. Nothing else of a container's content is
the daemon's: what a container needs is a provider's volume.

### 3.1 `daemon.yaml`

```yaml
port: 14980                       # DEFAULT_PORT, one above a provider's
postgres: {kind: local, max_connections: 1024}   # or {kind: remote, url: "postgres://user:pw@host:port/db"}
idle_seconds: 10                  # a container unused this long is stopped
accept_daemons: true              # other daemons reached through the providers
```

Four keys, all optional, unknown keys refused, including a key the
`postgres` kind does not have. `postgres` is read once; another
database is another start. With `remote` nothing of §4 exists and the
daemon dials the URL. `max_connections` is handed to the supervisor
as an argument.

### 3.2 Who writes what, when

| Path | Made | Written by | Removed |
|---|---|---|---|
| `agents/<id>/{log,index}` | first append (the `Active` item of the first run) | every chunk, every loop word, every start error, under the agent's live lock | `agents delete` |

## 4. The Postgres supervisor: `<root>/daemon/postgres/`

What `diverge-postgres` makes under the root it is given, with the
settings the daemon hands it:

```text
<root>/daemon/postgres/             made at start
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
no Unix socket, accepting the `--max-connections` it was given at
once; the supervisor prints one `Ready {url}` line on stdout
and the daemon takes the URL from it. The daemon does not leash the
supervisor: the next daemon start has the supervisor stop whatever
the last one left, under `init.lock`.

## 5. The whole picture, on one host with everything local

```text
~/.diverge/
├── provider.yaml
├── daemon.yaml
├── cli.yaml
├── provider/
│   ├── hooks/<name>/hook.yaml
│   ├── run/{auth.json, known_hosts, mounts/<uuid>/}
│   └── podman_data/{ephemeral/<uuid>, …podman's store…}
└── daemon/
    ├── postgres/{password, data.ready, data/, bin/<version>/, bin/locks/}
    └── agents/<id>/{log, index}

elsewhere, named by provider.yaml:
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
