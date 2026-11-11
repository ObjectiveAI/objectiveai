//! Pushing an image into the caller's Store, without building it.
//!
//! A caller names an image — its digest, and the references under
//! which its bytes may be fetched — and a description if it has one;
//! the provider takes the image, keeps its own copy, and the caller
//! owns it in its Store from then on, as it owns one it built.
//!
//! # Where the image comes from, in order
//!
//! 1. **Already owned.** A digest the caller owns in its Store is not
//!    pushed again: `Exists`, and nothing moves.
//! 2. **A registry the provider uses.** A reference naming a registry
//!    its policy lists, which serves the digest: the provider fetches
//!    it from there, verifies every byte against its digest, and the
//!    answer's source is
//!    [`Registry`](crate::shared::containers::response::Source::Registry).
//!    No nonce check: the caller supplied nothing.
//! 3. **The caller.** Otherwise the provider asks the caller on this
//!    scope's channels, exactly as a run does: the manifest by digest,
//!    then every blob it does not already hold, each verified against
//!    its digest, with a `Blob` progress answer per blob pulled; then,
//!    always — whether or not it pulled any byte — the nonce check of
//!    [`oci::prove`](crate::shared::containers::oci::prove). A pass is
//!    the image owned, with the source
//!    [`Client`](crate::shared::containers::response::Source::Client);
//!    a failure is `ProofFailed`, and the provider removes what the
//!    attempt brought in.
//! 4. **Otherwise** the image is `Unavailable`.
//!
//! Capacity is checked before any byte moves, from the sizes the
//! manifest declares; a push the Store has no room for is
//! `InsufficientCapacity`, with nothing moved.
//!
//! Split by who SENDS, as everywhere else: the request, the answers to
//! the provider's asks and the executor are in [`client`]; the asks
//! and the answers in [`server`].

pub mod client;
pub mod server;
