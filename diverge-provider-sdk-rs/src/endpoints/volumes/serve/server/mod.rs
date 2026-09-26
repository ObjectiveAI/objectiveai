//! The server side of a serve: what the provider sends.
//!
//! [`response`] is what comes back on channel `0`: that the volume is
//! served, or why not, and the finish when the caller stops.
//! [`channel_response`] answers each ask the caller opens, with that
//! ask's own frame from the shared vocabulary.
//!
//! There is no `channel_request`: the provider opens no channel on a
//! serve. It answers.
//!
//! # And, behind the `server` feature, a way to answer it
//!
//! [`handle`] performs the exchange rather than describing it: hand it
//! the [`ScopeHandle`](crate::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::server::session::Session) yielded, whose caller
//! it is, and a
//! [`VolumeManager`](crate::server::volume_manager::VolumeManager), and
//! it holds the volume and answers every ask until the stop.

pub mod channel_response;
pub mod response;

#[cfg(feature = "server")]
pub mod handle;
