# Every document, every field

The five documents in a root. Each is YAML, each refuses a key it does not have, and an absent file means the
defaults shown. Byte sizes are plain integers in BYTES. Read the live value of any of them with
`diverge <program> config get`.

Contents: [daemon.yaml](#daemonyaml) · [provider.yaml](#provideryaml) · [issuer.yaml](#issueryaml) ·
[cli.yaml](#cliyaml) · [viewer.yaml](#vieweryaml)

---

## daemon.yaml

Read once, when the daemon starts.

```yaml
port: 14980
identities:
  http://127.0.0.1:14981: daemon-a
admins:
  - issuer: http://127.0.0.1:14981
    subject: admin
description: build farm, east
postgres:
  kind: local
  max_connections: 1024
idle_seconds: 10
accept_daemons: true
capacity_poll_seconds: 10
```

| Field | Default | Meaning |
|---|---|---|
| `port` | `14980` | TCP port the daemon accepts clients on: a WebSocket upgrade at any path, on every interface. |
| `identities` | none | Map of issuer → subject: the daemon's own identity at each issuer, whose token it presents in every handshake from that issuer (with clients, providers and other daemons). **An issuer absent here is one the daemon trusts no token from**: a client, a provider or a remote daemon presenting a token from an unlisted issuer cannot connect. Each entry must be signed in under this root first (`diverge identities login`). |
| `admins` | none | List of `{issuer, subject}`. Each is admitted as a client whether or not it is an account, and holds every grant. This is what makes a fresh daemon, which has no account, reachable: the first account is made by an admin. With no admin and no account, the daemon admits nobody. Admins do **not** count for a connection arriving from another daemon: that needs an account. |
| `description` | none | One line of plain text, at most 132 characters, that the daemon says of itself to every provider it is listable to. Other daemons see it when they list daemons through that provider. |
| `postgres` | `{kind: local, max_connections: 1024}` | Which database the daemon keeps its records in and serves to its containers. Never changed while the daemon runs. See below. |
| `idle_seconds` | `10` | How many seconds a container (agent or tool) may go unused before the daemon stops it. The clock resets on every use and does not run while the container is active. The record stays; the next use starts the container again. Raise it to keep containers warm. |
| `accept_daemons` | `true` | Whether other daemons may connect to this one through the providers it is connected to. `false`: no daemon reaches it or the tools it serves (no relay into it, no connected tools from it). |
| `capacity_poll_seconds` | `10` | How long the daemon waits, after caching a provider's capacity answer, before asking again. The daemon's own acts that change capacity ask at once regardless. |

`postgres`, tagged by `kind`:

| Kind | Fields | Meaning |
|---|---|---|
| `local` | `max_connections` (default `1024`) | The daemon runs its own Postgres (`diverge-postgres`) under `<root>/daemon/postgres/`, on a loopback port picked at every start. `max_connections` is the postmaster's limit; every container connection is one of them. |
| `remote` | `url` (required), `password` (optional) | The daemon dials a Postgres you run. `url` is `postgres://user@host:port/database` with **no password in it**; `password` is given separately, as written. `diverge run postgres get` reports the URL without the password. |

A key that belongs to the other kind is refused (`max_connections` under `remote`, `url` under `local`).

There is no `providers` key: providers are records in the daemon's database, added with
`diverge run providers add` (skill `diverge-daemons`).

---

## provider.yaml

Read once, when the provider starts. Every relative path in it resolves against the root (the directory that
holds `provider.yaml`), never the working directory.

```yaml
port: 14979
identities:
  http://127.0.0.1:14981: provider
accepted:
  - issuer: http://127.0.0.1:14981
    subject: daemon-a
clients:
  - address: hub.example.com:7000
    issuer: https://auth.example.com
containers:
  enabled: true
  podman:
    registries:
      - host: docker.io
      - host: ghcr.io
        credential:
          username: bolt
          password: ghp_example
    storage_path: provider/podman_data
    image_cache_disk: 34359738368
    container_overlay_disk: 34359738368
    memory: 8589934592
volumes:
  stores:
    - path: provider/volumes
      capacity: 107374182400
  fixed:
    - name: datasets
      path: /srv/datasets
      bytes: 536870912000
      mode: read_only
      authorize_hook: datasets
images:
  path: provider/images
  capacity: 274877906944
messages:
  messages: 60
  seconds: 60
```

### Top level

| Field | Default | Meaning |
|---|---|---|
| `port` | `14979` | TCP port the provider accepts peers on: a WebSocket upgrade at any path, on every interface. |
| `identities` | none | Map of issuer → subject: the provider's own identity at each issuer. An issuer absent here is one the provider trusts no token from. Each must be signed in under this root. |
| `accepted` | none | List of `{issuer, subject}`: the identities the provider serves. A peer whose verified identity is not listed is closed in the handshake, whichever end dialled. **Absent, nobody is served.** A daemon that should use this provider must be here. One connection per identity: two daemons need two subjects. |
| `clients` | none | Peers the provider dials itself (and redials whenever the connection ends), for a provider that cannot be dialled. Each: `address` (`host:port`, dialled as `ws://<address>/`, or a URL with its own scheme) and `issuer` (the issuer the provider presents its token from; `identities` must name a subject there, and the peer's identity must be in `accepted`). |
| `containers` | the defaults below | The container runtime. |
| `volumes` | none | Where volumes may be created and which exist already. Absent: no volume can be created and none exists. |
| `images` | none | The image Store: where images clients build or push on the provider are kept. Absent: a build or push is refused, a listing is empty, and an image comes only from the provider's podman cache, a listed registry, or the caller. |
| `messages` | none | The rate at which the provider relays direct messages between the daemons connected to it. Absent: it relays none, and `providers daemons listen` / `message` are answered unsupported. |

### containers

| Field | Default | Meaning |
|---|---|---|
| `containers.enabled` | `true` | `false`: the provider runs no containers; every agent and tool run is refused as unsupported. Written to the file only when `false`. |
| `containers.podman` | the defaults below | Required when `containers` is written at all; then **every field of `podman` is required**. |
| `podman.registries` | `docker.io`, `ghcr.io`, `quay.io`, anonymous | The registries the provider will pull from when an image's references name them. Each: `host` (what a reference names before its first `/`; no scheme, no path; a port is allowed) and optional `credential: {username, password}` (both required; a personal access token is a password). A reference naming an unlisted registry is ignored. The daemon refuses a run unless at least one of the image's references names a listed registry. |
| `podman.storage_path` | `provider/podman_data` | Where podman's data lives: image cache, container overlays, ephemeral scratch. On Linux it is podman's storage root (`--root`), and its filesystem must support project quotas (XFS `pquota`, or ext4 with project quota) or every run fails. On macOS and Windows it is where the podman machine `diverge-provider` lives. **Changing it gives a fresh, empty store (or machine); the old one is left as it was.** |
| `podman.image_cache_disk` | `34359738368` (32 GiB) | The most the image cache may hold. The provider evicts images no running container uses, least recently run first, except images labelled `diverge.network/keep`. An image larger than this cannot be pulled. |
| `podman.container_overlay_disk` | `34359738368` (32 GiB) | The sum of every running container's `disk` never exceeds it; a run that would is refused. |
| `podman.memory` | `8589934592` (8 GiB) | The sum of every running container's `memory` never exceeds it; a run that would is refused. On macOS the podman machine is given exactly this much; on Windows the machine's memory is WSL's. |

### volumes

| Field | Meaning |
|---|---|
| `volumes.stores` | List, in order of preference, of places new volumes are created: `path` (made at start if absent) and `capacity` (total bytes the volumes in it may reserve; `0` is refused at start). Absent or empty: no client can create a volume. |
| `volumes.fixed` | Volumes that exist before any client asks. Each: `name` (unique; never given to a created volume), `path` (must already be a directory, or the provider refuses to start), `bytes` (declared size, not enforced), `mode` (`persistent`, `ephemeral` or `read_only`; never edited), and optional `authorize_hook`. A fixed volume refuses every edit and is never resized. |
| `fixed[].authorize_hook` | The name of a folder `<root>/provider/hooks/<name>/` holding a `hook.yaml` that names a command per platform. The provider runs it with one JSON line on stdin, `{"identity":{"issuer":…,"subject":…},"volume":"<name>"}`, and lists the volume to that identity only if it exits `0` printing `{"authorized": true}`. Absent: the volume is listed to every identity. |

### images

| Field | Meaning |
|---|---|
| `images.path` | The directory the Store lives under, made at start if absent. |
| `images.capacity` | The most the Store may hold, in bytes, every blob counted once. A push or build that would pass it is refused as insufficient capacity. `0` is refused at start. Nothing owned is evicted. |

### messages

| Field | Meaning |
|---|---|
| `messages.messages` | The most sends one identity makes in one window. At least `1`. |
| `messages.seconds` | The length of one window, in seconds. At least `1`. |

Both are required when the section is present: there is no unlimited provider.

### The provider's state

Under `<root>/provider/`: `hooks/` (one folder per hook), `run/` (scratch), and `podman_data/` unless
`storage_path` says otherwise.

---

## issuer.yaml

Read once, when the issuer starts. Holds no secret: subjects, password hashes, signing keys and refresh tokens
are in `<root>/issuer.sqlite`.

```yaml
port: 14981
issuer: https://auth.example.com
access_token_seconds: 600
refresh_token_days: 30
```

| Field | Default | Meaning |
|---|---|---|
| `port` | `14981` | TCP port the issuer serves plain HTTP on, on every interface. |
| `issuer` | `http://127.0.0.1:<port>` | The issuer identifier: exactly the `iss` of every token, and the URL every endpoint is under. Must be `https`, or `http` on a loopback host (`127.0.0.1`, `localhost`, `[::1]`); no query, no fragment. The issuer itself serves plain HTTP, so an `https` identifier means a proxy in front terminates TLS. |
| `access_token_seconds` | `600` | Lifetime of an access token, and of the ID token issued with it — the token every handshake presents. Programs refresh by themselves; a shorter value only means more refreshes. |
| `refresh_token_days` | `30` | Lifetime of a refresh token, from its issue; each refresh issues a new one. An identity unused for longer is signed out and must log in again. |

The issuer's identifier is the string every other document uses as `issuer`. It must match character for
character: `http://127.0.0.1:14981` and `http://localhost:14981` are two different issuers.

---

## cli.yaml

Read by the CLI on every command. Holds no token.

```yaml
default: a
daemons:
  a:
    address: ws://127.0.0.1:14980/
    default: admin
    accounts:
      admin:
        issuer: http://127.0.0.1:14981
        subject: admin
      reader:
        issuer: http://127.0.0.1:14981
        subject: reader
  b:
    address: ws://127.0.0.1:14982/
    default: admin
    accounts:
      admin: {issuer: "http://127.0.0.1:14981", subject: admin}
```

| Field | Default | Meaning |
|---|---|---|
| `default` | `local` | The name of the daemon a `run` dials when `--daemon` is absent. Also the daemon `diverge daemon stop` asks to stop. |
| `daemons` | one daemon `local` | The daemons the CLI may dial, by a name of your choosing. |
| `daemons.<name>.address` | — (required) | A WebSocket URL, or `host:port` dialled as `ws://<address>/`. |
| `daemons.<name>.default` | none | The account used when `--account` is absent. Absent, every `run` on this daemon must pass `--account`. |
| `daemons.<name>.accounts` | none | The accounts the CLI may dial that daemon as, by a name of your choosing. |
| `daemons.<name>.accounts.<name>.issuer` / `.subject` | — (required) | One of this root's signed-in identities. The CLI presents that identity's token; the daemon must hold an account (or an admin) for exactly that `(issuer, subject)`. |

The built-in default, when there is no file:

```yaml
default: local
daemons:
  local:
    address: ws://127.0.0.1:14980/
```

It has no account, so no `run` works until one is added.

The names here (`a`, `admin`) are the CLI's own labels. They are not the daemon's account names; the daemon
knows an account only by its identity.

---

## viewer.yaml

Read once, when the world viewer (`diverge-world`) starts. Holds no token.

```yaml
daemon:
  address: ws://127.0.0.1:14980/
  issuer: http://127.0.0.1:14981
  subject: admin
  redial: 5
```

| Field | Default | Meaning |
|---|---|---|
| `daemon` | none | The daemon the viewer dials and as whom. **Absent, the viewer does not start.** |
| `daemon.address` | — (required) | A WebSocket URL, or `host:port` (the viewer adds `ws://` and `/`). |
| `daemon.issuer` / `daemon.subject` | — (required) | One of this root's signed-in identities; it must be an account or an admin of that daemon. What the viewer shows is what that account may see. |
| `daemon.redial` | `5` | `never`, or a whole number of seconds (at least `1`) the app waits before dialling again after a connection ends or a dial fails. |

The viewer keeps its own files under `<root>/viewer/`.
