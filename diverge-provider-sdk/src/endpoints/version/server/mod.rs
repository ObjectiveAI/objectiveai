//! The server side of a version request: what a provider sends.
//!
//! [`response`] is the whole of it. A provider answers and is done; it
//! opens no channels of its own for a question this small, so there is
//! no `request` here the way there is on the agentic loop's server
//! side.
//!
//! # And, behind the `server` feature, a way to answer it
//!
//! [`handle`] performs the exchange rather than describing it: hand it
//! the [`ScopeHandle`](crate::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::server::session::Session) yielded and it answers.
//!
//! It takes nothing else, which no other handler can say. The version
//! is this crate's own, read from the manifest at compile time, so
//! there is no provider state to consult and nothing to pass in.

pub mod response;

#[cfg(feature = "server")]
pub mod handle;
