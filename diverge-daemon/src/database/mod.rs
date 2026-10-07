//! The database, served into every container: one scope per
//! container, on any Postgres.
//!
//! The daemon keeps its records in one Postgres and serves that same
//! Postgres to its containers — its own, started beside it, or a
//! remote one at a URL — and asks of it as little as possible: two
//! privileges on the role the URL names, `CREATEROLE` and `CREATE` on
//! the database, and nothing on the server. No superuser, no
//! extension, no change to its configuration.
//!
//! A SCOPE is one login role and one schema of the same name, owned
//! by the role, in the one database: `diverge_` and a hash of the
//! container's once-and-for-all identity. Made at the container's
//! first connection, in one transaction under an advisory lock, by
//! statements every Postgres since 10 takes — the role with a
//! SCRAM-SHA-256 verifier, a member of nothing, able to create
//! nowhere but its schema, its search path pinned to that schema
//! alone — and dropped at the container's delete. The role's password
//! is the daemon's, minted in memory for the daemon's life and never
//! written anywhere; nothing is ever handed to the container.
//!
//! A CONNECTION a container opens arrives on its run scope as a pair
//! of channels, raw pgwire. The daemon answers the container's
//! handshake itself — an `SSLRequest` is answered `N`, the startup
//! message read for its parameters, `AuthenticationOk` sent, since the
//! loopback is the trust boundary — and performs its own toward the
//! database as the container's role: the URL dialled, TLS by its
//! `sslmode` as libpq reads it, a startup message of the daemon's with
//! the container's parameters passed through, and whatever the server
//! asks answered — SCRAM-SHA-256, md5, or cleartext over TLS alone —
//! through the `postgres-protocol` crate. What the database said at
//! its handshake, every `ParameterStatus`, the `BackendKeyData` and
//! `ReadyForQuery`, is forwarded to the container as it was; after
//! that every message either way is relayed whole and unread. A
//! `CancelRequest` is sent on for a backend key the same container's
//! session was given, and dropped for any other. Either end ending
//! ends the other.
//!
//! [`Target`] is what the daemon knows of its database; [`role_of`]
//! and [`container_of`] name a container's scope; [`provision`] is the
//! statements; [`Scope`] is the password in memory and the scope made
//! once per daemon life; [`connect`] is one connection, handshake to
//! end. Not guaranteed, as the design says: names leak through the
//! catalog, `public` is the database's, `LISTEN` and advisory locks
//! are database-wide, and resources are shared.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod scope;
mod scopes;
mod target;

pub mod connect;
pub mod provision;

pub use scope::*;
pub use scopes::*;
pub use target::*;
