//! The server side of a volume edit: what a provider sends.
//!
//! [`response`] is the whole of it. A provider makes the change,
//! answers, and is done.
//!
//! # And, behind the `server` feature, a way to answer it
//!
//! [`handle`] performs the exchange rather than describing it: hand it
//! the [`ScopeHandle`](crate::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::server::session::Session) yielded, whose caller
//! it is, and a
//! [`VolumeManager`](crate::server::volume_manager::VolumeManager), and
//! it
//! makes the change and answers.
//!
//! It is the mirror of [`execute`](super::client::execute) on the other
//! side, and it is gated the same way and for the same reason: this
//! module is types only unless somebody asked for the half of the crate
//! that can hold a socket.

pub mod response;

#[cfg(feature = "server")]
pub mod handle;
