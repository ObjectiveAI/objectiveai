//! The server side of an image hash: what a provider sends.
//!
//! [`response`] is the whole of it. A provider computes the proof
//! over the image as it can supply it, answers, and is done; it opens
//! no channel of its own.
//!
//! # And a way to answer it
//!
//! The provider crate's `handle` performs the exchange rather than
//! describing it: hand it the
//! [`ScopeHandle`](crate::wire::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::wire::server::session::Session) yielded, whose
//! caller it is, and the provider's `ImageManager`, and it answers.
//! It is the mirror of [`execute`](super::client::execute) on the
//! other side.

pub mod response;
