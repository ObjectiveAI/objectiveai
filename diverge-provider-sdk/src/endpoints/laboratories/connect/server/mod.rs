//! The server side of a connection: what a provider sends.
//!
//! [`channel_request`] is the one thing it asks a connector for, and
//! only ever in answer to a write the connector started.
//! [`channel_response`] answers the channels the connector opens into
//! the container. [`response`] is what comes back on channel `0`.

//!
//! # And, behind the `server` feature, [`handle`]
//!
//! It resolves the laboratory, relays the connector's authorization to
//! whoever runs it, and serves the connector against a container it
//! does not own — the one handler that deploys nothing and stops
//! nothing.

pub mod channel_request;
pub mod channel_response;
pub mod response;

#[cfg(feature = "server")]
pub mod handle;
