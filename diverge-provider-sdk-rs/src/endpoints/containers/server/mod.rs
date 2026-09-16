//! What the container scopes' handlers share, behind the `server`
//! feature.
//!
//! A run and a connect differ in how they get a container — one
//! deploys, one is handed a running one — and are alike in
//! everything after: the container's proxy is spoken to over one
//! WebSocket in the frames of
//! [`container_proxy_endpoints`](crate::container_proxy_endpoints),
//! with this end as that wire's client; every channel the proxy opens
//! is carried to the caller; every channel the caller opens is served
//! against the proxy. Written once here, and each scope's `handle`
//! wraps it:
//!
//! - `held`, the volumes a run's request names, each found through
//!   the provider's `VolumeMountManager` and locked before anything else
//!   is done, and every one unlocked on every ending — the server
//!   half's whole enforcement of one container per volume.
//! - `setup`, the ORDERED preparation of a run: the content the
//!   caller mounts by identity, fetched where the store lacks it; the
//!   registry told to serve a caller-held image; the deploy; the one
//!   connection to the proxy, and on it the family's `begin` — an
//!   agent container's carrying the agent — and then one `fuse::mount`
//!   scope per mount, each complete before the next. Nothing the
//!   caller opens is read until all of it is done and the id is out.
//! - `relay`, the proxy's asks — the channels it opens on `begin`,
//!   the asks each mount makes on its scope, and the agent's chunks
//!   off the begin's main stream — each carried to the caller and its
//!   answer carried back on the proxy's own channel, every one on a
//!   task of its own.
//! - `serve`, the channels the caller opens — a tree, a read, a
//!   write, its half of a database connection, and the family's own
//!   exchange — each served against the proxy on a task of its own,
//!   read off the scope by one loop that also hears the stop, the
//!   container leaving, and the caller going away.
//! - `Run`, what those tasks share: the scope, the connection to the
//!   proxy, the begin scope on it, the tasks themselves, the database
//!   pairs in flight, and the signals that the container is gone and
//!   that the run is ending.
//! - `Family` and `Runs`, what a scope supplies to all of that: its
//!   frame types, since the three scopes' frames are byte-identical
//!   and distinct types, and how its container begins.

pub(crate) mod begin;
pub(crate) mod content;
pub(crate) mod encoded;
pub(crate) mod family;
pub(crate) mod handler;
pub(crate) mod held;
pub(crate) mod own;
pub(crate) mod pairs;
pub(crate) mod relay;
pub(crate) mod render;
pub(crate) mod run;
pub(crate) mod serve;
pub(crate) mod setup;
