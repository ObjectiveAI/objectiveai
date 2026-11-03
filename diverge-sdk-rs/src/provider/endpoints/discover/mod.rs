//! Listing a provider's discoverable connected identities, and keeping
//! the listing.
//!
//! One question with no parameters, and a stream kept open, shaped as
//! [`images::list`](super::images::list): every identity connected to
//! the provider whose [discover flag](crate::wire::frame::auth::Auth)
//! was `1`, the word that the listing is whole, then every such
//! identity that connects or disconnects after, until the caller
//! stops.
//!
//! # The flag is the peer's, and the handshake carries it
//!
//! Each end of a connection sends the flag in its authorization frame.
//! A provider honors the one it receives — whichever end dialled — and
//! lists that identity for exactly as long as that connection lasts.
//! An identity connected with the flag `0` is never listed, and an
//! identity that is not connected is never listed.
//!
//! Split by who SENDS, as everywhere else: the question and the stop
//! are in [`client`], the answer in [`server`].

pub mod client;
pub mod server;
