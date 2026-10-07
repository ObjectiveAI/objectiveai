//! The daemon, running: the port bound, every connection read, every
//! request answered, and the stop.
//!
//! The SDK's frame-level server takes a finished WebSocket and yields
//! the scopes a client opens on it; it neither listens nor decides
//! what a request is. So this module owns the socket and the
//! dispatch. [`listen`] accepts a WebSocket upgrade at any path on
//! the configured port, on every interface; [`connection`] is one
//! accepted socket for its life: the credential that must come
//! first, [`judge`]d, and then every request, read as a
//! [`ClientRequest`](diverge_sdk::daemon::endpoints::ClientRequest)
//! and handed to [`refuse`], which answers it with its endpoint's own
//! error and finishes the scope. [`run`] binds, listens, waits for
//! Ctrl-C or SIGTERM, and drains. [`Error`] is why the daemon could
//! not start or could not listen, the one report a failed start gets;
//! the daemon prints nothing else.
//!
//! # Every request is refused, for now
//!
//! The daemon holds no records yet, so it admits no credential and
//! serves no request. [`judge`] is where admission will be decided
//! and refuses everyone today; [`refuse`] is the one answer there is,
//! and is where each endpoint's real handler will take its arm from.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod connection;
mod error;
mod judge;
mod listen;
mod refuse;
mod run;

pub use connection::*;
pub use error::*;
pub use judge::*;
pub use listen::*;
pub use refuse::*;
pub use run::*;
