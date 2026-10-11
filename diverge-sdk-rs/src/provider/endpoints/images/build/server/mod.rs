//! The server side of an image build: what a provider sends.
//!
//! [`response`] is the whole of it. A provider builds, telling the build's output as it comes, and answers the image built, that it has no room, or an error; it opens no channel of its own.
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
