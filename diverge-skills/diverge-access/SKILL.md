---
name: diverge-access
description: Who may do what on a Diverge daemon. Covers accounts (an (issuer, subject) identity the daemon admits), administrators named in daemon.yaml, roles and grants (every grant kind, every action, the make / over / tagging shapes, `within` and tags), creators, the host's identities as the daemon lists them, and how to diagnose a `forbidden` answer or a connection the daemon closes. Load it before running any `diverge run accounts …`, `diverge run roles …` or `diverge run identities …` command, when creating an account or a role for a person, an agent host or another daemon, when writing or narrowing a grant, when tagging records so grants can reach them, when disconnecting an account, or whenever a Diverge request answers `forbidden`.
---

# Diverge access: accounts, roles, grants

A Diverge daemon serves every request for exactly one requester, and decides it by that requester's grants.
A requester is an **account** (an identity the daemon admits, holding roles), an **administrator** (an
identity `daemon.yaml` names, holding every grant), or a tool's container (holding the grants its program
declared). A **role** is a named list of **grants**; an account may do what any grant of any role it holds
allows. Nothing denies: the union is the whole rule.

All commands here are `diverge run <family> <request> '<JSON body>'`. Each prints one compact JSON line per
answer. Run `<command> request-schema` or `<command> response-schema` for exact shapes (they dial nothing).
Every body may also carry `relay` (see the `diverge-daemons` skill).

## The model in six rules

1. **An identity is `(issuer, subject)`**, written `{"issuer":"http://127.0.0.1:14981","subject":"alice"}`.
   It is what a client's OpenID Connect ID token establishes in the handshake. The daemon keeps no list of
   trusted issuers: an identity from any issuer connects once a record (account, administrator, provider) names it.
2. **An account is one identity the daemon admits**, with a description, roles and tags. One account per
   identity; the identity is the account's name and never changes. An identity that is no account, no
   administrator and no provider is refused in the handshake: the connection closes with no frame.
3. **Administrators** are the identities under `admins:` in `daemon.yaml`. An administrator is admitted
   whether or not it has an account, and every judgment allows it everything without reading a grant. A
   fresh daemon has no account, so its first accounts are made by an administrator. `admins` is read once at
   the daemon's start (see `diverge-config` to change it).
4. **Admin status is local to one daemon.** It does not travel: a request relayed to another daemon is
   served there as an *account* of that daemon, and that daemon's `admins` do not count for a daemon
   connecting to it. Being an administrator of daemon A gives nothing on daemon B.
5. **A role holds grants; an account holds roles.** A role holds no roles. An account with no role may do
   nothing (it can still connect, and `info` needs no grant). A role's edit takes effect at each holder's
   next request.
6. **Every request is judged fresh**: the account's roles are read inside the request, so a role edited or
   an account's roles changed applies at once. Editing or deleting an account also closes its connections.

## Grants: the four shapes

A grant is one JSON object with one member, named for the kind: `{"agents": …}`. The value takes one of
these shapes, told apart by form. A value that is none of them does not decode (the request fails to
parse; it is not `forbidden`).

| Shape | Form | Meaning |
|---|---|---|
| **Make** | `{"agents":["create"]}` | A bare array of the kind's making actions. Holding it is the whole test. |
| **Over** | `{"agents":{"actions":["get","message"],"within":"any"}}` | Actions on things that exist, and how far they reach. |
| **Tagging** | `{"agents":{"actions":["tag","untag"],"within":"any","tags":["crew"]}}` | `tag`/`untag`, how far, and which tags (`"any"` or a list). |
| **The one thing** | `{"events":["list"]}` | For `postgres`, `events`, `images`: a bare array; nothing to narrow. |

Mixing shapes is malformed: `tag` among `actions` without `tags`, or `create` inside an Over object, does
not decode. Write a making grant and an over grant as two separate grants.

### `within`

`within` is either the string `"any"` (every record of the kind) or the kind's own **list filter** read as a
test: a record passes when it passes every member given; a member that lists candidates passes a record
that is any one of them; `all_tags` needs every tag, `any_tags` at least one; an empty list is absent.

```json
{"agents":{"actions":["get","list","message","logs"],"within":{"all_tags":["crew"]}}}
{"agents":{"actions":["delete"],"within":{"creators":[{"type":"client","identity":{"issuer":"http://127.0.0.1:14981","subject":"alice"}}]}}}
{"standalone_tools":{"actions":["attach","detach"],"within":{"names":["notebook","courier"]}}}
```

The filter members differ per kind (`names`, `templates`, `creators`, `active`, `deployed`, tags, created
range, …). For `providers` and `providers_daemons`, `within` is only `"any"` or
`{"all_tags":[…],"any_tags":[…]}`. The full list per kind is in
[references/grants.md](references/grants.md).

### The kinds

`agents`, `agents_templates`, `standalone_tools`, `standalone_tools_templates`, `connected_standalone_tools`,
`dependency_tools`, `providers`, `providers_daemons`, `daemons`, `accounts`, `roles`, `volumes`, `vault`,
`postgres`, `events`, `images`, `identities`. There is no separate `logs`, `frontend`, `exec` or `terminal`
kind: those are *actions* inside the kinds that have them (`logs` over agents, tools and accounts;
`frontend` and `exec` over agents, standalone tools and dependency tools; a terminal is judged as `exec`).

**Every action of every kind, and which command needs it, is in
[references/grants.md](references/grants.md).** Read it before writing a role.

### Requests that need more than one grant

A request naming several things is allowed only when every one is:

- `accounts create` / `accounts edit` naming roles: `create` (or `edit`) over accounts **and** `grant` over
  each role named. An edit that changes the roles needs `grant` over every role in the resulting list, the
  ones kept included; an edit that leaves the roles as they were needs none.
- Attaching or detaching a tool and one agent: `attach`/`detach` over the tool **and** `edit` over the agent.
  With an agent template: `attach`/`detach` over the tool **and** `attach` over the template.
- `connected-standalone-tools create`: `create` over connected tools **and** `connect` over the remote
  daemon's record.
- Mounting a volume in a container's create or edit: the container's grant **and** `mount` over the volume.
- Binding a vault key in `secrets`: the container's grant **and** `use` over the key.
- A transfer: `transfer` over the source **and** `upload` over where it lands.
- Any request with `relay`: `relay` over the daemons record that is the first ID, on this daemon; the
  request itself is then judged on the remote daemon, as the account the record names.

### Lists

A `list` is judged record by record: it sends what the account's `list` grants reach (then narrowed by the
request's own filter) and nothing else. It is `forbidden` only when no grant holds `list` over the kind at
all. A caller whose grants reach nothing gets `"listed"` with no items, not `forbidden`.

## Tags, and what a creator is

- **Tags** are plain strings on a record, put on with the family's `tag` request and taken off with `untag`.
  They exist so grants (and list filters) can reach groups of records without naming them. Agents, both
  kinds of template, standalone / connected / dependency tools, providers, remote daemon records, accounts,
  roles, volumes, vault keys and the host's identities all carry tags.
- A tagging grant limits *which* tags: `{"roles":{"actions":["tag"],"within":"any","tags":["team-a"]}}`
  lets the holder put only `team-a` on roles. Putting several tags in one request needs one grant that
  covers all of them.
- Being able to tag is being able to move a record into another grant's reach. Hand out tagging grants
  with a narrow `tags` list.
- **A creator** is who made a record, carried on every list item as `creator`:
  `{"type":"client","identity":{…}}` for an account (or administrator) over an endpoint,
  `{"type":"agent","template":…,"index":…,"name":…}` for an agent, or `{"type":"tool",…}` for a tool that
  made it through the daemon. Only the direct maker is carried. Records are not owned: an account sees and
  acts on exactly what its grants reach, whoever made it. To give an account "its own things only", use
  `within: {"creators":[{"type":"client","identity":<that account>}]}`.

## Roles

```bash
# Create. `grants` may be empty (a role that allows nothing).
diverge run roles create '{"name":"operator","description":"runs the crew agents","grants":[
  {"agents":["create"]},
  {"agents":{"actions":["get","list","message","logs","edit","delete"],"within":{"all_tags":["crew"]}}},
  {"agents":{"actions":["tag","untag"],"within":"any","tags":["crew"]}},
  {"agents_templates":{"actions":["get","list"],"within":"any"}},
  {"events":["list"]},
  {"images":["info"]}]}'
# -> "created" | "exists" | "forbidden" | {"error":…}

diverge run roles get  '{"name":"operator"}'          # -> {"found":{name, description, grants, accounts, tags, created, creator}}
diverge run roles list                                 # -> {"added":{…}} per role, then "listed"
diverge run roles list '{"accounts":[{"issuer":"http://127.0.0.1:14981","subject":"alice"}]}'   # roles alice holds
diverge run roles tag   '{"name":"operator","tags":["team-a"]}'    # -> "tagged"
diverge run roles untag '{"name":"operator","tags":["team-a"]}'    # -> "untagged"
diverge run roles delete '{"name":"operator"}'         # -> "deleted" | "not_found" | "in_use" | "forbidden"
```

`roles edit` takes a JSON Patch (RFC 6902) applied to the document `{"description":…,"grants":[…]}`, whole
or not at all. The name is not editable (a new name is a new role).

```bash
# Append one grant
diverge run roles edit '{"name":"operator","patch":[
  {"op":"add","path":"/grants/-","value":{"volumes":{"actions":["get","list"],"within":"any"}}}]}'
# Replace all grants
diverge run roles edit '{"name":"operator","patch":[{"op":"replace","path":"/grants","value":[{"events":["list"]}]}]}'
# Remove the third grant, only if it is what you think it is
diverge run roles edit '{"name":"operator","patch":[
  {"op":"test","path":"/grants/2","value":{"agents":{"actions":["tag","untag"],"within":"any","tags":["crew"]}}},
  {"op":"remove","path":"/grants/2"}]}'
# -> "edited" | "not_found" | "forbidden" | {"error":…} (a patch that does not apply, or a result that is not a role)
```

Rules: `roles delete` answers `in_use` while any account holds the role (take it off the accounts first).
The daemon stores grants exactly as given and checks nothing beyond decoding them, so a grant naming a tag
or a name nothing has is accepted and simply reaches nothing. List filter members for roles: `names`,
`accounts`, `creators`, `all_tags`, `any_tags`, `created_from`, `created_to`, plus `count`. `roles list`
takes `--watch`.

## Accounts

```bash
# Create: the identity is admitted from now on. `roles` may be empty.
diverge run accounts create '{"identity":{"issuer":"http://127.0.0.1:14981","subject":"alice"},
  "description":"crew operator","roles":["operator"]}'
# -> "created" | "exists" | "no_role" | "forbidden" | {"error":…}

diverge run accounts get '{"account":{"issuer":"http://127.0.0.1:14981","subject":"alice"}}'
# -> {"found":{"identity":{…},"description":…,"roles":[…],"connected":false,"last_connected":…,"tags":[…],
#              "created":…,"creator":{…}}}   (an administrator also carries "admin":true)

diverge run accounts list                                  # every account the grants reach, then the admins
diverge run accounts list '{"roles":["operator"],"connected":true}'

# Edit: a JSON Patch over {"description":…,"roles":[…]}. The identity is not editable.
diverge run accounts edit '{"account":{"issuer":"http://127.0.0.1:14981","subject":"alice"},
  "patch":[{"op":"add","path":"/roles/-","value":"auditor"}]}'
# -> "edited" | "not_found" | "no_role" | "forbidden" | {"error":…}

diverge run accounts tag   '{"account":{"issuer":"…","subject":"alice"},"tags":["team-a"]}'   # -> "tagged"
diverge run accounts untag '{"account":{"issuer":"…","subject":"alice"},"tags":["team-a"]}'   # -> "untagged"

diverge run accounts disconnect '{"account":{"issuer":"…","subject":"alice"}}'   # -> {"disconnected":2}
diverge run accounts delete     '{"account":{"issuer":"…","subject":"alice"}}'   # -> "deleted" | "not_found" | "forbidden"
```

What to know:

- **Creating an account does not create the identity.** The subject must exist at its issuer and sign in
  there to get a token (`diverge-config`: the issuer's `identities add`, and `diverge identities login`).
  The issuer string must match the token's `iss` exactly. The daemon needs no entry for that
  issuer in `daemon.yaml`, and need not be signed in there: the account record alone admits the identity.
- To use the new account from the CLI, add it under the daemon in `cli.yaml` and pass `--account <name>`
  (`diverge-config`).
- `no_role` (a role the daemon does not have) is answered before `forbidden` (no `grant` over a role named).
- **`accounts edit` closes every connection the account holds**, so it reconnects as it now stands;
  `accounts delete` closes them too. What a deleted account created stays, keeping its creator.
- **`accounts disconnect`** closes every connection a client has as the account (direct ones, and one
  another daemon made as it through a provider), answers the count (`0` if none) without waiting, and
  changes nothing else: the account may reconnect at once. An account may disconnect itself. It also works
  on an administrator that has no account. Use it to force a reconnect or to cut a session off; to keep
  someone out, delete the account or take its roles away.
- **Administrators in lists.** `accounts list` and `get` show each `admins` identity with `"admin":true`.
  One with no account of its own has no roles, tags, `created` or `creator`, and `edit`, `tag` and `delete`
  answer `not_found` for it (there is nothing to edit). A grant's `within` reaches such an administrator
  only through `identities` or `connected`, never through a filter on roles, tags, creators or times.
- List filter members for accounts: `identities`, `roles`, `connected`, `creators`, `all_tags`, `any_tags`,
  `created_from`, `created_to`, plus `count`. `accounts list` takes `--watch` (an account's `connected`
  flipping arrives as `changed`).

## The host's identities (`diverge run identities …`)

These are the OIDC identities signed in **on the daemon's host** (files under `<root>/identities/`), as
opposed to accounts, which are identities the daemon *admits*. The daemon presents exactly one of them
(its `daemon.yaml` `identity`) on every connection it accepts and to every provider it dials, and uses others as "our account" on remote daemons (`diverge-daemons`). No
request ever answers a token.

```bash
diverge run identities list                         # -> {"listed":[{"issuer":…,"subject":…,"tags":[…]}, …]}
diverge run identities list '{"issuers":["http://127.0.0.1:14981"],"any_tags":["peer"]}'
diverge run identities tag   '{"identity":{"issuer":"…","subject":"daemon-a"},"tags":["peer"]}'   # "tagged" | "not_found"
diverge run identities untag '{"identity":{"issuer":"…","subject":"daemon-a"},"tags":["peer"]}'   # "untagged"

# Sign an identity in on the daemon's host, by password (no browser; how an agent does it):
printf '%s' "$PASSWORD" | diverge run identities login --password-stdin \
  --issuer http://127.0.0.1:14981 --subject daemon-a --client-id diverge
# -> {"signed_in":{"issuer":…,"subject":…}} | "rejected" | {"other_subject":{…}} | "held" | "forbidden" | {"error":…}

# Or by device code: prints {"pending":{"user_code":…,"verification_uri":…,"expires_in":…}} and waits
diverge run identities login --no-browser --issuer http://127.0.0.1:14981 --subject daemon-a --client-id diverge
# -> then {"signed_in":…} | "denied" | "expired" | {"other_subject":…} | {"error":…}
```

- `identities list` is a snapshot: one `listed` answer holding the array, then the end (no `--watch`).
- `login` flags stand in for body members (`--issuer`, `--subject`, `--client-id`, `--scope`, `--resource`);
  a JSON body with the same members works too. `--password-stdin` reads the password from stdin; the
  daemon's command log keeps it redacted. The built-in public client of Diverge's own issuer is `diverge`.
- A login replaces an identity already on the host. `held` means another login or a reader holds the
  identity's lock; nothing was asked of the issuer. Ending the command ends the login and writes nothing.
- Signing an identity in does not make the daemon present it: which identity the daemon presents
  stays `daemon.yaml`'s `identity`.
- Grants: `{"identities":["login"]}` for an identity not yet on the host;
  `{"identities":{"actions":["list","login"],"within":…}}` for those there (filter members `issuers`,
  `subjects`, `all_tags`, `any_tags`); the tagging shape for `tag`/`untag`.
- This is not `diverge identities login` (no `run`), which signs in on the host the CLI runs on. See
  `diverge-config`.

## Worked example: a narrow account for one team

```bash
ISS=http://127.0.0.1:14981
# 1. As an administrator: a role that reaches only what carries the tag "team-a".
diverge run roles create '{"name":"team-a","grants":[
  {"agents":["create"]},
  {"agents":{"actions":["get","list","message","logs","filetree"],"within":{"all_tags":["team-a"]}}},
  {"agents":{"actions":["get","list","edit","delete"],"within":{"creators":[{"type":"client","identity":{"issuer":"'$ISS'","subject":"alice"}}]}}},
  {"agents":{"actions":["tag"],"within":{"creators":[{"type":"client","identity":{"issuer":"'$ISS'","subject":"alice"}}]},"tags":["team-a"]}},
  {"agents_templates":{"actions":["get","list"],"within":"any"}},
  {"accounts":{"actions":["logs"],"within":{"identities":[{"issuer":"'$ISS'","subject":"alice"}]}}}]}'
# 2. The account.
diverge run accounts create '{"identity":{"issuer":"'$ISS'","subject":"alice"},"roles":["team-a"]}'
# 3. Tag the existing agents the team should see.
diverge run agents tag '{"agent":{"name":"echo"},"tags":["team-a"]}'
# 4. Check as alice (after adding her under the daemon in cli.yaml).
diverge run --account alice agents list
```

## Diagnosing a refusal

There are three different failures. Tell them apart first.

| What you see | What it is |
|---|---|
| The command fails before any answer: the daemon "closed the connection with no answer" / rejected | **Not admitted.** The identity is no account and no administrator of this daemon, or its issuer is not in the daemon's `identities`, or the token could not be had (not signed in on this root). Fix: `accounts create` (as an administrator), check `cli.yaml`'s account identity, `diverge identities list`. |
| The answer is `"forbidden"` | **Admitted, not allowed.** No grant of the account allows this action over everything the request names. |
| The answer is `"not_found"`, `"no_role"`, `"in_use"`, `{"error":…}` | The request was allowed and came to something else. Not an access problem. |

For `"forbidden"`, work through this list:

1. **Who was I?** The CLI dials as the daemon's default account in `cli.yaml` unless `--account` says
   otherwise. `diverge run accounts get '{"account":{…}}'` shows the account's `roles` and whether `admin`.
2. **What do the roles grant?** `diverge run roles list '{"accounts":[<identity>]}' --jq '.added | {name, grants}'`.
3. **Is the action held at all?** Look the command up in [references/grants.md](references/grants.md): the
   kind and the action it needs. A making action (`create`, `add`, `insert`, `login`) needs the bare-array
   grant; an Over grant listing `create` does not exist.
4. **Does `within` reach the record?** Get the record as an administrator and test the filter by hand:
   every member given must pass (tags, names, creators, `active`, …). A `within` with a tag the record does
   not carry reaches nothing. `forbidden` is also what a record that exists but is out of reach answers;
   with no grant of the action at all, `forbidden` comes before the record is even looked up.
5. **Does the request name a second thing?** See "Requests that need more than one grant": roles named in
   an account, the agent or template of an attach, the daemons record of a connected tool, a volume
   mounted, a vault key bound, a transfer's destination, the first `relay` ID.
6. **Tagging:** the grant's `tags` must cover every tag in the request, or be `"any"`.
7. **Relayed?** With `relay`, `forbidden` can come from this daemon (no `relay` grant reaching the record)
   or from the remote, which judges the request as the account the record names, by *its* roles there. A
   remote's administrators are not administrators for a relayed connection.
8. **Is the requester a tool?** A tool's container holds only what its image declared at registration, and
   is no account; an agent's container asks the daemon nothing. See `diverge-tools`.
9. **Read the command log.** The daemon keeps every request a requester put, with whether it was let
   through. As an administrator (or with `logs` over accounts):

   ```bash
   diverge run logs '{"of":{"type":"account","issuer":"http://127.0.0.1:14981","subject":"alice"}}' \
     --jq '.item.account | select(.allowed == false) | {logs_index, created, path, request}'
   ```

   `allowed:false` is exactly a `forbidden`; `path` is the request (`agents/message`), `request` its body.
   An administrator's own log is `{"type":"admin","issuer":…,"subject":…}` and only administrators read it.
   See `diverge-logs`.

Fixes, in order of preference: tag the record so an existing grant reaches it; add a narrowly scoped grant
to the role with `roles edit`; add a role to the account with `accounts edit` (this disconnects it).
Changes apply at the next request; nothing needs restarting.

## Not verified

- The whole access model is read from the SDK and daemon source and from the CLI's schemas. The read-only
  requests (`accounts list|get`, `roles list`, `identities list`, `logs`) were run against a live daemon as
  an administrator; no account, role or tag was created, edited or deleted while writing this, and no
  request was run as a non-administrator account.
- The exact error text of an unadmitted connection is the CLI's own and may differ by version.
- The SDK's module documentation mentions an `assign` action over accounts; no such action exists in the
  grant types on this branch, so it is not listed.

## When this task is done

Load the `diverge` skill again before starting the next Diverge task: it is the index, and it says which
skill the next task needs.
