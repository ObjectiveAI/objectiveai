//! The proof over an image in the caller's Store, computed by the
//! provider that holds it.
//!
//! A caller names an image it owns by its digest, the platform when
//! that digest names an index, and a nonce; the provider answers
//! `SHA-256(nonce ‖ D)` over its own copy of the image, exactly as
//! [`oci::prove`](crate::shared::containers::oci::prove) defines `D`.
//!
//! # What it is for
//!
//! A caller whose image is in the Store of one provider, asked by
//! another provider to [`prove`](crate::shared::containers::oci::prove)
//! that it holds it, relays the nonce here and the 32 bytes back. No
//! byte of the image moves, and the caller computes nothing over it:
//! it need hold no image, and no tool to read one.
//!
//! # The rules
//!
//! 1. The proof is computed over the provider's own stored bytes of
//!    the caller's own image, with the same function its own nonce
//!    checks use.
//! 2. A digest the caller does not own, or a platform that is not in
//!    the index it names, is
//!    [`NotFound`](server::response::Frame::NotFound): the caller
//!    cannot learn the proof of an image it does not own.
//! 3. A provider with no Store answers
//!    [`Unsupported`](server::response::Frame::Unsupported).
//! 4. Exactly one answer, then the finish.
//!
//! Split by who SENDS, as everywhere else: the ask is in [`client`],
//! the answer in [`server`].

pub mod client;
pub mod server;
