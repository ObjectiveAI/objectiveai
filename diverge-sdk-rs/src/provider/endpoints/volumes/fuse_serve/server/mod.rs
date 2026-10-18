//! The server side of a serve: what the provider sends.
//!
//! [`response`] is what comes back on channel `0`: that the volume is
//! served, or why not, and the finish when the caller stops.
//! [`channel_response`] answers each ask the caller opens, with that
//! ask's own frame from the shared vocabulary, and each tree the
//! caller opens with a filetree stream.
//!
//! There is no `channel_request`: the provider opens no channel on a
//! serve. It answers.
//!
//! # And a way to answer it
//!
//! the provider crate's `handle` performs the exchange rather than describing it: hand it
//! the [`ScopeHandle`](crate::wire::server::scope_handle::ScopeHandle) a
//! [`Session`](crate::wire::server::session::Session) yielded, whose caller
//! it is, and a
//! `VolumeManager`, and
//! it holds the volume and answers every ask, and every tree, until
//! the stop.

pub mod channel_response;
pub mod response;
