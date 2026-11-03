//! Asking a provider which connected identities are discoverable.
//!
//! One question, one answer, shaped as [`info`](super::info): a client
//! asks, a provider answers with every identity connected to it whose
//! [discover flag](crate::wire::frame::auth::Auth) was `1`, and the
//! scope finishes.
//!
//! # The flag is the peer's, and the handshake carries it
//!
//! Each end of a connection sends the flag in its authorization frame.
//! A provider honors the one it receives — whichever end dialled — and
//! lists that identity here for as long as that connection lasts. An
//! identity connected with the flag `0` is never listed, and an
//! identity that is not connected is never listed.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and a provider answers, so the answer
//! is in [`server`].

pub mod client;
pub mod server;
