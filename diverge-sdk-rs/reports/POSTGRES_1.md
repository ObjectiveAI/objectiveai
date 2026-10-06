# Postgres, draft 1: one scope per container, on any Postgres

How the daemon gives every container its own compartment in the
database it serves — local or remote — such that the container names
its tables unqualified, never leaves its compartment, and never holds
a credential. The organizing rule: the external Postgres is asked for
as little as possible. No superuser, no extension, no change to its
configuration, no change to anything that is not the daemon's own,
and nothing newer than Postgres 10. What the daemon cannot have on
those terms, it does without.

## 1. What a scope is

A SCOPE is one Postgres login role and one schema of the same name,
owned by that role, in the one database the mode names. That is the
whole of it. The role owns nothing else and is a member of nothing;
the schema is the only place the role may create; and the role's
search path is pinned to that schema alone, so an unqualified
`CREATE TABLE foo` lands in it and an unqualified `foo` resolves in
it and nowhere else.

- **One per container.** A scope belongs to one container, named once
  and for all — an agent by template and index, a tool by template and
  index or, connected, by provider and id. The role is
  `diverge_` followed by a hash of that identity, which fits
  Postgres's 63-byte name limit, is stable across daemon restarts, and
  is never reused, because the identity is never reused.
- **Not per account, not per template.** Two containers of one
  account, or made from one template, do not share a scope. Sharing
  is what a scope exists to prevent; anything shared is a volume or a
  resource, not a table.
- **Data isolation is Postgres's.** The container's role holds
  `USAGE` and `CREATE` on its own schema by owning it, and no
  privilege on any other schema beyond what the database grants to
  `PUBLIC`. A query naming another scope's table is `permission
  denied`, as Postgres says it. The daemon enforces nothing after the
  handshake and does not need to.

## 2. What the external Postgres is asked for

Exactly two privileges on the role the URL names, and nothing on the
server:

| asked for | why | who has it |
|---|---|---|
| `CREATEROLE` | to make one login role per container, and later to drop it | every managed service's master user; any role an operator makes with `CREATEROLE` |
| `CREATE` on the database the URL names | to make one schema per container | the database's owner, or any role granted `CREATE` on it |

Not asked for, and not used:

- **Superuser.** A superuser URL works, and gains nothing: the daemon
  runs the same statements. It is accepted, not required, and not
  recommended, since any daemon bug then has the whole cluster.
- **`CREATEDB`.** A scope is a schema, not a database, precisely so
  that no database is ever created. A database per container would
  close two leaks named in §7 and would cost this privilege, a
  database per container on the remote, and a `CREATE DATABASE` that
  cannot run inside a transaction. The schema is the right trade for
  "least asked for"; the other is noted, not built.
- **Extensions.** None. `CREATE EXTENSION` of a trusted extension
  takes `CREATE` on the database, which the container's role does not
  have; untrusted ones take superuser.
- **`pg_hba.conf`, `postgresql.conf`, any server setting.** Nothing
  is read or changed. The authentication method the server offers is
  what the daemon answers, §4.
- **Ownership of `public`, or any change to it.** The daemon never
  grants or revokes on `public`. It pins each role's search path away
  from it, which is the daemon's own setting on the daemon's own role.
  What `PUBLIC` may do in `public` stays whatever the database made
  it, §7.
- **A version.** Every statement below is core grammar since Postgres
  9; the SCRAM verifier in §3 is Postgres 10, and the daemon sends a
  plain password to anything older. Nothing in the design keys on a
  version beyond that.

A set of a remote mode checks these at once, on one connection as the
URL's role: `rolcreaterole` or `rolsuper` in `pg_roles`, and
`has_database_privilege(current_database(), 'CREATE')`. A URL the
daemon cannot connect with, or one whose role lacks either, is the
set's error, in the server's words, and the mode is as it was.

## 3. Provisioning: four statements, idempotent, at first use

A scope is made the first time its container opens a connection
through the current database, not at the container's create. The
create then does not depend on the database being reachable, a mode
swap needs no migration pass, and a container that never dials costs
nothing. The daemon runs, in one transaction, as the URL's role:

```sql
CREATE ROLE diverge_<hash> LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE
    NOINHERIT NOREPLICATION NOBYPASSRLS
    PASSWORD 'SCRAM-SHA-256$<verifier>';
CREATE SCHEMA diverge_<hash> AUTHORIZATION diverge_<hash>;
ALTER ROLE diverge_<hash> SET search_path = diverge_<hash>;
```

- **Idempotent.** `CREATE ROLE` is run in a `DO` block that swallows
  `duplicate_object`, since roles are cluster-wide and a scope of this
  container may exist from an earlier daemon run; `CREATE SCHEMA` is
  `IF NOT EXISTS`; `ALTER ROLE ... SET` is a plain overwrite. Two
  connections racing to make one scope are serialized by
  `pg_advisory_xact_lock` on a key derived from the role name, so the
  second finds the first's work.
- **The password is the daemon's, in memory only.** The daemon mints
  it, 32 random bytes, when the scope is made and whenever it has no
  password for a scope in hand — after a restart, say — by
  `ALTER ROLE diverge_<hash> PASSWORD ...`, which the URL's role may
  run on a role it created. Nothing is written to disk, nothing is
  stored in a table, and nothing is ever handed to the container.
  Losing it costs one `ALTER ROLE`.
- **Sent as a verifier, not a password.** On Postgres 10 and later the
  daemon computes the SCRAM-SHA-256 verifier itself and sends that, so
  the password never crosses the admin connection in the clear
  whatever the server's `password_encryption`. Older servers get the
  password, over TLS when the URL allows it.
- **`NOINHERIT`, and no memberships.** The role is a member of
  nothing, so `SET ROLE` and `SET SESSION AUTHORIZATION` inside a
  session can reach nothing. `NOINHERIT` is belt and braces for a
  membership the operator might add by hand.
- **`search_path` alone.** The path is the scope's schema and nothing
  else — not `"$user", public`, the default, whose fall-through to
  `public` is what the old stack relied on for reading its base
  tables and what this design has no use for. A container that sets
  its own path in-session may do so and reaches only what its
  privileges reach.

A scope is made in the database the mode names at that moment. A mode
swap points at another database, where the same container's first
connection makes a fresh scope, empty. Nothing is moved; §6.

## 4. The connection: handshake by the daemon, the rest relayed

The provider carries each container connection to the daemon raw, a
pair of channels per connection, and that leg stays raw. What changes
is the daemon's end of it: the daemon cannot splice the container
straight through to the remote, because the remote will ask for a
password the container does not have. So the daemon answers the
container's handshake itself and performs its own toward the
database, and relays everything after.

- **Who the connection is for is known before a byte is read.** The
  connection arrives on the run scope of one container; the proxy's
  connection id names it; the daemon maps it to the container, and
  from the container to the scope. Nothing the container writes is
  consulted for its identity. The `diverge` user and `diverge`
  database the container's proxy makes every container send are
  placeholders, read and discarded.
- **Toward the container.** An `SSLRequest` is answered `N`: the leg
  inside the container is loopback to its own proxy and carries no
  secret. The `StartupMessage` is read for its parameters. The daemon
  answers `AuthenticationOk` — the loopback is the trust boundary,
  and there is no credential to check — and then forwards what the
  database said at its own handshake: every `ParameterStatus`, the
  `BackendKeyData`, and `ReadyForQuery`.
- **Toward the database.** The daemon dials the URL's host and port,
  negotiates TLS by the URL's `sslmode` as libpq reads it — `prefer`
  when it says nothing — and sends a `StartupMessage` of its own:
  `user` the scope's role, `database` the URL's, and the container's
  other parameters — `client_encoding`, `application_name`,
  `DateStyle`, `TimeZone`, and the rest — passed through, all but
  `user`, `database`, `replication` and `options`, which are the
  daemon's or nobody's. It answers whatever the server asks:
  `SCRAM-SHA-256`, `md5`, or cleartext `password`, the last only over
  TLS, since otherwise the scope's password would cross the network
  bare. A server asking for a method the daemon does not speak, or
  for cleartext without TLS, is a failed connection, and the
  container's socket is closed as a refused dial.
- **After `ReadyForQuery`, bytes.** Every message in either direction
  is relayed as it comes, unread: simple and extended queries, COPY
  both ways, notices, notifications, `Terminate`. The daemon parses
  nothing after the handshake, and a message larger than a frame
  spans frames as the shared `postgres` module says. A container
  session cannot become another role, §3, so there is nothing in the
  stream the daemon would need to look for.
- **Cancel.** A `CancelRequest` arrives as a fresh connection carrying
  a process id and a secret key. The daemon knows every
  `BackendKeyData` it forwarded and which container's session each
  belongs to; a request whose key is one of the SAME container's
  sessions is sent on to the database on a fresh connection, as
  Postgres expects, and any other is dropped unanswered. A container
  cancels only its own queries, which is one better than Postgres
  alone gives.
- **Ends.** The container's socket ending ends the database session,
  by a `Terminate` if the stream was at a message boundary and by
  closing the socket otherwise; the database closing ends the
  container's, and the provider's channel is finished.

The container's driver sees a Postgres that asked for no password. A
URL in the container of the form
`postgres://diverge@127.0.0.1:81/diverge` works, and so does one with
any password at all, since none is checked.

## 5. Local mode is the same thing

The local mode is the daemon's own cluster, and it is provisioned by
the same four statements through the same code path, the daemon's own
admin role standing where the URL's does. The only differences are
the daemon's: it picks the port, holds the admin password, and may
set `password_encryption = scram-sha-256` and a `pg_hba.conf` of
`scram-sha-256` on loopback, since the cluster is its own. A scope
made locally and a scope made remotely are indistinguishable to the
container.

## 6. Lifecycle

- **Made** at the container's first connection through the current
  database, §3.
- **Kept** for the container's life, across daemon restarts: the role
  and the schema are in the database, and the password is re-minted
  whenever the daemon lacks it.
- **Dropped** at the container's delete, in the current database:

  ```sql
  DROP SCHEMA IF EXISTS diverge_<hash> CASCADE;
  DROP ROLE IF EXISTS diverge_<hash>;
  ```

  The `DROP ROLE` fails when the role owns something in another
  database of the same cluster — a scope made before a mode swap to
  another database on the same cluster, say — and then the schema is
  dropped and the role left, which is harmless: it owns nothing here
  and nobody holds its password. The daemon does not reach into other
  databases.
- **Orphaned** by a mode swap. A scope in the database the daemon no
  longer points at is left exactly as it was, with its data, which is
  what "nothing carried over, the old one not touched" promises. An
  operator who wants it gone drops every `diverge_*` schema and role
  there; the daemon never does, since it no longer has that database.
- **Swapped under nothing.** A set is answered `InUse` while any
  container connection is open, as the API already states; a session
  cannot be moved, and this design does not try.

## 7. What this does and does not guarantee

Guaranteed, on any Postgres meeting §2:

- A container reads and writes its own tables and no other
  container's. The guarantee is Postgres's privilege system, with the
  container holding a role that owns one schema and is a member of
  nothing.
- A container never holds a credential to the database. There is
  nothing in it to read, leak, or reuse, and its proxy's loopback is
  the only thing it can dial.
- A container's identity toward the database is the channel it came
  in on, never a value it sent.
- Nothing the daemon does to the external database outlives the
  daemon's roles and schemas, all named `diverge_*`, and nothing it
  does touches another role's objects.

Not guaranteed, and said plainly:

- **Names leak.** Every scope's schema name and table names are
  visible in `pg_catalog` to every other scope, as they are to any
  role in a Postgres database; data is not. `pg_stat_activity` shows
  other sessions exist, with their query text hidden. A database per
  container would close this, at the cost of `CREATEDB`, §2.
- **`public` is the database's.** On Postgres 15 and later `PUBLIC`
  cannot create in `public`, so containers cannot meet there. On
  older servers `PUBLIC` can, and two containers that both name
  `public.x` explicitly share it. The daemon does not revoke this,
  since `public` is not its to change; an operator who wants it
  closed runs `REVOKE CREATE ON SCHEMA public FROM PUBLIC` once.
- **`LISTEN`/`NOTIFY` and advisory locks are database-wide.** A
  container can hear another's notifications on a channel name it
  guesses, and can hold an advisory lock another waits on. Both are a
  database-per-container matter, as above.
- **Resources are shared.** Connections, CPU, disk, temp files, and
  the lock table are the cluster's, and a container can exhaust
  them. Postgres has no per-role disk quota. `CONNECTION LIMIT`,
  `statement_timeout` and `temp_file_limit` on the role bound the
  rest, and are the daemon's to set as it sees fit; none is required
  for the design.
- **It is as strong as Postgres.** A privilege-escalation bug in the
  server breaks every multi-tenant Postgres, this one included.

## 8. Against the old stack

The old `objectiveai` compartments are the same role-plus-schema, with
three differences this design makes on purpose:

- The old stack hands the role's password into the container as a
  URL in its environment, and the database authenticates whatever
  crosses the pipe. Here the container never sees one; the daemon
  authenticates for it, from the channel's identity.
- The old stack's search path falls through to a shared base schema
  every plugin may read, over a cleartext `password` handshake. Here
  the path is the scope alone, there is no shared schema, and the
  password, when it crosses anything, is a SCRAM exchange or a
  verifier.
- The old stack never drops a compartment. Here a delete drops it.

What is kept: the role and the same-named schema, `search_path` as
the whole of "implicit", the accepted leak of names through the
catalog, and local and remote being one code path.

## 9. What the API already says, and one sentence to change

Nothing in the daemon API changes for this. `postgres::get`, `set`
and `connections`, `Mode`, `InUse` and the `postgres` grant kind are
as they are, and the `diverge` user and database the proxy constants
name stay as the placeholders they now are. One sentence in the
`postgres` family doc — that a container connection is "a splice of
bytes the daemon does not read" — becomes "relayed unread after a
handshake the daemon performs", and a paragraph states the scope as a
guarantee: one schema per container, implicit, inescapable, no
credential in the container.
