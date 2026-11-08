//! The proof over an image, computed by the provider that can supply
//! it.
//!
//! A caller names an image by the shared
//! [`oci::prove`](crate::shared::containers::oci::prove) request — the
//! image, the platform when its digest names an index, and a nonce —
//! and the provider answers `SHA-256(nonce ‖ D)`, exactly as
//! [`oci::prove`](crate::shared::containers::oci::prove) defines `D`.
//!
//! # What it is for
//!
//! A caller whose image one provider can supply, asked by another
//! provider to [`prove`](crate::shared::containers::oci::prove) that
//! it holds it, relays the request here and the 32 bytes back. No byte
//! of the image moves, and the caller computes nothing over it: it
//! need hold no image, and no tool to read one.
//!
//! # The rules
//!
//! 1. **Over the image the request names.** The proof is computed over
//!    the image the request's Image names, as the provider can supply
//!    it — from the caller's Store, or from a registry it is
//!    configured with that one of the Image's references names — with
//!    the same function its own nonce checks use. How the provider
//!    holds those bytes is not stated.
//! 2. **Never another's.** The provider never computes it over another
//!    identity's Store, nor over images it holds only because other
//!    clients supplied them to runs.
//! 3. **Not found.** An image the provider cannot supply, or a
//!    platform that breaks the rule of
//!    [`oci::prove`](crate::shared::containers::oci::prove) — present
//!    exactly when the digest names an index, and then a manifest
//!    that index lists — is
//!    [`NotFound`](server::response::Frame::NotFound): the caller
//!    cannot learn the proof of an image it could not obtain.
//! 4. **Unsupported.** A provider with neither a Store nor any
//!    registry answers
//!    [`Unsupported`](server::response::Frame::Unsupported).
//! 5. **Exactly one answer, then the finish.**
//!
//! Split by who SENDS, as everywhere else: the ask is in [`client`],
//! the answer in [`server`].

pub mod client;
pub mod server;
