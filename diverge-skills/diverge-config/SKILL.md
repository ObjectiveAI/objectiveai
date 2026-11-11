---
name: diverge-config
description: Configuring and operating a Diverge installation's root. Load it to find or choose the config root (`--config`, `DIVERGE_CONFIG`, `./.diverge`, `~/.diverge`); to read or change `daemon.yaml`, `provider.yaml`, `issuer.yaml`, `cli.yaml` or `viewer.yaml` with `diverge <program> config get|set|patch`; to add a daemon or an account to the CLI; to sign an identity in, list or remove one (`diverge identities login --password-stdin`, `diverge run identities …` on the daemon's host); to manage the issuer's subjects, passwords and signing key (`diverge issuer identities add|list|remove|password`, `diverge issuer rotate`); to start or stop the daemon, provider or issuer; to look up ports, admins, `idle_seconds`, registries, volume stores; or to work out what must be restarted after a change and why a connection or handshake is refused.
---

# Diverge: the root, the documents, identities, start and stop

Everything a Diverge program is told lives in one directory, the **root**: one YAML document per program, the
signed-in identities, logs and locks. This skill covers every command that works on the root — everything in
the `diverge` CLI that is not a request to a daemon's records — plus `diverge run identities`, the same
identity work done on a daemon's host.

If you have not loaded the `diverge` skill in this task, load it first.

## The root

```
<root>/
  daemon.yaml  provider.yaml  issuer.yaml  cli.yaml  viewer.yaml     the documents (each optional)
  identities/<percent-encoded issuer>/<subject>.json                 signed-in identities (hold refresh tokens)
  issuer.sqlite                                                      the issuer's subjects, hashes, keys
  daemon.log  provider.log  issuer.log                               a started server's output, truncated at each start
  daemon.lock provider.lock issuer.lock                              held while that server runs; holds its process id
  daemon/  provider/  viewer/                                        each program's own state
  bin/                                                               the binaries, in a default installation
```

Which directory is the root:

| Program | Order |
|---|---|
| The CLI (`diverge`) | `--config <dir>`, else `DIVERGE_CONFIG`, else `./.diverge` **if that directory exists**, else `~/.diverge` |
| Daemon, provider, issuer, viewer | `--config <dir>`, else `DIVERGE_CONFIG`, else `~/.diverge` |

- The root is created if it does not exist. A mistyped `--config` path silently makes an empty root with default
  documents.
- `--config` is written before the command: `diverge --config <root> daemon config get`.
- A server started by the CLI is handed the CLI's root as `--config`, so it runs on the root the CLI found.
- One of each server per root (the lock enforces it). A second daemon on the same host is a second root with
  its own `daemon.yaml`, `cli.yaml` and identities.
- Identities are per root: a program uses only the identities signed in under its own root.

## Ports

| Port | Who | Set in |
|---|---|---|
| 14979 | provider (all interfaces) | `provider.yaml` `port` |
| 14980 | daemon (all interfaces) | `daemon.yaml` `port` — and repeated in `cli.yaml` and `viewer.yaml` addresses |
| 14981 | issuer, plain HTTP (all interfaces) | `issuer.yaml` `port` — and inside every `issuer` URL elsewhere |
| random loopback | the daemon's own Postgres | picked at every daemon start |

## The documents at a glance

Every field of every document, with defaults, is in [references/documents.md](references/documents.md). Read it
before changing a field you have not changed before.

| Document | Read | Holds |
|---|---|---|
| `daemon.yaml` | at daemon start | `port`, `identity` (`{issuer, subject}`: who the daemon is), `admins`, `postgres` (`local` or `remote`), `idle_seconds`. Nothing per provider lives here: whether the daemon announces itself, accepts other daemons and hears messages is on each provider's record (`diverge run providers edit`, skill `diverge-daemons`) |
| `provider.yaml` | at provider start | `port`, `identity` (who the provider is), `incoming` (who may dial it), `outgoing` (peers it dials, each an `address` and the `identity` that must answer), `containers` (podman: `registries`, `storage_path`, `image_cache_disk`, `container_overlay_disk`, `memory`; `enabled`), `volumes` (`stores`, `fixed`), `images` (the Store: `path`, `capacity`), `messages` (rate) |
| `issuer.yaml` | at issuer start | `port`, `issuer` (the identifier URL), `access_token_seconds`, `refresh_token_days` |
| `cli.yaml` | by every CLI command | `default` daemon; `daemons.<name>`: `address`, `default` account, `accounts.<name>`: `issuer`, `subject` |
| `viewer.yaml` | at viewer start | `daemon`: `address`, `issuer`, `subject`, `redial` |

Rules common to all five:

- An absent file is the built-in defaults; nothing needs to exist for `config get` to work.
- **An unknown key is refused**, with the path to it. A typo never passes silently.
- No document holds a token. The only secrets in any of them are `postgres.password` (daemon, remote kind) and
  registry `credential`s (provider). Do not print those documents into logs or chat without need.
- The `issuer` string must be identical everywhere it appears — in `issuer.yaml`, in logins, in `identity`,
  `admins`, `incoming`, `cli.yaml`, `viewer.yaml`. `http://127.0.0.1:14981` and
  `http://localhost:14981` are different issuers.

## Reading and changing a document: `config get | set | patch`

The same three subcommands exist beneath each program: `diverge daemon config`, `diverge provider config`,
`diverge issuer config`, `diverge viewer config`, `diverge cli config`. They work on the file in the root and
never contact a running program.

### get — read, through jq

```bash
diverge daemon config get                    # the whole document, one compact JSON line
diverge daemon config get .idle_seconds      # 10
diverge cli config get '.daemons | keys[]'   # one line per value the filter yields
diverge provider config get '.incoming[]?'
```

The document is shown as JSON. Empty optional members are left out (`identity`, `admins`, `incoming`,
`volumes`, …), so a filter over one may see `null`.

### set — rewrite the whole document with a jq program

The program runs with the document as input and must yield **exactly one value**, which becomes the whole new
document.

```bash
diverge daemon config set '.idle_seconds = 120'
diverge daemon config set '.admins = (.admins // []) + [{issuer:"http://127.0.0.1:14981", subject:"admin"}]'
diverge daemon config set '.identity = {issuer:"http://127.0.0.1:14981", subject:"daemon-a"}'
diverge viewer config set '.daemon = {address:"ws://127.0.0.1:14980/", issuer:"http://127.0.0.1:14981", subject:"admin"}'
```

Pitfall: a member left out of the document is `null` to the program. Default a list before adding to it
(`(.admins // [])`, as above), and set `identity` whole: it is one `{issuer, subject}` object, and one member
of it alone is not a valid document.

### patch — change places with a JSON Patch (RFC 6902)

The argument is a list of operations written as YAML (JSON is YAML). Operations: `add`, `remove`, `replace`,
`move`, `copy`, `test`. Paths are JSON Pointers: `/port`, `/admins/0`, `/admins/-` (append; `add` only); inside
a key, `/` is written `~1` and `~` is `~0`. Operations apply in order, **all or none**.

```bash
diverge daemon config patch '[{op: replace, path: /idle_seconds, value: 120}]'
diverge provider config patch '[{op: add, path: /incoming/-, value: {issuer: "http://127.0.0.1:14981", subject: daemon-b}}]'
diverge provider config patch '[{op: add, path: /messages, value: {messages: 60, seconds: 60}}]'
diverge daemon config patch '[{op: add, path: /identity, value: {issuer: "https://auth.example.com", subject: daemon-1}}]'
diverge daemon config patch '[{op: test, path: /port, value: 14980}, {op: remove, path: /admins/0}]'
diverge cli config patch '[{op: add, path: /daemons/b, value: {address: "ws://127.0.0.1:14982/", default: admin, accounts: {admin: {issuer: "http://127.0.0.1:14981", subject: admin}}}}]'
```

- `add` to an object makes or replaces the member; `add` to a list inserts at the index. `replace` and `remove`
  need the place to exist already.
- The parent must exist: `/incoming/-` fails with `nothing is at …` when the document has no `incoming` yet.
  Add the whole member instead: `[{op: add, path: /incoming, value: [{…}]}]`.
- Use `test` first when a patch depends on a list index, so a document that changed underneath fails the patch
  rather than removing the wrong element.

### What `set` and `patch` do to the file

- The result is validated against the program's own type before anything is written. A refused result changes
  nothing: `Error: the document is not one its program takes: bogus: unknown field …`, exit 1.
- The file is rewritten whole, atomically, as the type's own YAML: **comments are lost** and keys come out in
  the type's order (the CLI's daemons and accounts keep theirs). Members equal to "none" are dropped.
- Validation is of shape only. A wrong port, an identity not signed in, or a path that does not exist is found
  when the program starts.
- The running program is not told. See "What needs a restart".
- Editing the YAML file by hand is equally valid, and is the only way to keep comments.

## Identities on this host: `diverge identities`

An identity is one subject at one issuer, signed in once and kept signed in by its refresh token. All programs
on a root share `<root>/identities/`. No command prints a token.

### login

```bash
# How an agent signs in at Diverge's own issuer: no browser, password on stdin.
printf '%s' "$PASSWORD" | diverge identities login \
  --issuer http://127.0.0.1:14981 --subject admin --client-id diverge --password-stdin
# → {"issuer":"http://127.0.0.1:14981","subject":"admin"}
```

| Flag | Meaning |
|---|---|
| `--issuer` (required) | The issuer, exactly as its discovery document names it. `https`, or `http` on loopback. |
| `--subject` (required) | The subject to sign in as. A grant as any other subject writes nothing and fails. |
| `--client-id` (required) | The public client to sign in as. At Diverge's own issuer it is always `diverge`. |
| `--password-stdin` | Read the password from stdin (to its end, trailing newline dropped; empty is refused) and sign in with no browser. This is the mode for an agent, and it is specific to issuers that accept the password as Basic credentials at the authorization endpoint, as Diverge's does. |
| *(neither mode flag)* | OpenID Connect Authorization Code flow: prints `{"authorization_uri":…}`, opens the browser there, and waits for the redirect on a loopback port of this host. Needs a person. |
| `--device` | Device authorization grant: prints `{"user_code","verification_uri","verification_uri_complete","expires_in"}` and waits for a person to approve it in any browser. For external issuers that offer it; Diverge's own issuer has no device flow. |
| `--no-browser` | Do not open a browser; the printed line names the page. |
| `--scope` | Space-separated; `openid` and `offline_access` are added if missing. Absent, those two. |
| `--resource` | RFC 8707 `resource` for the access token. Rarely needed. |

Facts and pitfalls:

- A login replaces an identity already there, signed out or not. Logging in again is how a signed-out identity
  is repaired.
- The interactive modes block until a person acts. Do not run them unattended; use `--password-stdin`, or run
  with `--no-browser` and give the printed URL to the user.
- Pass the password by stdin from a variable or file. Never put it on the command line or in a document.
- The issuer must be reachable (its discovery document is fetched): start the issuer first.
- An identity unused for longer than the issuer's refresh-token lifetime (30 days by default at Diverge's
  issuer), or whose refresh tokens were revoked, is *signed out*: its file stays, handshakes fail, and a new
  login fixes it.

### list, remove

```bash
diverge identities list
# {"issuer":"http://127.0.0.1:14981","subject":"admin"}      one line per identity; signed-out ones included
diverge identities remove --issuer http://127.0.0.1:14981 --subject old
# {"removed":{"issuer":"…","subject":"old"}}
```

Removing the identity a running daemon or provider presents (its `identity`) breaks every later handshake
of that program. Removing one that `cli.yaml` or `viewer.yaml` names breaks that account.

## Identities on the daemon's host: `diverge run identities`

The same store, on the host of the daemon you dial — for when the daemon runs somewhere you have no shell.
These are requests to the daemon: they need an account with grants over `identities` (an admin has them), take
the usual `run` flags and `relay`, and answer `"forbidden"` otherwise.

```bash
diverge run identities list                                   # {"listed":[{"issuer":…,"subject":…,"tags":[…]}, …]}
diverge run identities list --issuer http://127.0.0.1:14981   # also --subject; body: issuers, subjects, all_tags, any_tags
diverge run identities tag   '{"identity":{"issuer":"http://127.0.0.1:14981","subject":"daemon-a"},"tags":["prod"]}'   # "tagged" | "not_found"
diverge run identities untag '{"identity":{"issuer":"http://127.0.0.1:14981","subject":"daemon-a"},"tags":["prod"]}'   # "untagged" | "not_found"
printf '%s' "$PASSWORD" | diverge run identities login \
  --issuer http://127.0.0.1:14981 --subject daemon-a --client-id diverge --password-stdin    # {"signed_in":{…}}
```

- `list` is one snapshot (`{"listed":[…]}`), not a stream; there is no `--watch`. Tags exist only so grants can
  reach some identities and not others (skill `diverge-access`).
- `login` takes `--issuer --subject --client-id [--scope --resource --no-browser --password-stdin]`, or the same
  members in a BODY (`issuer`, `subject`, `client_id`, `scope`, `resource`, `password`). With a password the
  daemon signs in on its own host and answers the outcome. Without one it starts a device login and prints
  `{"pending":{"user_code",…}}`, then the outcome when a person approves. There is no browser-redirect login
  here.
- Outcomes: `{"signed_in":{…}}`; `"rejected"` (the issuer refused the subject and password); `"denied"` and
  `"expired"` (device login not approved); `{"other_subject":{…}}` (approved as someone else; nothing written);
  `"held"` (another login or reader holds that identity's lock right now; try again); `"forbidden"`;
  `{"error":…}` (issuer unreachable, or the identity could not be written). Ending the
  command ends the login and writes nothing.
- Signing an identity in does not make the daemon present it: which identity the daemon presents is
  still `daemon.yaml` `identity`, read at start.

## The issuer: `diverge issuer`

Diverge's own OpenID Connect provider. It has exactly one client, the public client `diverge`; no client
secrets, no web pages. Its subjects are managed here, by the CLI writing `<root>/issuer.sqlite` directly, so
these commands work whether the issuer is running or not, need no account, and act on the root's issuer only.

```bash
printf '%s' "$PASSWORD" | diverge issuer identities add alice        # {"added":"alice"}; refused if alice exists
diverge issuer identities list                                       # {"subject":"alice","made":…} per line
printf '%s' "$NEW" | diverge issuer identities password alice        # {"changed":true}; false = no such subject
diverge issuer identities remove alice                               # {"removed":true}; false = no such subject
diverge issuer rotate                                                # {"kid":…,"made":…,"removed":…,"signed_out":…,"takes_effect":"now"}
diverge issuer rotate --sign-out
```

| Command | Effect beyond the obvious |
|---|---|
| `issuer identities add <subject>` | Password from stdin (trailing newline dropped, empty refused). The subject is the `sub` of every token issued to it. Adding a subject signs nothing in: follow with `diverge identities login` under the root of the program that will use it. |
| `issuer identities password <subject>` | Revokes every refresh token of that subject: **every program signed in as it is signed out** and must log in again with the new password. |
| `issuer identities remove <subject>` | Also removes every code and refresh token issued to it. Programs signed in as it stop working when their current token expires (10 minutes by default). |
| `issuer rotate` | Mints a new signing key and removes the old. Tokens signed before stop verifying; signed-in programs refresh by themselves and carry on. A running issuer rotates at once (`"takes_effect":"now"`); a stopped one from its next start. |
| `issuer rotate --sign-out` | Also revokes every refresh token: **every identity of this issuer, in every root, must log in again**, including the daemon's and the provider's own. Use only when the user asks to sign everyone out. |

A subject at the issuer is not yet anything to a daemon. To let it use a daemon: add the subject here, sign it
in under the client's root, name it in that root's `cli.yaml`, and give it an account on the daemon
(`diverge run accounts create`, skill `diverge-access`) or list it under `admins` in `daemon.yaml`.

## Start and stop: `diverge daemon|provider|issuer start|stop`

```bash
diverge issuer start   > issuer-start.out   2>&1; cat issuer-start.out     # started issuer, process <pid>
diverge provider start > provider-start.out 2>&1; cat provider-start.out   # started provider, process <pid>
diverge daemon start   > daemon-start.out   2>&1; cat daemon-start.out     # started daemon, process <pid>

diverge daemon stop      # stopped daemon      (or: daemon is not running — exit 0)
diverge provider stop
diverge issuer stop
```

**Never pipe a `start`** (no `|`, no `$(…)`): the detached server inherits the pipe and the shell hangs.
Redirect to a file, as above.

`start`:

- Runs the program's binary from the directory the `diverge` executable is in, detached, with `--config <root>`.
  It returns once the program holds the root's lock.
- Refuses when one already runs on this root: `the daemon is already running on this root, as process <pid>`.
- If the program dies first, the error carries its whole log. Usual causes: the port is taken, a document it
  refuses, a root it cannot use, and for the provider podman or `ssh` missing from `PATH`. The same text is in `<root>/<program>.log`.
- The log file is truncated at every start. A program that starts well prints nothing.
- The provider's first start on Windows or macOS creates and boots a podman machine and can take minutes.

`stop`:

- Asks the program to stop over its own port from this host, and returns once the lock is free — which is the
  whole teardown done. A daemon stop ends every container run and its Postgres; a **provider stop removes every
  container it ran**.
- `daemon stop` finds the port from **`cli.yaml`: the address of the default daemon**, not from `daemon.yaml`.
  That address must be on this host (`127.0.0.1`, `localhost`, `::1`), else `… is not this host`. So each root's
  `cli.yaml` default must point at that root's own daemon, and must be updated when `daemon.yaml` `port`
  changes. `provider stop` and `issuer stop` read the port from their own documents.
- `nothing is listening on port …: the program is not running, or is still starting` means the lock is held but
  the port is not open yet (or `cli.yaml` names the wrong port). Wait and retry, or fix the address.
- A stop is honoured only from the same host; there is no remote stop.

Order. Start: issuer, then provider, then daemon (each needs the one before to sign in or connect; a daemon
redials its providers every 5 seconds, so a late provider is picked up). Stop in reverse: daemon, provider,
issuer. With several roots, stop every daemon before the provider they share.

There is no `restart`: stop, then start. There is no `viewer start`: the world viewer is a desktop app started
outside the CLI.

## What needs a restart

No server watches its document. A change takes effect as follows.

| Change | Takes effect |
|---|---|
| Anything in `daemon.yaml` | At the next daemon start: `diverge daemon stop`, then `start`. Agents and tools keep their records; running containers are ended and start again on next use. |
| `daemon.yaml` `port` | Daemon restart **and** update every `cli.yaml` and `viewer.yaml` address that names it (before the stop, `cli.yaml` must still hold the old port, or the stop cannot find the daemon). Other daemons and providers that dial this one by address need their records edited. |
| `daemon.yaml` `postgres` | Daemon restart. A different database is a different set of records: the daemon comes up with whatever that database holds. |
| Anything in `provider.yaml` | At the next provider start. Stopping the provider removes every container it ran; connected daemons show it `disconnected`, redial every 5 seconds, and start containers again on next use. |
| `provider.yaml` `containers.podman.storage_path` | Provider restart onto a fresh, empty podman store (or a new machine): every image built into the old one must be built again. |
| Anything in `issuer.yaml` | At the next issuer start. Tokens already issued stay valid until they expire, if the keys are unchanged. |
| `issuer.yaml` `issuer`, or `port` while `issuer` is absent | This changes the issuer's identity. Every identity signed in under the old URL is useless: log each in again under the new URL and replace the old URL in every document and in every daemon's account records. Avoid unless asked. |
| `cli.yaml` | The next CLI command. Nothing to restart. |
| `viewer.yaml` | The next start of the viewer app. |
| `diverge identities login` of an identity a running program already uses | No restart: programs read the identity's file whenever they need a token. |
| A program's `identity` changed, or a *new* entry in `admins` or `incoming` | Restart that program. Its own `identity` must be signed in under its root first. |
| `diverge issuer identities …`, `diverge issuer rotate` | Immediate; no restart. |
| Records in the daemon (providers, accounts, roles, agents, …) | Immediate; those are `diverge run` requests, not configuration. |

Before restarting anything, say what it will interrupt and make sure the user asked for it.

## Recipes

**Add an account to the CLI and make it the default.**

```bash
diverge cli config patch '[
  {op: add, path: /daemons/local/accounts/me, value: {issuer: "http://127.0.0.1:14981", subject: admin}},
  {op: add, path: /daemons/local/default, value: me}]'
diverge run info        # {"info":{"version":"…"}}
```

**Add a second daemon to the CLI** — the `cli config patch` example with `/daemons/b` above — then
`diverge run --daemon b info`.

**Let a daemon use a provider.** In the provider's root: the daemon's identity goes into `incoming`, then the
provider restarts. In the daemon's root nothing changes; the provider record is added with
`diverge run providers add` (skill `diverge-daemons`).

**Keep containers up longer.** `diverge daemon config patch '[{op: replace, path: /idle_seconds, value: 300}]'`,
then restart the daemon.

**Give the provider volumes.**
`diverge provider config patch '[{op: add, path: /volumes, value: {stores: [{path: provider/volumes, capacity: 107374182400}]}}]'`,
then restart the provider. (Volumes themselves: skill `diverge-storage`.)

## When something refuses

| Message | Cause | Fix |
|---|---|---|
| `no account was named, and the daemon … has no default` | `cli.yaml` has no default account for that daemon (this stops even `request-schema`) | Add an account and `default`, or pass `--account` |
| `no daemon is named …` / `the daemon … has no account named …` | `--daemon` / `--account` name nothing in `cli.yaml` | `diverge cli config get` |
| `the daemon could not be dialled: …` | Daemon down, or wrong `address` in `cli.yaml` | `diverge daemon start`, or fix the address |
| `the handshake did not complete: …` | The account's identity is not signed in under this root, is signed out, the issuer is unreachable, or the daemon has no `identity` or its identity is not signed in under the daemon's root | `diverge identities list`; log in again; check `daemon.yaml` `identity` |
| `the daemon closed the connection without answering: the account may be none of the daemon's` | The identity is neither in `admins` nor an account on that daemon | Add it to `admins` and restart, or have an admin create the account |
| `the document is not one its program takes: <path>: …` | A `set`/`patch` result the type refuses | Fix the key or value at that path; nothing was written |
| `the patch could not be applied: #<n> <op>: nothing is at …` | The path, or its parent, does not exist | Add the parent member whole |
| `the jq program: …` | The filter does not parse, or failed on a missing member | Default absent members with `//` |
| `` `…yaml` could not be read: … `` | The file on disk does not parse as its type (hand edit) | Fix the file by hand; `config set` cannot run on it |
| A provider record stays `disconnected` | Daemon's identity missing from `provider.yaml` `incoming`; no `identity` in `daemon.yaml` or `provider.yaml`, or it is not signed in; the provider's `identity` is not the one the record names; provider down | Fix, restart the provider; the daemon redials by itself |
| Wrong documents are being read | A `./.diverge` in the working directory, or `DIVERGE_CONFIG` set | `--config <root>` explicitly |

## Not verified

- On Linux the provider must run as root while the CLI and daemon usually do not; how one root's `identities/`
  is shared between them (or whether the provider should have a root of its own) is not established.
- `diverge issuer identities …` and an offline `issuer rotate` writing `issuer.sqlite` while the issuer runs is
  how the setup guide uses them and how the code is written (the CLI opens the same database), but concurrent
  behaviour under load was not exercised.
- What a daemon or provider does when the identity its `identity` names is not signed in, or is signed
  out — whether it refuses to start or starts and fails each handshake — was not traced. Either
  way the fix is a login under that root (and a restart if the program did not start).
- Whether a `remote` Postgres needs anything prepared in it before the daemon's first start was not checked.

## When this task is done

Load the `diverge` skill again before starting the next Diverge task: it is the index, and it says which
skill the next task needs.
