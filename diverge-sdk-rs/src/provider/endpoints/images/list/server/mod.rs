//! The server side of an image listing: what a provider sends.
//!
//! [`response`] is the whole of it: an image added or removed, the
//! word that the listing is whole, or the error. A provider opens no
//! channel of its own on a listing; the one channel on it is the
//! caller's stop.
//!
//! # And a way to answer it
//!
//! The provider crate's `handle` performs the exchange rather than
//! describing it: hand it the
//! [`ScopeHandle`](crate::wire::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::wire::server::session::Session) yielded, whose
//! caller it is, an `ImageManager` and the provider's `Changes`, and
//! it answers with the images, and keeps answering until the caller
//! stops. It is the mirror of [`execute`](super::client::execute) on
//! the other side.

pub mod response;
