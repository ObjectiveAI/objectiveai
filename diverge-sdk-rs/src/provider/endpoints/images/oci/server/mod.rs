//! The server side of an image pull: what a provider sends.
//!
//! Nothing on the main stream — there is no `response`: the provider
//! answers every ask on the ask's own channel and sends no frame on
//! channel `0` but the finish that ends the scope. There is no
//! `channel_request` either: the provider opens no channel on a pull.
//! [`channel_response`] is the three answers, one module per kind of
//! channel the caller opens, each the run's own answer type.
//!
//! # And a way to answer it
//!
//! The provider crate's `handle` performs the exchange rather than
//! describing it: hand it the
//! [`ScopeHandle`](crate::wire::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::wire::server::session::Session) yielded, whose
//! caller it is, and the provider's `OciStore`, and it answers every
//! ask until the caller disconnects. It is the mirror of
//! [`execute`](super::client::execute) on the other side.

pub mod channel_response;
