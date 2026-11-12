---
{
  "name": "identity-diverge-network",
  "schema": {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "title": "Arguments",
    "description": "The arguments an identity.diverge.network tool is made with.\n\nNone: the tool is the same for every caller, and the accounts it\nkeeps are told apart by the template the tool was made from. `{}` is\nthe one value; any member is refused, so a misspelled setting is\nnever silently ignored.",
    "type": "object",
    "additionalProperties": false
  },
  "frontend": true
}
---
# identity.diverge.network

Signs identities in on the daemon, at Diverge's own issuer,
`https://identity.diverge.network`, with no person involved.

**This tool needs permission to log in to new identities.** It signs
identities in on the daemon's host, so the daemon must grant it the
`login` over identities: see [Permissions](#permissions). Without it,
every `login` is refused.

Every subject at that issuer is `<issuer>.<id>.<name>`: `github` or
`google`, the owner's user id there, and a name the owner chose. An
**account** is `<issuer>.<id>`. The tool keeps accounts — the GitHub or
Google sign-ins themselves — in its database, and uses one to approve
a login the daemon begins. So an agent that may call this tool can
put any subject of any account the tool holds onto the daemon's host.

## The tools

For agents, over MCP:

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
- No tool answers a token, and no tool adds an account.

## The page

For people, in the tool's frontend. It is where an account gets into
the tool, which no tool call can do:

- **Add an account.** Choose GitHub or Google. The page shows a code
  and a link to the issuer's own page; sign in there and enter the
  code, and the account appears on the page by itself.
- **See the accounts** the tool holds, each with the name its issuer
  has for it now: the GitHub username, or the Google email. The name is
  asked of the issuer each time the page loads and is kept nowhere.
- **Remove an account.** The tool forgets it. What it already signed in
  on a daemon stays signed in.

The accounts are shared: anyone the daemon lets open the page can add
or remove them, and any agent attached to the tool can log in as any of
them. The page answers no token.

## Running it

A tool container, standalone or a dependency.

- **Arguments:** none; `{}`.
- **Secrets:** none. The accounts are kept in the tool's database, not
  in the vault.
- **Database:** the tool's own, through the proxy. One table,
  `identity_accounts`, each row an account and its issuer's tokens,
  under the template the tool was made from.
- **Network:** outbound HTTPS to `identity.diverge.network`, and to
  GitHub (`github.com`, `api.github.com`) and Google
  (`oauth2.googleapis.com`, `openidconnect.googleapis.com`) to sign an
  account in, to renew its token, and to read its name.
- **Frontend:** yes; the page above.

### Permissions

**The tool needs permission to log in to new identities**, and to log
in again ones already on the host. It declares both itself, at its
registration, and the daemon must grant them for `login` to work:

```json
[
  { "identities": ["login"] },
  { "identities": { "actions": ["login"], "within": "any" } }
]
```

| grant | what it allows | without it |
|---|---|---|
| `{"identities": ["login"]}` | signing in an identity that is not on the daemon's host yet: a subject's first login there | a login of a new subject answers that it is forbidden |
| `{"identities": {"actions": ["login"], "within": "any"}}` | signing in again one that is already there | a login of a subject already on the host answers that it is forbidden |

Nothing else: the tool lists no identity, tags none, and asks the
daemon for nothing but the login. The page needs no grant of the
tool's own; who may open it is the daemon's `frontend` grant over the
tool.
