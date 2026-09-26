//! The server side of a volume write: what a provider sends.
//!
//! [`response`] is what comes back on channel `0`: that the file
//! landed, or why not. [`channel_request`] is the one channel the
//! provider opens, for the content.
//!
//! There is no `channel_response`, because a client opens no channel
//! on a write for the provider to answer.
//!
//! # And a way to answer it
//!
//! [`handle`] performs the exchange rather than describing it: hand it
//! the [`ScopeHandle`](crate::wire::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::wire::server::session::Session) yielded, whose caller
//! it is, and a
//! [`VolumeManager`](crate::provider::server::volume_manager::VolumeManager), and
//! it collects the content and puts the file in place.

pub mod channel_request;
pub mod response;

pub mod handle;
