# Diverge setup: a live diverge-world with two daemons on one machine

This guide is for a coding agent (Claude Code) on a fresh machine that has this repository cloned. By the end you
will have:

- one **provider** and **two daemons** (A and B) running on one host
- the agent images built into the provider's podman store
- an agent running on daemon A, plus a tool on daemon B that daemon A's agent reaches across the daemons
- **diverge-world** running in dev mode against daemon A, and optionally a second window against daemon B

Every command, flag and request body here was checked against the source, the installed CLI's `--help`, and the
CLI's `request-schema` output. Steps 1 to 8 and the CLI half of step 10, daemon to daemon, were also run live on
macOS (Apple Silicon, podman 6.1.2). Not run there at this revision: diverge-world, which is steps 9 and 11 and
what step 10 shows in it; and nothing was run on Linux. Section 9 lists everything that is uncertain. When this
guide and the code disagree, the code wins.

Read sections 1 and 2 before running anything.

---

## 1. Concepts (short)

| Term | What it is |
|---|---|
| **Root** (config root) | One directory holding a set of Diverge programs' documents and state: `provider.yaml`, `daemon.yaml`, `cli.yaml`, `viewer.yaml`, `identities/`, `daemon/`, `provider/`, `bin/`, and the `*.lock`/`*.log` files. Default `~/.diverge`. Each program runs at most once per root, enforced by its `<program>.lock`. |
| **Issuer** | An identity's issuer is any OpenID Connect provider that offers the device authorization flow. Diverge's own is `https://identity.diverge.network`, hosted, with one client id, `diverge`; its subjects are `<issuer>.<id>.<name>` (for example `github.12345.daemon-a`), where `<issuer>.<id>` is a GitHub or Google account signed in with `diverge identity.diverge.network accounts login`. Every program proves who it is with an ID token from an issuer. |
| **Identity** | One `(issuer, subject)` pair, signed in once with `diverge identities login` and stored at `<root>/identities/<pct-encoded issuer>/<subject>.json`. Programs on a root use the identities stored under **that root**. Handshakes are mutual: both ends present an ID token. |
| **Daemon** (`diverge-daemon`) | The hub clients talk to, over a WebSocket on its `port` (default 14980). It keeps all records in its own Postgres: accounts, roles, providers, remote daemons, templates, agents, tools, events, logs, vault. `diverge-postgres` is a supervisor with Postgres embedded at build time; the daemon starts it on a free loopback port, with its data under `<root>/daemon/postgres/`. |
| **Provider** (`diverge-provider`) | Runs containers with podman for the daemons in its `incoming` list, and also serves volumes and an image store. Default port 14979. On Windows and macOS it drives a podman machine named **`diverge-provider`** through the rootful connection **`diverge-provider-root`**. On Linux it runs podman directly and must run as root. |
| **Container proxy** (`diverge-container-proxy`) | A static Linux musl binary. The provider bind-mounts it into every container (`/.diverge/diverge-container-proxy`) and runs it there. It bridges the provider and the program inside the container: MCP, Postgres, secrets, the message queue. It is **never part of an image**, but the inside-container contract it speaks has to match the image's program. |
| **Agent program / image** | An OCI image whose program speaks the inside-container contract. The five agent loops are `diverge-agents/{python,openrouter,codex,hermes,cc}`. Each image carries `/DIVERGE.md`: JSON frontmatter `{name, schema[, frontend]}` (the schema of the `arguments` the image accepts), followed by prose. |
| **Template** | `agents templates create` / `standalone-tools templates create`: an image (`{digest, references}`) plus `memory` and `disk` (both in bytes), `arguments`, and the names of the `secrets` it needs. Its id is a hash, so creating the same template twice gives back the same id. |
| **Agent** | `agents create {template, name}`: one container made from a template. It runs when it is messaged (`agents message`) and is stopped after `idle_seconds` of disuse. Its record and conversation persist. |
| **Tools** | MCP-server containers. **Standalone tools** are created by hand and attached to agents. **Dependency tools** are declared by an agent image's `dependencies` argument and deployed by the daemon. **Connected standalone tools** are another daemon's standalone tools, held under a name on this daemon. |
| **Accounts / roles / grants** | The daemon admits a client by identity. The `admins` listed in `daemon.yaml` hold every grant. Anyone else must be an **account** (`accounts create`) holding **roles** (`roles create`), and each role is a list of **grants** such as `{"agents":["create"]}` or `{"agents":{"actions":["list","get"],"within":"any"}}`. |
| **Vault / secrets** | `vault insert {key, value}` stores a value. A template names the secrets it needs, and `agents create` binds each name to a vault key: `"secrets": {"OPENROUTER_API_KEY": "my-or-key"}`. Secrets are never arguments or environment variables. |
| **Events** | The daemon's graph of who sent what to whom. diverge-world draws each one as a line. |
| **Remote daemons and relay** | Daemon A records daemon B with `daemons add`, which returns a `DaemonId`. A reaches B only **through a provider both are connected to**, where B announces itself (an `announcement` on B's record of that provider) and accepts other daemons (`accept_daemons`, default `true`). Any request may carry `"relay": [<ids>]`: daemon A takes the first ID off and forwards the request to B, which serves it as the account A presents there. diverge-world uses this to open another daemon's board in a tab. |
| **diverge-world** | A Tauri + Phaser app (`diverge-world/`). Its Rust side holds one connection to the daemon named by `<root>/viewer.yaml`, and the page speaks the daemon protocol through `@diverge/sdk` (`diverge-sdk-js`). |

---

## 2. Hard rules for the agent doing this

0. **Create a new branch before doing any work.** Before the first command that changes anything — an install, a
   config edit, a commit — make and switch to a branch of your own, so nothing lands on the branch you were handed:
   ```bash
   git switch -c setup-$(date +%Y%m%d)
   git branch --show-current   # expected: setup-<today's date>
   ```
   Keep every change, commit and push on that branch.
1. **Never pipe `diverge … start`.** The started server inherits the pipe, and your shell hangs. Always redirect to a
   file, for example `diverge provider start >"$WORK/provider-start.out" 2>&1; cat "$WORK/provider-start.out"`.
   The server's own log is `<root>/<program>.log`.
2. **Global CLI flags go before the subcommand.**
   - `--config <dir>` must come first: `diverge --config "$B" daemon start`. It is not a global flag.
   - `--daemon`, `--account`, `--jq` and `--timeout` belong to `run`: `diverge run --daemon b agents list`.
3. **Root resolution.**
   - Servers and diverge-world: `--config`, else `DIVERGE_CONFIG`, else `~/.diverge`.
   - The CLI checks one more place between those last two: a `./.diverge` directory in the current working
     directory, if one exists. Don't run the CLI from a directory that contains a stray `.diverge/`.
4. **The CLI starts servers from its own directory.** `diverge daemon|provider start` launches the binary in
   the same directory as the running `diverge` executable and passes it `--config <root>`. Root B therefore needs no
   `bin/`: use root A's `diverge` with `--config "$B"`.
5. **`diverge daemon stop` reads `cli.yaml`, not `daemon.yaml`.** It takes the port from the address of the
   **default daemon in that root's `cli.yaml`**. Each root therefore needs a `cli.yaml` whose default daemon is that
   root's own daemon.
6. **Heavy builds are slow.** Run them one at a time, in the background, with output going to a file.
   - Every image compiles Rust inside the build and takes about 10 minutes.
   - The first `install-diverge.sh` and the first `tauri dev` each take many minutes.
   - All cargo builds share one `target/` lock.

---

## 3. Prerequisites

| Tool | Why | Notes |
|---|---|---|
| Rust (stable, MSVC toolchain on Windows), `rustup` | every crate | Edition 2024 crates, so a recent stable is needed (1.94 is known to work). |
| `rustup target add x86_64-unknown-linux-musl` (or `aarch64-…`) | the container proxy | Built for `<host arch>-unknown-linux-musl`. |
| `cargo install cargo-zigbuild` + `pip install ziglang` | the container proxy | `diverge-container-proxy/install.sh` refuses to run without `cargo-zigbuild`, and `ring` needs zig as the C toolchain for musl. A `zig` already on PATH also works. |
| Python 3 | `pip install ziglang`, and the helper snippets below | On Windows the command is `python`; elsewhere `python3`. |
| Node ≥ 22.18 and pnpm 10.x | diverge-world, diverge-sdk-js | The root `package.json` pins `pnpm@10.25.0`; `diverge-sdk-js` requires node `>=22.18.0`. |
| Tauri v2 system prerequisites | diverge-world | Windows: MSVC C++ Build Tools and WebView2. Linux: webkit2gtk and the rest of Tauri's list. The Tauri CLI comes from `diverge-world`'s devDependencies (`pnpm tauri`). |
| **podman** (6.x) | the provider | **Windows:** install the upstream MSI (6.1.4 is known to work). It installs to `%LOCALAPPDATA%\Programs\Podman`, which is **not on Git Bash's PATH**, so add it (step 0). Podman machines run on **WSL2**, so enable WSL2 first (`wsl --install --no-distribution` in an elevated shell, then reboot). **macOS:** podman with its machine support. **Linux:** rootful podman. See §8. |
| `ssh` client on PATH | the provider | The provider opens an `ssh -R` tunnel into the podman machine on Windows and macOS. Windows 10+ ships OpenSSH. |
| Network access during builds | `diverge-postgres`, images | `diverge-postgres` uses `postgresql_embedded` with the `bundled` feature, which fetches the Postgres archive at **build** time. Image builds pull pinned base images from docker.io. |

On Windows, run everything below in **Git Bash**. The install scripts are bash.

---

## 4. Layout used by this guide

| Thing | Value |
|---|---|
| Root A (provider, daemon A, CLI, world A) | `A="$HOME/.diverge"` |
| Root B (daemon B, world B) | `B="$HOME/.diverge-b"` |
| Issuer URL | `ISS="https://identity.diverge.network"` |
| Subjects at the issuer | `github.12345.admin` (you: CLI and both worlds, administrator of both daemons), `github.12345.provider`, `github.12345.daemon-a`, `github.12345.daemon-b`. Each is `<provider>.<id>.<name>`; `12345` stands for your own numeric id at GitHub. |
| Scratch dir for request bodies | `WORK="$HOME/diverge-setup"` |

| Port | Who | Configured in |
|---|---|---|
| 14979 | provider (all interfaces) | `provider.yaml` `port` |
| 14980 | daemon A (all interfaces) | `$A/daemon.yaml` `port`, `$A/cli.yaml` |
| **14982** | daemon B | `$B/daemon.yaml` `port`, `$B/cli.yaml`. The value is this guide's choice; any free port works. |
| random loopback | each daemon's Postgres | chosen at every start, so the two daemons never collide |
| 1440 / 1441 | diverge-world Vite dev server / HMR | `diverge-world/vite.config.ts` (`strictPort: true`) |
| 1430 | diverge-viewer (the other app, not used here) | |

| Path | What |
|---|---|
| `<root>/bin/` | installed binaries: `diverge`, `diverge-daemon`, `diverge-provider`, `diverge-postgres` (each `.exe` on Windows), `diverge-container-proxy` (no extension) |
| `<root>/identities/` | signed-in identities |
| `<root>/daemon/postgres/` | the daemon's Postgres (`bin/`, `data/`, `password`) |
| `<root>/provider/podman_data/` | `containers.podman.storage_path` (default). On Windows and macOS **this directory is the podman machine**: its disk and SSH key live here. |
| `<root>/{provider,daemon}.log` | server output |

The two daemons share **one issuer and one provider**. The code allows this: the provider's `incoming` list holds
both daemon identities, and it allows one connection per identity, so the two daemons need different subjects.
Sharing an issuer is this guide's choice, not a requirement: a handshake's two tokens need not be from the same
issuer, and no program keeps a list of trusted issuers.
Sharing a provider is also *required* for daemon-to-daemon traffic, because A reaches B only through a provider
both are connected to. On Windows and macOS a second provider would not work anyway, since the machine name
`diverge-provider` is fixed.

---

## 5. Steps

Run from the repository root in Git Bash (Windows) or bash.

### Step 0: shell setup (repeat in every new shell)

```bash
REPO="$(pwd)"                       # the repository root
A="$HOME/.diverge"; B="$HOME/.diverge-b"
ISS="https://identity.diverge.network"
WORK="$HOME/diverge-setup"; mkdir -p "$WORK"
export PATH="$A/bin:$PATH"
# Windows only: podman's per-user install is not on Git Bash's PATH.
command -v cygpath >/dev/null && export PATH="$(cygpath "$LOCALAPPDATA")/Programs/Podman:$PATH"
PY=python            # python3 on Linux/macOS
```

Check: `podman --version` prints 6.x, and `cargo zigbuild --help` prints usage.

Do **not** `export MSYS_NO_PATHCONV=1` globally. Git Bash's conversion of `/c/...` arguments into `C:/...` is what
makes `--config "$B"` work for the Windows binaries. Set it only for the single command that needs it (§8).

### Step 1: build and install every program into root A

```bash
./install-diverge.sh "$A" --release > "$WORK/install.log" 2>&1     # long: run in the background
tail -n 3 "$WORK/install.log"
```

The script runs the five crates one after another: postgres, container-proxy, daemon, provider, cli.
`--debug` also works and builds faster.

Check: the log ends with `installed every Diverge program under …/bin (release)`, and `ls "$A/bin"` lists the five
binaries.

To rebuild one program later, run `<crate>/install.sh "$A" --release`. Stop that program first: Windows refuses to
replace a running executable.

### Step 3: sign identities in, per root

Every program uses identities from **its own root**:

- root A needs `github.12345.admin` (for the CLI and world A), `github.12345.provider`, and `github.12345.daemon-a`
- root B needs `github.12345.daemon-b` and `github.12345.admin` (for world B)

> **This step cannot be followed at this revision.** Identities are signed in at
> `https://identity.diverge.network`, and that service is not yet live. The commands below are what the step
> will be once it is; until then the identities this guide needs cannot be made, and this step and everything
> after it cannot be followed.

First sign your GitHub (or Google) account in, once per root. This is the one part that needs a person:

```bash
diverge --config "$A" identity.diverge.network accounts login github
# {"user_code":"ABCD-EFGH","verification_uri":"https://github.com/login/device",…,"browser_opened":true}
# Enter the code on the page it opened, then:
diverge --config "$A" identity.diverge.network accounts list
# {"account":"github.12345","issuer":"github","id":"12345"}       12345 is your own GitHub id; use it below
```

Then sign each identity in. At this issuer the command approves the code itself and waits:

```bash
diverge --config "$A" identities login --issuer "$ISS" --subject github.12345.admin --client-id diverge
# {"issuer":"https://identity.diverge.network","subject":"github.12345.admin"}
diverge --config "$A" identities list   # every identity signed in under root A
```

Root B needs its own account sign-in (`--config "$B"`), since accounts are per root.

### Step 4: write the documents

All documents refuse unknown keys. A typo is an error at start, not ignored.

```bash
cat > "$A/provider.yaml" <<EOF
port: 14979
identity:
  issuer: $ISS
  subject: github.12345.provider
incoming:
  - issuer: $ISS
    subject: github.12345.daemon-a
  - issuer: $ISS
    subject: github.12345.daemon-b
EOF
```

`containers` is left out of `provider.yaml`, so its defaults apply:

- `podman.storage_path: provider/podman_data`
- `registries`: docker.io, ghcr.io, quay.io
- `image_cache_disk` and `container_overlay_disk`: 32 GiB each
- `memory`: 8 GiB (macOS only)

Optional sections:

- `volumes: {stores: [{path, capacity}], fixed: [...]}` gives the provider volumes.
- `messages: {messages: <n>, seconds: <n>}` enables daemon-to-daemon direct messages
  (`providers daemons listen|message`). Relay and connected tools do not need it.

```bash
cat > "$A/daemon.yaml" <<EOF
port: 14980
identity:
  issuer: $ISS
  subject: github.12345.daemon-a
admins:
  - issuer: $ISS
    subject: github.12345.admin
idle_seconds: 120
EOF
mkdir -p "$B"
cat > "$B/daemon.yaml" <<EOF
port: 14982
identity:
  issuer: $ISS
  subject: github.12345.daemon-b
admins:
  - issuer: $ISS
    subject: github.12345.admin
idle_seconds: 120
EOF
```

Notes on `daemon.yaml`:

- `idle_seconds` defaults to **10**: a container is stopped 10 s after its last use. A larger value keeps agents
  and tools up while you look at them in the world.
- `daemon.yaml` holds `port`, `identity`, `admins`, `postgres` and `idle_seconds`, and nothing else. It no
  longer has `description`, `accept_daemons` or `capacity_poll_seconds`: the document is strict, so a
  `daemon.yaml` that still holds one of them is refused at start. What the daemon announces itself with, whether
  it accepts other daemons and whether it hears messages are now set **per provider record** (below); the
  capacity poll is a fixed ten seconds.
- `postgres` defaults to `{kind: local, max_connections: 1024}`.

```bash
cat > "$A/cli.yaml" <<EOF
default: a
daemons:
  a:
    address: ws://127.0.0.1:14980/
    default: admin
    accounts:
      admin: {issuer: "$ISS", subject: github.12345.admin}
  b:
    address: ws://127.0.0.1:14982/
    default: admin
    accounts:
      admin: {issuer: "$ISS", subject: github.12345.admin}
EOF
# Root B's cli.yaml exists so that `diverge --config "$B" daemon stop` finds daemon B's port.
cat > "$B/cli.yaml" <<EOF
default: b
daemons:
  b:
    address: ws://127.0.0.1:14982/
EOF
cat > "$A/viewer.yaml" <<EOF
daemon:
  address: ws://127.0.0.1:14980/
  issuer: $ISS
  subject: github.12345.admin
EOF
cat > "$B/viewer.yaml" <<EOF
daemon:
  address: ws://127.0.0.1:14982/
  issuer: $ISS
  subject: github.12345.admin
EOF
```

`viewer.yaml` also accepts `daemon.redial`: either `never` or a number of seconds (default 5).

Check: each of these prints its document back as JSON, which means each document parses.

```bash
diverge provider config get
diverge daemon config get
diverge --config "$B" daemon config get
diverge cli config get
diverge viewer config get
```

### Step 5: start the provider

The provider's first start on Windows or macOS **creates the podman machine** (`podman machine init --rootful
diverge-provider`, with its data under `$A/provider/podman_data`) and starts it. It needs `podman` and `ssh` on the
PATH of the shell that runs `start`, and the first start downloads the machine image, so it can take minutes.

```bash
diverge provider start > "$WORK/provider-start.out" 2>&1; cat "$WORK/provider-start.out"
# expected: started provider, process <pid>
```

Check:

- `podman machine list` shows `diverge-provider` as running.
- `$A/provider.log` has no error. On success the provider prints nothing; a failed start is reported there.

### Step 6: start both daemons and connect each to the provider

```bash
diverge daemon start > "$WORK/daemon-a-start.out" 2>&1; cat "$WORK/daemon-a-start.out"
diverge --config "$B" daemon start > "$WORK/daemon-b-start.out" 2>&1; cat "$WORK/daemon-b-start.out"
diverge run info                      # daemon A answers with its version
diverge run --daemon b info           # daemon B
```

The first daemon start initialises its Postgres cluster under `<root>/daemon/postgres/`.

Providers are records in the daemon's database; there is no `providers` key in `daemon.yaml`. Add the shared
provider to **both** daemons, `outgoing` (the daemon dials it), each with its own `announcement`. The
announcement is what lets each daemon find the other on the provider.

```bash
PA='{"identity":{"issuer":"'"$ISS"'","subject":"github.12345.provider"},"direction":"outgoing","address":"127.0.0.1:14979","announcement":"daemon A","messageable":true}'
PB='{"identity":{"issuer":"'"$ISS"'","subject":"github.12345.provider"},"direction":"outgoing","address":"127.0.0.1:14979","announcement":"daemon B","messageable":true}'
diverge run providers add "$PA"
diverge run --daemon b providers add "$PB"
diverge run providers list            # items, then "listed"; the record's "status" should be "connected"
diverge run --daemon b providers list
```

A record that stays `disconnected` usually has one of two causes:

- The daemon identity is missing from `provider.yaml` `incoming`.
- The provider is not running.

The daemon redials every 5 s, so fix the cause and list again.

What the three per-provider members of a record do, each independent of the other two:

| Member | Default | Meaning |
|---|---|---|
| `announcement` | absent | One line, 1 to 132 characters. Present: the daemon announces itself on that provider with it, and other daemons see it in `providers daemons list`. Absent: it is not listed there. |
| `accept_daemons` | `true` | The daemon accepts connections from other daemons through that provider (step 10 needs it on B). |
| `messageable` | `false` | The daemon listens for direct messages on that provider. |

A daemon sends direct messages whatever its own three are. A receiver is found through the listing when sent
to with `diverge run providers daemons message`, so a receiver that is messageable but has no `announcement`
answers `not_found` there.

Change them on a running daemon with `providers edit`; nothing is restarted and the provider's connection
stays up, so no container stops:

```bash
ID='"identity":{"issuer":"'"$ISS"'","subject":"github.12345.provider"}'
diverge run --daemon b providers edit "{$ID,\"announcement\":\"daemon B, staging\",\"messageable\":false}"
diverge run --daemon b providers edit "{$ID,\"announcement\":null}"       # stop announcing; absent would leave it
diverge run --daemon b providers edit "{$ID,\"announcement\":\"daemon B\",\"messageable\":true}"
```

`edit` takes `announcement` and `ips` (absent = leave, `null` = clear, a value = set), `accept_daemons`,
`messageable` and `description`. It does not take `direction` or `address` (a body naming them is refused):
the identity, direction and address are fixed for a record's life. The only edit that closes a connection is
one whose new `ips` no longer admit an incoming provider's source address.

### Step 7: build the images into the provider's store

A deploy finds an image **by digest** (`diverge-provider/src/host/container_deployer/source.rs`):

1. the caller's own Store
2. **then the provider's local podman cache**: `podman image ls --filter digest=<d>`, counting only images that
   have a name *not* starting with `127.0.0.1:`
3. then the listed registries
4. then the caller

So build each image **inside the provider's podman** under a normal name, and pass its digest. On Windows and
macOS that means the provider's machine via `--connection diverge-provider-root`, with `XDG_DATA_HOME` and
`XDG_CONFIG_HOME` pointed at the provider's `storage_path` (`diverge-provider/src/host/tools/podman/command.rs`).
The provider must be running, because that keeps the machine up.

```bash
# Windows / macOS: the same podman the provider drives.
PD="$A/provider/podman_data"; command -v cygpath >/dev/null && PD="$(cygpath -w "$PD")"
pp() { XDG_DATA_HOME="$PD" XDG_CONFIG_HOME="$PD" podman --connection diverge-provider-root "$@"; }
# Linux (provider runs as root): pp() { sudo podman --root "<provider root>/provider/podman_data" "$@"; }

cd "$REPO"
build() {  # build <dir with Containerfile> <short name>
  pp build --label diverge.network/keep=true -t "localhost/diverge-local/$2:dev" -f "$1/Containerfile" . \
    > "$WORK/build-$2.log" 2>&1
}
build diverge-agents/python python      # ~10 min; run in the background
build diverge-examples/tools/memory memory                  # used for the daemon-to-daemon test
pp image ls --no-trunc --format '{{.Repository}}:{{.Tag}} {{.Digest}}'
DIGEST_PY="$(pp image inspect --format '{{.Digest}}' localhost/diverge-local/python:dev)"
DIGEST_MEM="$(pp image inspect --format '{{.Digest}}' localhost/diverge-local/memory:dev)"
echo "$DIGEST_PY $DIGEST_MEM"                  # sha256:… sha256:…
```

How the build works:

- **The build context is the workspace root (`.`)** for every image. Each Containerfile copies `diverge-sdk-rs/`,
  its own crate and `Cargo.lock`. The root `.dockerignore` keeps `target/` and `node_modules` out of the context.
- `--label diverge.network/keep=true` protects the image from the provider's cache eviction. Images the provider
  did not record itself are evicted first once the cache passes `image_cache_disk`.
- Never tag an image under `127.0.0.1:`. The lookup skips those names on purpose.

The images:

| Image dir | `name` | Kind | Required `arguments` | Secrets (vault-bound at create) |
|---|---|---|---|---|
| `diverge-agents/python` | python | agent, no model | `python` (source, run every turn); optional `requirements`, `dependencies` | none |
| `diverge-agents/openrouter` | openrouter | agent | `model`; optional `system_prompt`, sampling fields, `dependencies` | `OPENROUTER_API_KEY` |
| `diverge-agents/codex` | codex | agent | `model`, `auth` | exactly one, by `auth.method`: `OPENAI_API_KEY`, `CODEX_ACCESS_TOKEN`, `CODEX_AUTH` (rotating) or `OPENAI_IDENTITY_TOKEN` |
| `diverge-agents/hermes` | hermes | agent | `provider`, `model` | depends on provider and toolsets (see its DIVERGE.md) |
| `diverge-agents/cc` | cc | agent (Claude Code) | `model` | none declared. It installs Claude Code from npm at container start and "logs in by itself", so the container needs network. How credentials reach it is not established here. |
| `diverge-examples/tools/courier` | courier | dependency tool, service, `global` | `{}` | none |
| `diverge-examples/tools/taskboard` | taskboard | dependency tool, service, `template` | `{}` | none |
| `diverge-examples/tools/memory` | memory | dependency tool, transient | `{}` | none |
| `diverge-examples/tools/alarm` | alarm | dependency tool, service, `global` | `{}` | none |
| `diverge-examples/tools/chat` | chat | standalone tool, service, has a frontend | `{}` | none |
| `diverge-examples/agents/human` | human | agent, no model, has a frontend | `{}` | none |

Each image's full argument schema is the frontmatter of its `DIVERGE.md`. Running the image's program with
`--schema` prints the same schema, and `diverge run images info` returns the DIVERGE.md of a deployed image.

Dependency tools go into an agent's `arguments.dependencies` as
`{"image":{"digest","references"},"memory","disk","arguments":{},"mode":"transient"}`. A service adds `"mode":"service","scope":"global"|"template"|"instance"`.

**Rebuild images whenever the inside-container contract changes** (`diverge-sdk-rs/src/container_proxy/inside`,
the proxy, or a loop crate). An image built against an older contract fails against the current proxy. Build every
image you use from the current source.

### Step 8: a template, an agent, a message, a log (daemon A)

This agent script (python image, no model):

- echoes the text it is sent
- answers `tools` with the names of the tools it can see
- answers `call <tool> <json>` by calling that tool and then reporting the tool's answer

It was run against the image's real `harness.py` on a host Python.

```bash
cat > "$WORK/agent.py" <<'EOF'
import uuid

def text_of(blocks):
    return " ".join(b.get("text", "") for b in blocks
                    if isinstance(b, dict) and b.get("type") == "text")

last = input[-1] if input else None
if isinstance(last, dict) and last.get("type") == "tool_response":
    out = [{"type": "assistant_text_content",
            "text": "tool said: " + text_of(last.get("content", []))}]
else:
    said = text_of(last) if isinstance(last, list) else ""
    if said == "tools":
        out = [{"type": "assistant_text_content",
                "text": "tools: " + ", ".join(t["name"] for t in tools)}]
    elif said.startswith("call "):
        parts = said.split(" ", 2)
        out = [{"type": "assistant_tool_call", "id": "call_" + uuid.uuid4().hex,
                "name": parts[1], "arguments": parts[2] if len(parts) > 2 else "{}"}]
    else:
        out = [{"type": "assistant_text_content", "text": "echo: " + said}]
out
EOF
```

How the script contract works (`diverge-agents/python/HARNESS.md`):

- Each turn runs `python3 harness.py agent.py`. The globals `input`, `tools` and `resources` are supplied by the
  harness.
- The last bare expression, or else the printed JSON, must be a list of assistant chunks or notifications.
- A script may never emit `user_*`, `usage` or `tool_response` chunks.

Build the template body with Python. It avoids the quoting problem of a source string inside JSON inside bash:

```bash
$PY -c 'import json,sys; print(json.dumps({"type":"agent","image":{"digest":sys.argv[1],"references":[{"registry":"docker.io","name":"diverge-local/python"}]},"memory":536870912,"disk":1073741824,"arguments":{"python":open(sys.argv[2],encoding="utf-8").read()}}))' \
  "$DIGEST_PY" "$WORK/agent.py" > "$WORK/python-template.json"
TPL_PY="$(diverge run agents templates create --jq '.created // .exists' "$(cat "$WORK/python-template.json")" | tr -d '"')"
echo "$TPL_PY"                                   # a hash; {"created":…} new, {"exists":…} made before
```

Why the body has a `references` entry:

- The daemon refuses a run unless one of the image's `references` names a registry that the provider lists
  (`diverge-daemon/src/containers/capable.rs`), so `docker.io` must appear.
- Nothing is pulled from docker.io. The provider finds the digest in its local cache first.

Create the agent, message it, and read its log:

```bash
diverge run agents create "{\"template\":\"$TPL_PY\",\"name\":\"echo\"}"
diverge run agents message '{"agent":{"name":"echo"},"content":[{"type":"text","text":"hello"}]}'
# expected: "delivered". The container starts on the first message.
diverge run logs '{"of":{"type":"agent","name":"echo"}}'
# the user part "hello", then an assistant_text_content "echo: hello"
diverge run agents list
```

An agent that needs secrets is bound at create time:

```bash
diverge run vault insert '{"key":"or-key","value":"sk-or-…"}'
diverge run agents create '{"template":"<id>","name":"x","secrets":{"OPENROUTER_API_KEY":"or-key"}}'
```

Its template must list `"secrets":["OPENROUTER_API_KEY"]`.

Other commands:

- `diverge run agents create request-schema` prints the JSON Schema of any request body. Swap in any request
  name.
- Add `--watch` to a list, or `"watch": true` to a `logs` body, to keep the stream open.

### Step 9: diverge-world against daemon A (dev mode)

```bash
cd "$REPO" && pnpm install
cd "$REPO/diverge-world" && pnpm tauri dev > "$WORK/world-a.log" 2>&1    # background; first run compiles Rust
```

What `pnpm tauri dev` does:

1. Runs `pnpm run dev`. That builds `@diverge/sdk` (`pnpm --filter @diverge/sdk build`), then starts Vite on 1440.
2. Builds and opens the Tauri app.

The app reads `$A/viewer.yaml`, since `DIVERGE_CONFIG` is unset and `~/.diverge` is the default, and it signs in
as `github.12345.admin` from `$A/identities`.

Check:

- The window shows the board: a dark grid with the provider node, the `github.12345.admin` account and the `echo` agent.
- Clicking the agent opens its sheet with these tabs: logs, filesystem, console, frontend.
- The composer under the log sends `agents message`.

Notes:

- The app redials every 5 s, so starting it before the daemon is fine.
- Don't run `pnpm typecheck` in `diverge-world` while the dev server runs. It rebuilds the SDK `dist` and breaks
  the running page.
- Editing Rust (`src-tauri` or `diverge-sdk-rs`) makes `tauri dev` rebuild and reopen the window.

### Step 10: daemon A to daemon B (relay and a connected tool)

What A needs on B:

- A reaches B **as an account on B**. For relayed daemon connections, B's `admins` do **not** count
  (`diverge-daemon/src/judge/admit.rs`), so B needs an account for the identity A presents.
- This guide has A present `github.12345.daemon-a`, which is signed in under root A.
- B must announce itself on a provider A is connected to (an `announcement` on B's record of it, step 6), and
  that record must have `accept_daemons: true` (the default).

**On B (as admin):** a role and an account for `github.12345.daemon-a`. This role is deliberately broad, so that world A's
relayed board and the tool steps below work. Trim it once things work.

```bash
cat > "$WORK/peer-role.json" <<'EOF'
{"name":"peer","description":"daemon A's account on this daemon","grants":[
 {"agents":["create"]},
 {"agents":{"actions":["get","list","logs","message","filetree","connect"],"within":"any"}},
 {"agents_templates":["create"]},
 {"agents_templates":{"actions":["get","list"],"within":"any"}},
 {"standalone_tools":["create"]},
 {"standalone_tools":{"actions":["get","list","logs","connect","attach","filetree"],"within":"any"}},
 {"standalone_tools_templates":["create"]},
 {"standalone_tools_templates":{"actions":["get","list"],"within":"any"}},
 {"connected_standalone_tools":{"actions":["get","list","logs"],"within":"any"}},
 {"dependency_tools":{"actions":["get","list","logs"],"within":"any"}},
 {"providers":{"actions":["get","list"],"within":"any"}},
 {"daemons":{"actions":["get","list"],"within":"any"}},
 {"accounts":{"actions":["get","list","logs"],"within":"any"}},
 {"volumes":{"actions":["get","list"],"within":"any"}},
 {"events":["list"]},
 {"images":["info"]}]}
EOF
diverge run --daemon b roles create "$(cat "$WORK/peer-role.json")"
diverge run --daemon b accounts create "{\"identity\":{\"issuer\":\"$ISS\",\"subject\":\"github.12345.daemon-a\"},\"roles\":[\"peer\"]}"
```

**On A (as admin):** check that B is visible through the provider, then record B.

```bash
diverge run providers daemons list        # should list github.12345.daemon-b with the provider as a route (a daemon is never listed to itself)
ID_B="$(diverge run daemons add --jq '.added // .exists' \
  "{\"daemon\":{\"issuer\":\"$ISS\",\"subject\":\"github.12345.daemon-b\"},\"identity\":{\"issuer\":\"$ISS\",\"subject\":\"github.12345.daemon-a\"},\"description\":\"daemon B\"}" | tr -d '"')"
echo "$ID_B"                               # base62, up to 11 characters
```

The record:

- `identity` and `daemon` may be from different issuers; here both are from the one issuer.
- The route filters `providers`, `ips` and `addresses` are optional; leaving them out allows any route.
- The ID hashes `{daemon, identity, routes}`, so changing any of them means deleting the record and adding a new
  one.
- Nothing connects at `add`. The first relay or connected tool that names the record does.

**A relay test:** A forwards the request to B, which serves it as `github.12345.daemon-a`'s account.

```bash
diverge run agents list "{\"relay\":[\"$ID_B\"]}"     # an empty list then "listed", not "forbidden"/error
```

**A tool on B, made through the relay**, so that it belongs to `github.12345.daemon-a`'s account there:

```bash
$PY -c 'import json,sys; print(json.dumps({"type":"tool","mode":"transient","image":{"digest":sys.argv[1],"references":[{"registry":"docker.io","name":"diverge-local/memory"}]},"memory":268435456,"disk":268435456,"arguments":{},"relay":[sys.argv[2]]}))' \
  "$DIGEST_MEM" "$ID_B" > "$WORK/memory-template.json"
TPL_MEM="$(diverge run standalone-tools templates create --jq '.created // .exists' "$(cat "$WORK/memory-template.json")" | tr -d '"')"
diverge run standalone-tools create "{\"template\":\"$TPL_MEM\",\"name\":\"notebook\",\"relay\":[\"$ID_B\"]}"
```

**Hold B's tool on A, attach it to the echo agent, and use it:**

```bash
diverge run connected-standalone-tools create "{\"daemon\":\"$ID_B\",\"tool\":{\"name\":\"notebook\"},\"name\":\"b-notebook\"}"
diverge run connected-standalone-tools attach-agent-instance '{"tool":{"name":"b-notebook"},"agent":{"name":"echo"}}'
diverge run agents message '{"agent":{"name":"echo"},"content":[{"type":"text","text":"tools"}]}'
diverge run agents message '{"agent":{"name":"echo"},"content":[{"type":"text","text":"call b-notebook_remember {\"text\":\"hi from A\"}"}]}'
diverge run logs '{"of":{"type":"agent","name":"echo"}}'
diverge run logs '{"of":{"type":"connected_standalone_tool","name":"b-notebook"}}'
```

What to expect:

- The `tools` reply lists the tool names exactly as the agent sees them. Use those names in `call`. The memory
  tool's tools are `remember`, `recall` and `forget`, and the agent sees each under the name the tool is held by
  on A: `b-notebook_remember`, `b-notebook_recall`, `b-notebook_forget`
  (`diverge-daemon/src/containers/mcp/prefix.rs`).
- The agent's log ends `tool said: {"id":1}`.
- The second log is every call A routed into B's tool. On B, the log of
  `{"of":{"type":"standalone_tool","name":"notebook"}}` shows the same calls arriving, their caller the client
  `github.12345.daemon-a`.

**In world A:**

- The Home board shows a **daemon** node (from `daemons list`) linked to the connected tool.
- The daemon's sheet has one tab, `daemon`, with a button "Open <name>". It opens a new tab on the window: B's
  board, with every request carrying `relay: [ID_B]`.
- That board shows what `github.12345.daemon-a`'s account sees on B.

### Step 11 (optional): a second diverge-world against daemon B

`diverge-world/CLAUDE.md` says `DIVERGE_CONFIG=/some/root pnpm tauri dev` runs a second copy, but Vite is pinned
to port 1440 with `strictPort: true`. A second `pnpm tauri dev` will most likely fail to start its own Vite while
the first runs (see §9). Two ways that avoid this:

```bash
# (a) Reuse the first copy's dev server. A debug build loads http://localhost:1440.
#     Run the app binary tauri dev built, against root B.
cd "$REPO" && DIVERGE_CONFIG="$(cygpath -w "$B" 2>/dev/null || echo "$B")" ./target/debug/diverge-world > "$WORK/world-b.log" 2>&1
#     (On Windows the binary is ./target/debug/diverge-world.exe.)

# (b) A second tauri dev with its beforeDevCommand turned off, so it uses the running Vite.
cd "$REPO/diverge-world" && DIVERGE_CONFIG="$(cygpath -w "$B" 2>/dev/null || echo "$B")" \
  pnpm tauri dev --config '{"build":{"beforeDevCommand":""}}' > "$WORK/world-b.log" 2>&1
```

Both share the page source with world A, which is fine because the page is identical. World B reads
`$B/viewer.yaml` and signs in as `github.12345.admin` from `$B/identities`.

The app also accepts `--config <dir>` as its first argument. `DIVERGE_CONFIG` is the documented route.

### Stopping

Stop the servers in this order. Each command returns once the program's lock is free.

```bash
diverge --config "$B" daemon stop
diverge daemon stop
diverge provider stop        # also removes every container this provider ran, and its tunnel
```

Each daemon's Postgres is stopped by its daemon. A postmaster that a killed daemon leaves behind is stopped
cleanly at the next start.

---

## 6. Verifying from the outside

| Check | Command |
|---|---|
| Which identities a root holds | `diverge --config <root> identities list` |
| Daemon up | `diverge run [--daemon b] info` |
| Provider connected to a daemon | `diverge run [--daemon b] providers list`, where `status` is `connected` |
| Peers visible through providers | `diverge run providers daemons list` |
| Remote-daemon records | `diverge run daemons list` |
| Images in the provider store | `pp image ls --no-trunc` |
| Containers running | `pp ps` |
| Server failures | `$A/provider.log`, `<root>/daemon.log` |
| Every request an account made, allowed or refused | `diverge run logs '{"of":{"type":"admin","issuer":"…","subject":"github.12345.admin"}}'`, or `{"type":"account",…}` (see `diverge run logs request-schema`) |

---

## 7. Troubleshooting (pitfalls actually hit)

| Symptom | Cause / fix |
|---|---|
| The shell hangs after `diverge … start` | Its output was piped. Always redirect to a file (§2). |
| Agent containers stop moments after a reply, or the world shows them dim | `idle_seconds` defaults to 10. Raise it in `daemon.yaml` and restart the daemon. |
| A deploy fails with the image unavailable, or the daemon says "the provider pulls from no registry the image's references name" | The image is not in the **provider's** podman store (it was built with your default podman connection or machine, not `--connection diverge-provider-root` plus the XDG variables), or it is only tagged under `127.0.0.1:`, or the template has no `references` entry naming a registry the provider lists (`docker.io`). |
| An image deploys but its program misbehaves against the proxy (refused enqueues, a finished channel with no answer) | The image predates the current inside-container contract. **Rebuild every image from current source.** |
| A plain `podman …` command fails or acts on the wrong machine | Podman's *default* connection may point at another or broken machine. Always pass `--connection diverge-provider-root` with the provider's XDG variables (the `pp` helper). On Windows, podman ignores `XDG_CONFIG_HOME`, so connection definitions live in `~/.config/containers` while the machine's disk and key live under `XDG_DATA_HOME`. |
| The provider refuses its machine (`Error::Machine`) | The machine's `SSHConfig.IdentityPath` is not under `storage_path`, for example because a `diverge-provider` machine was made by hand with other XDG variables. The provider removes and re-makes a machine it made itself. A machine made elsewhere has to be removed by hand. |
| `podman exec <ctr> /some/path` turns the path into `C:/Program Files/Git/some/path` | Git Bash path conversion. Prefix that one command with `MSYS_NO_PATHCONV=1`. |
| A Windows binary is handed a `/c/...` path through an environment variable | Use `cygpath -w` for `DIVERGE_CONFIG`, `XDG_*` and similar variables. Arguments are converted by Git Bash; environment values are less reliably converted. |
| `install.sh` fails with "Access is denied" or the move fails | The program is running. Stop it, then reinstall. |
| `daemon stop` targets the wrong daemon or says NotLocal | It uses the default daemon of `<root>/cli.yaml`. Fix that root's `cli.yaml`. |
| The first message to a fresh agent fails with "connection ended before the channel answered" | Its dependency tools may still be deploying. Send it again. |
| `providers add` record stays `disconnected` | The daemon identity is missing from `provider.yaml` `incoming`, or either program has no `identity` in its document or that identity is not signed in under its root, or the provider's `identity` is not the one the record names, or the provider is down. |
| `daemons add` works but relayed requests error | B is not announced (no `announcement` on B's record of the shared provider), B has no **account** for the presented identity (admins don't count), `accept_daemons` is false on B's record of that provider, or A and B are not on a common provider. |
| `pnpm tauri dev` page breaks after `pnpm typecheck` / an SDK rebuild | The SDK `dist` was rebuilt under the running Vite. Restart the dev server, or reload the window. |
| Second `pnpm tauri dev` exits at once | Port 1440 is taken (`strictPort`). Use §5 step 11 (a) or (b). |
| The daemon fails at start after an upgrade, or `providers add` errors on a database made by an earlier build | The daemon's database has no migrations, so a database made by an earlier build is not carried over: move `<root>/daemon/postgres/` aside (do not delete it) and let the daemon initialise a new one. Also remove `description`, `accept_daemons` and `capacity_poll_seconds` from `daemon.yaml`. |
| The `diverge-postgres` build fails fetching PostgreSQL | It fetches the bundled archive at build time. Check network access. GitHub rate limiting may need a `GITHUB_TOKEN` (unverified). |

---

## 8. Platform notes

**Windows (the live-proven platform)**

- Use Git Bash for every script.
- podman 6.x comes from the upstream MSI under `%LOCALAPPDATA%\Programs\Podman`. Put it on `PATH` in every shell
  that starts the provider or runs `pp`.
- WSL2 backs the machine. The provider creates the machine `diverge-provider` (rootful, data under
  `<root>\provider\podman_data`) and ignores the default `podman-machine-default`. Two running machines coexist
  fine on podman 6.1.4.
- Machine memory comes from WSL (`.wslconfig`); `containers.podman.memory` only applies on macOS.
- The provider sees host paths inside the machine as `/mnt/c/...`.
- The proxy binary at `<root>/bin/diverge-container-proxy` is mounted read-only into each container.

**macOS (run live on Apple Silicon, podman 6.1.2, the Apple hypervisor)**

- Same as Windows, with the same machine name, connection, XDG variables, `pp` helper and `ssh` tunnel.
- The provider adds `--memory <containers.podman.memory>` and a `--volume P:P` share for each of: every volume
  store, every fixed volume, `<root>/provider`, and the executable's directory.
- A machine missing a share is removed and re-made, which empties its image cache. Rebuild images after changing
  volume paths. A machine that sees every share is kept across provider restarts, its images with it: a first
  start takes about half a minute, a later one a second or two.
- `podman machine inspect` does not list a machine's mounts here. They are in the machine's description,
  `<storage_path>/containers/podman/machine/applehv/diverge-provider.json`, which is where the provider reads
  them; a description it cannot read is an error at its start, not a machine made again.
- In step 0, `PY=python3`, and the `cygpath` line does nothing.

**Linux (from code, not run live)**

- No machine. The provider runs `podman --root <storage_path> …` directly, **must run as root** (effective UID 0,
  else `Error::Root`), and rootless is unsupported.
- `storage_path` must be on a filesystem with project quotas (XFS `pquota`, or ext4 with project quota), because
  per-container `--storage-opt size=` is used.
- Build images with `sudo podman --root <that storage_path> build …`. Also run `image ls` and `image inspect` the
  same way.
- Under `sudo`, `$HOME` is root's, so always pass `--config` explicitly.
- How a root-owned provider shares `<root>/identities` with a user-run CLI and daemon is not worked out here. A
  separate root for the provider, with its own `github.12345.provider` login, is the obvious arrangement, but it is untested.

---

## 9. Open questions / not verified

1. **Daemon-to-daemon (step 10): the CLI half is run live, the world half is not.** On macOS, two daemons on one
   provider: the relay, a tool made on B through it, and a connected tool called by A's agent all work as written.
   The requirements held as `diverge-daemon/src/judge/admit.rs` states them: an account (not an admin) for the
   presented identity — a record presenting an administrator that is no account is refused at once — an
   announced and accepting B, and a shared provider. What diverge-world shows of it (the daemon node, its "Open"
   tab, B's board) was not run.
2. **Which tool `connected-standalone-tools create` may name.** Its `tool` is "a standalone tool of the caller's"
   as B reads it, i.e. of `github.12345.daemon-a`'s account. That is why step 10 creates the tool on B *through the relay*. It
   is not verified whether a tool created by B's admin could be named instead.
3. **The `peer` role's grant set** is generous on purpose. Under it, each of the eleven lists world A's relayed
   board opens answers through the relay. Which of its grants could go was not established, nor whether what
   `github.12345.daemon-a`'s account sees on B is limited to what it owns.
4. **World B's view as `admin`.** Asked by the CLI as `admin` on B, `standalone-tools list` shows `notebook`, which
   `github.12345.daemon-a`'s account made: an administrator's listing shows other accounts' records. World B was not run
   against it.
5. **The second diverge-world copy.** The CLAUDE.md recipe conflicts with Vite's `strictPort: 1440`. Workaround
   (a) in step 11 was run on macOS at an earlier revision, before the handshake changed, and worked: the debug
   binary started with `DIVERGE_CONFIG` set to root B opened B's board beside world A's. It was not run again
   since. Workaround (b) is reasoned from Tauri v2's behaviour (`--config` merges JSON) and was not run.
6. **`memory` as a standalone tool.** Settled, run live: the image serves as a standalone tool, on its own daemon
   and to another daemon's agent. Its DIVERGE.md still documents it as a dependency tool only.
7. **The python script's tool-call round trip.** Settled, run live in the container: after an
   `assistant_tool_call` the loop runs the script again with the `tool_response` as the last `input` item.
8. **Whether `"type":"agent"` / `"type":"tool"` is required in template bodies.** The CLI schema omits it, but the
   SDK's `Template` flattens a serde-tagged `type`. Including it is always safe, so this guide includes it.
9. **Linux provider behaviour** comes from code only (§8); macOS was run live. On Linux, the arrangement of a
   root-run provider next to a user-run CLI and daemon is undecided.
10. **cc image credentials.** The cc image declares no secrets and says Claude Code logs in by itself. How it gets a
    login inside a container (a mount of `/root/.claude`?) was not established.
11. **Whether `diverge-postgres` needs `GITHUB_TOKEN`** to build in a rate-limited environment.
12. **Several loop DIVERGE.md files** still describe a `permissions` member on dependency templates that the current
    SDK no longer has. Permissions moved to the tool's own registration. Those schemas are stale on that point.
