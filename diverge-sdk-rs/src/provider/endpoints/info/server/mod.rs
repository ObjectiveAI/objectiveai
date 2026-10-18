//! The server side of an info request: what a provider sends.
//!
//! [`response`] is the whole of it. A provider answers and is done; it
//! opens no channels of its own for a question this small, so there is
//! no `request` here the way there is on the agentic loop's server
//! side.
//!
//! # And a way to answer it
//!
//! The provider crate's `handle` performs the exchange rather than
//! describing it: hand it the
//! [`ScopeHandle`](crate::wire::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::wire::server::session::Session) yielded, and the
//! capabilities the provider holds out, and it answers. The version
//! is that crate's own, read from the manifest at compile time; the
//! capabilities are the one piece of provider state this exchange
//! consults.

pub mod response;
