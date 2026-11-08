//! The server side of an oci scope: what a provider sends.
//!
//! [`response`] is the one frame it sends on the main stream, first:
//! the answer to whether it can supply the image. After a held,
//! nothing more comes on channel `0` until the finish; after anything
//! else, the finish follows at once. There is no `channel_request`:
//! the provider opens no channel on an oci scope. [`channel_response`]
//! is the three answers, one module per kind of channel the caller
//! opens, each the run's own answer type.
//!
//! # And a way to answer it
//!
//! The provider crate's `handle` performs the exchange rather than
//! describing it: hand it the
//! [`ScopeHandle`](crate::wire::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::wire::server::session::Session) yielded, whose
//! caller it is, and the provider's image manager, and it answers the
//! question, then every ask until the caller cancels. It is the
//! mirror of [`execute`](super::client::execute) on the other side.

pub mod channel_response;
pub mod response;
