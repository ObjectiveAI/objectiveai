//! The server side of a volume listing: what a provider sends.
//!
//! [`response`] is the whole of it: a volume added, changed or
//! removed, the word that the listing is whole, or the error. A
//! provider opens no channel of its own on a listing; the one channel
//! on it is the caller's stop.
//!
//! # And a way to answer it
//!
//! [`handle`] performs the exchange rather than describing it: hand it
//! the [`ScopeHandle`](crate::wire::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::wire::server::session::Session) yielded, whose caller
//! it is, and a
//! [`VolumeManager`](crate::provider::server::volume_manager::VolumeManager)
//! and the provider's
//! [`VolumeChanges`](crate::provider::server::volume_changes::VolumeChanges),
//! and it answers with the volumes, and keeps answering until the
//! caller stops.
//!
//! It is the mirror of [`execute`](super::client::execute) on the other
//! side, and it is gated the same way and for the same reason: this
//! module is types only unless somebody asked for the half of the crate
//! that can hold a socket.

pub mod response;

pub mod handle;
