//! The server side of a volume write: what a provider sends.
//!
//! [`response`] is what comes back on channel `0`: that the file
//! landed, or why not. [`channel_request`] is the one channel the
//! provider opens, for the content.
//!
//! There is no `channel_response`, because a client opens no channel
//! on a write for the provider to answer.
//!
//! # And, behind the `server` feature, a way to answer it
//!
//! [`handle`] performs the exchange rather than describing it: hand it
//! the [`ScopeHandle`](crate::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::server::session::Session) yielded, whose caller
//! it is, and a
//! [`VolumeManager`](crate::server::volume_manager::VolumeManager), and
//! it collects the content and puts the file in place.

pub mod channel_request;
pub mod response;

#[cfg(feature = "server")]
pub mod handle;
