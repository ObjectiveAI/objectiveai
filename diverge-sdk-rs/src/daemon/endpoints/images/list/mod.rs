//! Listing the images in the daemon's Stores, and keeping the list. A
//! client asks for the images, narrowed, and the daemon sends every
//! image it owns in its Store on every provider the caller's `list`
//! grants reach that the request's filter lets through — by provider,
//! by digest, by when it came to be owned — one response each,
//! provider by provider in the order the daemon knows them and within
//! a provider by digest, then the word that the list is whole, and
//! keeps the scope open: each image added, changed or removed, as the
//! providers' listings and the connections change, until the client
//! cancels, the one channel it opens on the scope.
//!
//! The rules:
//!
//! 1. **The daemon's Stores.** An image listed is one the DAEMON owns
//!    on a provider: built there by an
//!    [`images::build`](super::build), or pushed there by an
//!    [`images::push`](super::push). The Store is the daemon's, not an
//!    account's: every account whose grant reaches the provider is
//!    listed the same images there.
//! 2. **Each as the provider lists it**, with the provider it is on:
//!    its digest, its description, its size, when the daemon came to
//!    own it. Only the description of an image changes.
//! 3. **Read from what the daemon holds.** The daemon holds one
//!    listing of its Store on each connected provider for the
//!    connection's life and reads from that, so a provider on record
//!    that is not connected now, that has no Store, or that has not
//!    yet said its listing is whole, contributes nothing until it
//!    does.
//! 4. **A count** keeps the list to the first that many that match. A
//!    request that says nothing is every image the grants reach. A
//!    caller whose grants reach none that matches is told the list is
//!    whole at once, and watched.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
