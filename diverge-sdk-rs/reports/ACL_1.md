# ACL, draft 1: accounts, roles, and what they reach

A first draft of a real access-control system for the daemon, to
replace the per-container `daemon_tools` members on the create and the
edit. The organizing idea you gave: clients of the daemon have
ACCOUNTS, and accounts differ in what they may do. Everything below
follows from making the account the only principal there is.

## 1. Principals are accounts

An ACCOUNT is the one kind of principal. Every request the daemon
serves is served for an account, and every container that calls the
daemon's own tools calls them as an account.

- **A client account** is what a connection authenticates as. The
  daemon judges a connecting client the way it judges an incoming
  provider: a key that names an account, or a hook that judges the
  credential and names one. The connection's identity string today IS
  the account; the change is that the account is a thing the daemon
  holds, with roles, rather than a bare string.
- **A container acts as an account.** An agent's or a tool's create
  names the account the container acts as, `account`, in place of the
  forty-eight tool members. The daemon knows which container is
  calling from the run scope, looks up the account the container was
  given, and judges the call as that account. A container given no
  account can call none of the daemon's tools.
- **Who may hand out which account** is a permission like any other:
  an account may give a container only an account it holds the
  `assign` action over. That is the delegation rule, and it is what
  closes the escalation hole a free-form reach left open.
- The `Creator` chain and `sender` name an account where they named a
  client, plus the agent or tool that acted, so a log still says both
  who and through what.

## 2. Resources are what the endpoints act on

Every thing the daemon holds is a resource of one of these kinds,
each with the once-and-for-all identity it already has:

| kind | identity | the actions over it |
|---|---|---|
| agent | template + index | get, delete, edit, message, logs, tag, untag |
| tool | template + index, or provider + id | get, edit, delete, attach, detach, tag, untag |
| agent template, tool template | hash | get, delete, tag, untag, and `create_from` |
| route | path | set, delete, list |
| resource | hash | list, delete, and `read`, `write` for transfer |
| provider, outgoing | address | get, edit, delete, `connect_through`, `list_for` |
| judge, incoming | identity or hook | get, edit, delete |
| account | name | get, edit, delete, `assign`, `grant`, `revoke` |
| role | name | get, edit, delete, `grant` |

Every resource has an OWNER, the account it was made under, and the
owner's view of "the caller's agents" becomes "the agents this account
may act on", which is a superset once roles cross accounts. Listing is
not an action over a kind; a list is the `get` action applied to every
resource that passes the filter, so there is nothing to grant for a
list that `get` does not already say.

Creating is the odd one. A create has no resource yet, so its subject
is what it is made FROM: `create_from` over templates for agents and
tools, `upload` over nothing for resources and templates, `add` over
nothing for providers and judges. Those are the booleans of today.

## 3. Actions are the verbs

One vocabulary, not one per family. The action names are the endpoint
verbs with a few additions where a verb had two subjects:

`get`, `create_from`, `upload`, `add`, `delete`, `edit`, `message`,
`logs`, `tag`, `untag`, `attach`, `detach`, `set` (routes), `read`
and `write` (transfer), `connect_through`, `list_for`, `assign`,
`grant`, `revoke`.

`attach` is granted over the tool, with the agent it may be attached
to as a second filter, exactly as `ToolsAttach` carries two today; the
same for `detach`. `tag` and `untag` keep their `Within` of tags.
`transfer` becomes two grants, `read` over a source and `write` over a
destination, with the edge rule — the pair has to be granted, not each
end alone — kept by granting them together in one role.

## 4. Permissions are grants in roles

A GRANT is one action over a filter of one kind, the filters we have,
read as tests, plus the two forms the filters cannot say:

```json
{"action": "message", "kind": "agent", "only": {"all_tags": ["crew"]}}
{"action": "get",     "kind": "agent", "any": true}
{"action": "get",     "kind": "agent", "self": true}
```

`self` is the relation form, replacing `agents_self` and `tools_self`:
the resource is the calling container itself. `created_by_me` is the
other relation worth having, the things this account made, which is
what `creators` already filters on and `own` would have meant.

A ROLE is a named list of grants, owned by an account, editable in one
place, and given to accounts. An account's permissions are the union
of its roles' grants; there is no deny grant in this draft, so union
is the whole algebra and order never matters. Default deny: an action
no grant covers is refused.

Why roles rather than grants pinned to each account or container: a
hundred workers made from one template should hold one role, so that
tightening it is one edit, and a template's description can say "give
it the `worker` role" rather than reproduce forty-eight members.

## 5. The endpoints this adds

Two families, in the house shape, filters and lists as everywhere:

- `accounts::{create, get, list, delete, edit, tag, untag}` — a name,
  how the account authenticates (a key, or a hook, as judges are), and
  the roles it holds. `accounts::grant` and `accounts::revoke` add and
  remove a role on an account.
- `roles::{create, get, list, delete, edit, tag, untag}` — a name and
  its grants, with `edit` under the `Change` wrapper so a grant list
  can be replaced or deleted.

And three removals: the forty-eight tool members leave `Inner` and
`Edit`, replaced by `account: Option<reference::Account>` on both;
`agents_self` and `tools_self` leave the family frames, replaced by the
`self` form of a grant; `DaemonTools`'s descendants `Reach`, `Held`,
`Switch` and `Within` stay only as far as grants use them.

## 6. Evaluation

1. Identify the account: the connection's, or the container's.
2. Find the action and the resource kind from the endpoint.
3. Collect every grant of that action and kind across the account's
   roles.
4. The request passes when any grant's form passes the resource: `any`,
   `self` with the resource being the caller, `created_by_me`, or
   `only` with the filter read as a test, the program's first value
   truthy.
5. A list is step 4 applied per resource, which is the narrowing the
   lists already do; the request's own filter narrows within.
6. Nothing is retried, and every refusal is one answer, the existing
   `NotFound` or a new `Forbidden`, which is the one place this draft
   adds a response variant everywhere.

## 7. What it costs, and what it does not

It costs two families, the `Forbidden` answer, and the removal of what
report 4 just finished. It does not cost the filters, the references,
the keys, the creator or the routes: they are the resource side and
stay as they are. The template keeps its description, which now has
something short to say about permissions, a role name.

## 8. Open

- Whether a container's account is the creator's by default when none
  is named, or none, which is the safe default this draft takes.
- Whether roles may contain roles. This draft says no; flat is enough
  until it is not.
- Whether an owner has every action over what it owns without a
  grant, or whether ownership is itself a role the create gives. This
  draft says the owner holds an implicit `owner` role over its own
  things, so that a fresh account is not locked out of what it makes.
- Whether `Forbidden` and `NotFound` are told apart. Telling them
  apart leaks existence; this draft tells them apart for the
  account's own things and answers `NotFound` for things it may not
  see at all.
