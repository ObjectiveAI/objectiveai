//! The server side of a watch: what a provider sends.
//!
//! [`response`] is the whole of it — a stream, for as long as the
//! scope lives.
//!
//! There is no `channel_request`. A provider opens no channels of its
//! own to answer a watch; it has everything it needs from the name.
//!
//! And no `channel_response`, though a caller does open one — the
//! [`channel_request`](super::client::channel_request) that ends a
//! watch is not answered on its own channel. What answers it is the
//! scope's finish, which is [`response`]'s.
//!
//! # And, behind the `server` feature, a way to answer it
//!
//! [`handle`] performs the exchange rather than describing it: hand it
//! the [`ScopeHandle`](crate::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::server::session::Session) yielded, whose caller
//! it is, and a
//! [`VolumeManager`](crate::server::volume_manager::VolumeManager), and
//! it
//! reports the tree until somebody stops.
//!
//! It is the mirror of [`execute`](super::client::execute) on the other
//! side, and it is gated the same way and for the same reason: this
//! module is types only unless somebody asked for the half of the crate
//! that can hold a socket.

pub mod response;

#[cfg(feature = "server")]
pub mod handle;
