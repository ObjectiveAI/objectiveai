//! The server side of an image push: what a provider sends.
//!
//! [`channel_request`] is what the provider asks the caller on the
//! scope when it takes the image from the caller — a manifest, a
//! blob, the proof — and [`response`] is what it answers on channel
//! `0`: a blob pulled, as progress, then the image pushed, the word
//! that the caller owns it already, or a refusal.
//!
//! # And a way to answer it
//!
//! The provider crate's `handle` performs the exchange rather than
//! describing it: hand it the
//! [`ScopeHandle`](crate::wire::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::wire::server::session::Session) yielded, whose
//! caller it is, and the provider's Store, and it answers. It is the
//! mirror of [`execute`](super::client::execute) on the other side.

pub mod channel_request;
pub mod response;
