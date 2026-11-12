---
{
  "name": "identity-diverge-network",
  "schema": {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "title": "Arguments",
    "description": "The arguments an identity.diverge.network tool is made with.\n\nNone: the tool is the same for every caller, and the accounts it\nkeeps are told apart by the template the tool was made from. `{}` is\nthe one value; any member is refused, so a misspelled setting is\nnever silently ignored.",
    "type": "object",
    "additionalProperties": false
  }
}
---
# identity.diverge.network

Signs identities in on the daemon, at Diverge's own issuer,
`https://identity.diverge.network`, with no person involved.

Every subject at that issuer is `<issuer>.<id>.<name>`: `github` or
`google`, the owner's user id there, and a name the owner chose. An
**account** is `<issuer>.<id>`. The tool keeps accounts — the GitHub or
Google sign-ins themselves — in its database, and uses one to approve
a login the daemon begins. So an agent that may call this tool can
put any subject of any account the tool holds onto the daemon's host.

Two tools:

| tool | parameters | does |
|---|---|---|
| `list_accounts` | none | the accounts the tool holds, each as `{"account","issuer","id"}` |
| `login` | `issuer`, `id`, `subject` | signs the subject `<issuer>.<id>.<subject>` in on the daemon's host, as that account, and waits for it |

- `issuer` is `github` or `google`: who issued the account's own
  sign-in, and never `identity.diverge.network` itself.
- `id` is the account's user id at that issuer, as `list_accounts`
  gives it.
- `subject` is the name alone, the third part: 1 to 64 of `a-z`, `0-9`,
  `-` and `_`. The whole subject is built from the three.
- A subject never signed in before and one that already is are the
  same call: a login under a new name is how a subject begins.
- `login` answers `{"signed_in": {"issuer": "https://identity.diverge.network", "subject": "<issuer>.<id>.<subject>"}}`.
  The identity is then on the daemon's host. Which identity the daemon
  presents is still its own configuration's.
- No tool answers a token.

**There is no way to add an account yet.** Signing a GitHub or Google
account in to the tool will be done at the tool's own page, which is
not built. Until it is, `list_accounts` answers none and `login` has
nothing to sign in as.

## Running it

A tool container, standalone or a dependency.

- **Arguments:** none; `{}`.
- **Secrets:** none. The accounts are kept in the tool's database, not
  in the vault.
- **Database:** the tool's own, through the proxy. One table,
  `identity_accounts`, each row an account and its issuer's tokens,
  under the template the tool was made from.
- **Network:** outbound HTTPS to `identity.diverge.network`, and to
  `github.com` or `oauth2.googleapis.com` when an account's token is
  renewed.
- **Permissions: the tool needs permission to log in.** It declares
  its own, at its registration, and they must be granted for `login`
  to work. They are the `login` that brings an identity onto the
  host, and the `login` that replaces one already there:

  ```json
  [
    { "identities": ["login"] },
    { "identities": { "actions": ["login"], "within": "any" } }
  ]
  ```

  Nothing else: the tool lists no identity, tags none, and asks the
  daemon for nothing but the login. Without the first, a login of a
  subject not yet on the host answers that it is forbidden; without
  the second, one already there does.
