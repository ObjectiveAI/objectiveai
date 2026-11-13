---
name: diverge-config
description: Configuring and operating a Diverge installation's root. Load it to find or choose the config root (`--config`, `DIVERGE_CONFIG`, `./.diverge`, `~/.diverge`); to read or change `daemon.yaml`, `provider.yaml`, `cli.yaml` or `viewer.yaml` with `diverge <program> config get|set|patch`; to add a daemon or an account to the CLI; to sign an identity in, list or remove one (`diverge identities login`; the accounts at Diverge's own issuer, `diverge identity.diverge.network …`; `diverge run identities …` on the daemon's host); to start or stop the daemon or provider; to look up ports, admins, `idle_seconds`, registries, volume stores; or to work out what must be restarted after a change and why a connection or handshake is refused.
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
  daemon.yaml  provider.yaml  cli.yaml  viewer.yaml                  the documents (each optional)
  identities/<percent-encoded issuer>/<subject>.json                 signed-in identities (hold refresh tokens)
  identity.diverge.network/<issuer>.<id>.json                        signed-in accounts at Diverge's own issuer (hold GitHub / Google tokens)
  daemon.log  provider.log                                           a started server's output, truncated at each start
  daemon.lock provider.lock                                          held while that server runs; holds its process id
  daemon/  provider/  viewer/                                        each program's own state
  bin/                                                               the binaries, in a default installation
```

Which directory is the root:

| Program | Order |
|---|---|
| The CLI (`diverge`) | `--config <dir>`, else `DIVERGE_CONFIG`, else `./.diverge` **if that directory exists**, else `~/.diverge` |
| Daemon, provider, viewer | `--config <dir>`, else `DIVERGE_CONFIG`, else `~/.diverge` |

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
| random loopback | the daemon's own Postgres | picked at every daemon start |

## The documents at a glance

Every field of every document, with defaults, is in [references/documents.md](references/documents.md). Read it
before changing a field you have not changed before.

| Document | Read | Holds |
|---|---|---|
| `daemon.yaml` | at daemon start | `port`, `identity` (`{issuer, subject}`: who the daemon is), `admins`, `postgres` (`local` or `remote`), `idle_seconds`. Nothing per provider lives here: whether the daemon announces itself, accepts other daemons and hears messages is on each provider's record (`diverge run providers edit`, skill `diverge-daemons`) |
| `provider.yaml` | at provider start | `port`, `identity` (who the provider is), `incoming` (who may dial it), `outgoing` (peers it dials, each an `address` and the `identity` that must answer), `containers` (`enabled`, and the runtime block: `registries`, `storage_path`, `image_cache_disk`, `container_overlay_disk`, `memory`), `volumes` (`stores`, `fixed`), `images` (the Store: `path`, `capacity`), `messages` (rate) |
| `cli.yaml` | by every CLI command | `default` daemon; `daemons.<name>`: `address`, `default` account, `accounts.<name>`: `issuer`, `subject` |
| `viewer.yaml` | at viewer start | `daemon`: `address`, `issuer`, `subject`, `redial` |

Rules common to all four:

- An absent file is the built-in defaults; nothing needs to exist for `config get` to work.
- **An unknown key is refused**, with the path to it. A typo never passes silently.
- No document holds a token. The only secrets in any of them are `postgres.password` (daemon, remote kind) and
  registry `credential`s (provider). Do not print those documents into logs or chat without need.
- The `issuer` string must be identical everywhere it appears — in logins, in `identity`,
  `admins`, `incoming`, `cli.yaml`, `viewer.yaml`. `https://identity.diverge.network` and
  `https://identity.diverge.network/` are different issuers.

## Reading and changing a document: `config get | set | patch`

The same three subcommands exist beneath each program: `diverge daemon config`, `diverge provider config`,
`diverge viewer config`, `diverge cli config`. They work on the file in the root and
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
diverge daemon config set '.admins = (.admins // []) + [{issuer:"https://identity.diverge.network", subject:"github.12345.admin"}]'
diverge daemon config set '.identity = {issuer:"https://identity.diverge.network", subject:"github.12345.daemon-a"}'
diverge viewer config set '.daemon = {address:"ws://127.0.0.1:14980/", issuer:"https://identity.diverge.network", subject:"github.12345.admin"}'
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
diverge provider config patch '[{op: add, path: /incoming/-, value: {issuer: "https://identity.diverge.network", subject: github.12345.daemon-b}}]'
diverge provider config patch '[{op: add, path: /messages, value: {messages: 60, seconds: 60}}]'
diverge daemon config patch '[{op: add, path: /identity, value: {issuer: "https://auth.example.com", subject: daemon-1}}]'
diverge daemon config patch '[{op: test, path: /port, value: 14980}, {op: remove, path: /admins/0}]'
diverge cli config patch '[{op: add, path: /daemons/b, value: {address: "ws://127.0.0.1:14982/", default: admin, accounts: {admin: {issuer: "https://identity.diverge.network", subject: github.12345.admin}}}}]'
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

## Accounts at Diverge's own issuer: `diverge identity.diverge.network`

`https://identity.diverge.network` is Diverge's hosted issuer. Every subject there is `<issuer>.<id>.<name>`:

- `<issuer>` is `github` or `google`: who issued the owner's own sign-in. In these commands "issuer" always
  means GitHub or Google, never `identity.diverge.network` itself.
- `<id>` is the owner's permanent user id there.
- `<name>` is chosen by the owner: 1 to 64 characters of `a-z`, `0-9`, `-`, `_`.

An **account** is `<issuer>.<id>`. It owns every subject that starts with it. Sign an account in once per
root; after that, signing any of its subjects in needs no person (see the next section).

```bash
diverge identity.diverge.network accounts login github
# {"user_code":"ABCD-EFGH","verification_uri":"https://github.com/login/device","verification_uri_complete":null,"expires_in":899,"browser_opened":true}
diverge identity.diverge.network accounts list
# {"account":"github.12345","issuer":"github","id":"12345"}        one line per account
diverge identity.diverge.network accounts info github 12345
# {"account":"github.12345","issuer":"github","id":"12345","info":{…}}
diverge identity.diverge.network subjects list github 12345
# {"subject":"github.12345.daemon-a","name":"daemon-a","refresh_tokens":2}     one line per subject signed in somewhere
diverge identity.diverge.network refresh-tokens list github 12345 daemon-a
# {"id":"…","created":…,"ip":"…","user_agent":"…","last_used":…,"last_ip":"…"}   one line per place it is signed in
diverge identity.diverge.network refresh-tokens end github 12345 daemon-a <id>
# {"ended":"<id>"}
```

| Command | What it does |
|---|---|
| `accounts login <issuer>` | Starts a device login at GitHub or Google, opens that page in this host's browser, prints the code, and **ends**. A person enters the code on the page. A background process saves the account once it is approved. `--no-browser` opens nothing. |
| `accounts list` | The accounts signed in under this root. Reads no token, asks nobody. |
| `accounts info <issuer> <id>` | What GitHub or Google says about that user right now. The `info` object is the issuer's own and differs between the two. Never cached. |
| `subjects list <issuer> <id>` | The account's subjects that are signed in somewhere, with how many places each. |
| `refresh-tokens list <issuer> <id> <name>` | Where one subject is signed in: one entry per sign-in, with the address and program that started it and when it was last used. |
| `refresh-tokens end <issuer> <id> <name> <refresh-token-id>` | Signs one of them out. Tokens it already holds last until they expire (15 minutes). |

Facts and pitfalls:

- **`accounts login` needs a person once.** Read `user_code` from the printed line and give it to the person.
  If `browser_opened` is `false`, also give them `verification_uri`. `true` means a browser was started, not
  that anyone saw the page, so the address is always in the line.
- **`accounts login` prints no outcome.** Check with `accounts list`: the account appears once the code was
  approved. If it was denied or expired, nothing is written: run it again.
- The account's id is not chosen: it is whoever approved the code. One person signing in twice replaces the
  same account.
- An account whose issuer no longer accepts its token answers an error that says it is signed out. Run
  `accounts login` again; the file is replaced.
- A subject is listed only while it is signed in somewhere. Ending its last refresh token makes it vanish
  from `subjects list`; signing in under the same name brings the same subject back.
- No command here prints a token.
- `DIVERGE_IDENTITY_URL` points these commands at another copy of the service. Unset, they use
  `https://identity.diverge.network`.

## Identities on this host: `diverge identities`

An identity is one subject at one issuer, signed in once and kept signed in by its refresh token. All programs
on a root share `<root>/identities/`. No command prints a token.

### login

```bash
# At Diverge's own issuer: nothing to approve, the command does it and waits.
diverge identities login --issuer https://identity.diverge.network --subject github.12345.admin --client-id diverge
# → {"issuer":"https://identity.diverge.network","subject":"github.12345.admin"}     signed in

# At any other issuer: a code to approve there.
diverge identities login --issuer https://auth.example.com --subject alice --client-id my-client
# → {"user_code":"…","verification_uri":"…","verification_uri_complete":"…","expires_in":…}   then the command ends
```

The login is the device authorization grant (RFC 8628), and it is the only login there is. It behaves in one
of two ways, decided by `--issuer` alone:

| | `--issuer https://identity.diverge.network` (exactly) | Any other issuer |
|---|---|---|
| Who approves the code | The command itself, as the account that owns the subject | A person, at the issuer |
| When the command ends | When the identity is signed in (a couple of seconds) | As soon as the code is printed |
| What it prints | `{"issuer","subject"}`, the identity signed in | The code and where to approve it |
| How to know it worked | The printed line; an error otherwise | `diverge identities list`, later |
| What it needs first | The subject's account signed in under this root | Nothing |

| Flag | Meaning |
|---|---|
| `--issuer` (required) | The issuer, exactly as its discovery document names it. `https`, or `http` on loopback. It must offer a device authorization endpoint. |
| `--subject` (required) | The subject to sign in as. An approval as any other subject writes nothing. |
| `--client-id` (required) | The public client to sign in as. At `identity.diverge.network` it is `diverge`. |
| `--scope` | Space-separated; `openid` and `offline_access` are added if missing. Absent, those two. |
| `--resource` | RFC 8707 `resource` for the access token. Rarely needed. |

At `https://identity.diverge.network`:

- **The issuer string must match exactly.** A trailing slash, or any other address, is treated as another
  issuer and takes the code-and-background path.
- **The subject must be `<issuer>.<id>.<name>`.** It is split at its first two periods. Anything else is
  refused before anything is contacted, with the form it should have.
- **The account `<issuer>.<id>` must be signed in under this root.** If it is not, the error says so and gives
  the fix: `diverge identity.diverge.network accounts login <issuer>`, then run the login again.
- **A new subject and an existing one are the same command.** Signing in under a name never used before is how
  a subject begins. Signing in again under a used name adds another sign-in for the same subject.
- The name must be 1 to 64 of `a-z`, `0-9`, `-`, `_`, or the issuer refuses the approval.

At any other issuer:

- **The command ends as soon as the code is printed.** It does not wait for the approval. A background
  process of the CLI keeps polling the issuer until the code is approved, denied or expires, and prints
  nothing.
- **Check the outcome with `diverge identities list`.** The identity appears once the login was approved as
  the subject named. If the code was denied or expired, or approved as another subject, nothing is written
  and nothing is reported: run `login` again.
- No browser is opened. Approve the printed code at the issuer.

At every issuer:

- A login replaces an identity already there, signed out or not. Logging in again is how a signed-out identity
  is repaired.
- A second `login` of the same identity while one is still waiting waits its turn behind the first.
- The issuer must be reachable (its discovery document is fetched).
- An identity whose refresh token was ended, or went unused past the issuer's refresh-token lifetime (30 days
  at `identity.diverge.network`), is *signed out*: its file stays, handshakes fail, and a new login fixes it.

### list, remove

```bash
diverge identities list
# {"issuer":"https://identity.diverge.network","subject":"github.12345.admin"}      one line per identity; signed-out ones included
diverge identities remove --issuer https://identity.diverge.network --subject github.12345.old
# {"removed":{"issuer":"…","subject":"github.12345.old"}}
```

Removing the identity a running daemon or provider presents (its `identity`) breaks every later handshake
of that program. Removing one that `cli.yaml` or `viewer.yaml` names breaks that account.

## Identities on the daemon's host: `diverge run identities`

The same store, on the host of the daemon you dial — for when the daemon runs somewhere you have no shell.
These are requests to the daemon: they need an account with grants over `identities` (an admin has them), take
the usual `run` flags and `relay`, and answer `"forbidden"` otherwise.

```bash
diverge run identities list                                   # {"listed":[{"issuer":…,"subject":…,"tags":[…]}, …]}
diverge run identities list --issuer https://identity.diverge.network   # also --subject; body: issuers, subjects, all_tags, any_tags
diverge run identities tag   '{"identity":{"issuer":"https://identity.diverge.network","subject":"github.12345.daemon-a"},"tags":["prod"]}'   # "tagged" | "not_found"
diverge run identities untag '{"identity":{"issuer":"https://identity.diverge.network","subject":"github.12345.daemon-a"},"tags":["prod"]}'   # "untagged" | "not_found"
diverge run identities login --issuer https://identity.diverge.network --subject github.12345.daemon-a --client-id diverge
# {"signed_in":{"issuer":"https://identity.diverge.network","subject":"github.12345.daemon-a"}}
```

- `list` is one snapshot (`{"listed":[…]}`), not a stream; there is no `--watch`. Tags exist only so grants can
  reach some identities and not others (skill `diverge-access`).
- `login` takes `--issuer --subject --client-id [--scope --resource]`, or the same members in a BODY
  (`issuer`, `subject`, `client_id`, `scope`, `resource`). The daemon starts a device login on its own host.
  What happens next depends on the issuer, exactly as for `diverge identities login` above.
- **At `https://identity.diverge.network`** the command approves the code itself and waits. It prints
  `{"signed_in":{…}}`, or why not. The daemon's host does the sign-in, but the approval uses the account
  signed in under **the CLI's own root**, on the machine where you run the command. The same rules apply:
  exact issuer string, subject `<issuer>.<id>.<name>`, account signed in here first.
- **At any other issuer** the command prints `{"pending":{"user_code",…}}` and ends. A background process of
  the CLI keeps the login alive until the code is approved, denied or expires. Check the outcome with
  `diverge run identities list`; it is never printed.
- Where the login could not begin, either way: `"held"` (another login or reader holds that identity's lock
  right now; try again), `"forbidden"`, or `{"error":…}` (issuer unreachable, or it has no device
  authorization endpoint).
- Signing an identity in does not make the daemon present it: which identity the daemon presents is
  still `daemon.yaml` `identity`, read at start.

## Start and stop: `diverge daemon|provider start|stop`

```bash
diverge provider start > provider-start.out 2>&1; cat provider-start.out   # started provider, process <pid>
diverge daemon start   > daemon-start.out   2>&1; cat daemon-start.out     # started daemon, process <pid>

diverge daemon stop      # stopped daemon      (or: daemon is not running — exit 0)
diverge provider stop
```

**Never pipe a `start`** (no `|`, no `$(…)`): the detached server inherits the pipe and the shell hangs.
Redirect to a file, as above.

`start`:

- Runs the program's binary from the directory the `diverge` executable is in, detached, with `--config <root>`.
  It returns once the program holds the root's lock.
- Refuses when one already runs on this root: `the daemon is already running on this root, as process <pid>`.
- If the program dies first, the error carries its whole log. Usual causes: the port is taken, a document it
  refuses, a root it cannot use, and for the provider its container runtime or `ssh` missing from `PATH`. The same text is in `<root>/<program>.log`.
- The log file is truncated at every start. A program that starts well prints nothing.
- The provider's first start on Windows or macOS creates and boots the virtual machine its containers run in, and can take minutes.

`stop`:

- Asks the program to stop over its own port from this host, and returns once the lock is free — which is the
  whole teardown done. A daemon stop ends every container run and its Postgres; a **provider stop removes every
  container it ran**.
- `daemon stop` finds the port from **`cli.yaml`: the address of the default daemon**, not from `daemon.yaml`.
  That address must be on this host (`127.0.0.1`, `localhost`, `::1`), else `… is not this host`. So each root's
  `cli.yaml` default must point at that root's own daemon, and must be updated when `daemon.yaml` `port`
  changes. `provider stop` reads the port from its own document.
- `nothing is listening on port …: the program is not running, or is still starting` means the lock is held but
  the port is not open yet (or `cli.yaml` names the wrong port). Wait and retry, or fix the address.
- A stop is honoured only from the same host; there is no remote stop.

Order. Start: provider, then daemon (the daemon connects to the provider; a daemon
redials its providers every 5 seconds, so a late provider is picked up). Stop in reverse: daemon, then
provider. With several roots, stop every daemon before the provider they share.

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
| `provider.yaml` the runtime block's `storage_path` | Provider restart onto a fresh, empty container store (or a new machine): the cache of pulled images starts empty. |
| `cli.yaml` | The next CLI command. Nothing to restart. |
| `viewer.yaml` | The next start of the viewer app. |
| `diverge identities login` of an identity a running program already uses | No restart: programs read the identity's file whenever they need a token. |
| A program's `identity` changed, or a *new* entry in `admins` or `incoming` | Restart that program. Its own `identity` must be signed in under its root first. |
| Records in the daemon (providers, accounts, roles, agents, …) | Immediate; those are `diverge run` requests, not configuration. |

Before restarting anything, say what it will interrupt and make sure the user asked for it.

## Recipes

**Add an account to the CLI and make it the default.**

```bash
diverge cli config patch '[
  {op: add, path: /daemons/local/accounts/me, value: {issuer: "https://identity.diverge.network", subject: github.12345.admin}},
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
- What a daemon or provider does when the identity its `identity` names is not signed in, or is signed
  out — whether it refuses to start or starts and fails each handshake — was not traced. Either
  way the fix is a login under that root (and a restart if the program did not start).
- Whether a `remote` Postgres needs anything prepared in it before the daemon's first start was not checked.

## When this task is done

Load the `diverge` skill again before starting the next Diverge task: it is the index, and it says which
skill the next task needs.
