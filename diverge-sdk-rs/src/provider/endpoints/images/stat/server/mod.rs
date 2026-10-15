//! The server side of an image stat: what a provider sends.
//!
//! [`response`] is the whole of it. A provider looks the image up, answers, and is done; it opens no channel of its own for a question this small.
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
