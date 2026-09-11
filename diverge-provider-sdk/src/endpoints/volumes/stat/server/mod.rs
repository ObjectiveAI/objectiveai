//! The server side of a volume stat: what a provider sends.
//!
//! [`response`] is the whole of it. A provider walks the volume,
//! answers, and is done; it opens no channels of its own for a
//! question this small.
//!
//! # And, behind the `server` feature, a way to answer it
//!
//! [`handle`] performs the exchange rather than describing it: hand it
//! the [`ScopeHandle`](crate::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::server::session::Session) yielded, whose caller
//! it is, and a
//! [`VolumeManager`](crate::server::volume_manager::VolumeManager), and
//! it answers with the volume examined.
//!
//! It is the mirror of [`execute`](super::client::execute) on the other
//! side, and it is gated the same way and for the same reason: this
//! module is types only unless somebody asked for the half of the crate
//! that can hold a socket.

pub mod response;

#[cfg(feature = "server")]
pub mod handle;
