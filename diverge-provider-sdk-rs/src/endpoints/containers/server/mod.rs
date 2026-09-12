//! What the three container scopes' handlers share, behind the
//! `server` feature.
//!
//! The mirror of [`client`](super::client): there the caller's side
//! of every container scope is written once and each `execute` wraps
//! it; here the provider's side is, and each `handle` wraps it. What
//! differs between the families is which frame types carry the same
//! bytes, which is what `Family` abstracts — a handler names its
//! frames once, and everything that serves a scope is written against
//! the trait.
//!
//! - `setup`, the ORDERED preparation of a run: the content the
//!   caller mounts by identity, fetched where the store lacks it; the
//!   registry told to serve a caller-held image; the deploy; the one
//!   `/requests` connection to the proxy. Nothing the caller opens is
//!   read until all of it is done and the id is out.
//! - `relay`, the container's asks — MCP, vault, command, fuse, a
//!   database connection — each carried to the caller on a channel
//!   this end opens and its answer carried back to the proxy's path,
//!   every one on a task of its own.
//! - `serve`, the channels the caller opens: the tree, a read, a
//!   write, its half of a database pair, and the family's own — a
//!   loop, a schema, the queue; the five MCP exchanges into a tool
//!   container — each on a task of its own, read off the scope by one
//!   loop that also hears the stop, the container leaving, and the
//!   caller going away.
//! - `Run`, what those tasks share: the scope, the client to the
//!   proxy, the tasks themselves, the database pairs in flight, and
//!   the signal that the container is gone.

pub(crate) mod content;
pub(crate) mod encoded;
pub(crate) mod family;
pub(crate) mod handler;
pub(crate) mod own;
pub(crate) mod pairs;
pub(crate) mod relay;
pub(crate) mod render;
pub(crate) mod run;
pub(crate) mod serve;
pub(crate) mod setup;
