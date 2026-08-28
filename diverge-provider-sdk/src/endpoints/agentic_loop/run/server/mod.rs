//! The server side of the agentic loop: what a provider sends.
//!
//! [`channel_request`] is what it opens channels of its own to ask
//! for. [`response`] is its answer to the client, on channel `0`.
//! [`channel_response`] answers the one channel a client opens — the
//! enqueue, with the message's fate. All of it happens inside the
//! scope the client's request opened — a server never initiates one.
//!
//! There is no `request` here. Only a client opens a scope.
//!
//! # And, behind the `server` feature, a way to answer it
//!
//! [`handle`] performs the exchange rather than describing it: hand it
//! the [`ScopeHandle`](crate::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::server::session::Session) yielded and a
//! [`ContainerDeployer`](crate::server::container_deployer::ContainerDeployer),
//! and it runs the agent and relays both directions.
//!
//! It is the only handler that writes on both of the modules above:
//! [`response`] carries the chunks down, and [`channel_request`] takes
//! the agent's tool calls out.

pub mod channel_request;
pub mod channel_response;
pub mod response;

#[cfg(feature = "server")]
pub mod handle;
