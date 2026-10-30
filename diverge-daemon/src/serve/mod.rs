//! The daemon, running: the port bound, every connection read, every
//! request answered, and the stop.
//!
//! The SDK's frame-level server takes a finished WebSocket and yields
//! the scopes a client opens on it; it neither listens nor decides
//! what a request is. So this module owns the socket and the
//! dispatch. [`run`] starts the database the configuration names,
//! opens the store on it, builds the one [`Daemon`](crate::daemon::Daemon),
//! listens, waits for Ctrl-C or SIGTERM — or for [`stop`], a loopback
//! `POST /stop` on the same port, the one plain HTTP request the
//! daemon answers — drains, and stops the database it started.
//! [`listen`] accepts a WebSocket upgrade at any other
//! path on the configured port, on every interface; [`connection`] is
//! one accepted socket for its life: the
//! [`handshake`](diverge_sdk::wire::handshake) that must come first,
//! the identity it establishes
//! [`admit`](crate::judge::DaemonAuthenticator::admit)ted, and then
//! every request,
//! read as a [`ClientRequest`](diverge_sdk::daemon::endpoints::ClientRequest)
//! and handed by [`dispatch`] to its handler — [`accounts`],
//! [`roles`], [`providers`], [`agents`], [`standalone_tools`],
//! [`dependency_tools`],
//! [`volumes`], [`vault`], [`postgres`] and [`events`] have one per request, every request
//! of the wire served — or, when the request carries `relay`, by
//! [`relay`], this daemon as a conduit to another; what the file
//! operations of every family share is [`files`]. An identity admitted
//! as a provider rather than a client hands the socket to
//! [`providers`](crate::providers) instead, where the daemon is the
//! caller. A connection carried
//! inside the process — a container's `/daemon` pair, or another
//! daemon come through a provider — is served by [`serve_pair`], the
//! same session and dispatch, for a standing admitted elsewhere.
//! [`reply()`] is how every handler sends a frame and how a store
//! failure becomes the wire's error. [`Error`] is why the daemon could not start or could
//! not listen, the one report a failed start gets; the daemon prints
//! nothing else.
//!
//! # Every handler, the same way
//!
//! A handler takes the scope, its decoded request, the [`Who`](crate::judge::Who)
//! the connection is served for, and the daemon; begins a
//! transaction; reads the account's [`Standing`](crate::judge::Standing);
//! judges; loads what the request names; judges again over it; acts;
//! commits on the answer that says it was done; and sends exactly the
//! answers the wire states, then the finish. A store failure on any
//! path is the endpoint's `Error`, and the transaction dropped is the
//! rollback.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod connection;
mod dispatch;
mod error;
mod listen;
pub mod logs;
mod pair;
pub mod reply;
mod run;
mod stop_route;

pub use connection::*;
pub use dispatch::*;
pub use error::*;
pub use listen::*;
pub use pair::*;
pub use reply::*;
pub use run::*;
pub use stop_route::*;

pub mod accounts;
pub mod agents;
pub mod dependency_tools;
pub mod events;
pub mod images;
pub mod info;
pub mod files;
pub mod inner;
pub mod postgres;
pub mod providers;
pub mod relay;
pub mod roles;
pub mod stream;
pub mod standalone_tools;
pub mod vault;
pub mod volumes;
