//! Pulling an image the caller holds: two fetches by digest.
//!
//! When a run names an
//! [`Image::Client`](crate::shared::containers::request::Image::Client),
//! the provider runs a registry — the read side of the OCI
//! Distribution API, on its own loopback — and its container runtime
//! pulls from it as from any registry. Behind that registry is a store
//! keyed by digest, and whatever the store does not hold the provider
//! asks the caller for here, on the run scope, exactly as it asks for
//! a mounted file it does not hold: a [`manifest`] by digest, a
//! [`blob`] by digest, bytes until the finish, or an empty finish for
//! a digest the caller does not have.
//!
//! # Two, and only two
//!
//! What a runtime does to pull a pinned image, and what each step
//! needs from the caller:
//!
//! | the runtime sends | needs from the caller |
//! |-------------------|-----------------------|
//! | `GET /v2/` | nothing |
//! | `GET` or `HEAD` a manifest by digest | the manifest, if not held |
//! | the manifest is an index: its platform's manifest by digest | that manifest, if not held |
//! | `HEAD` a blob | nothing — the size is in the manifest's descriptor |
//! | `GET` a blob, with or without `Range` | the blob, if not held; ranges come from the store |
//! | auth, redirects, tags, referrers, any push | never happens |
//!
//! Everything HTTP adds — `Accept`, `Range`, `HEAD`, `Content-Type`,
//! `Docker-Content-Digest` — is the concern of whoever SERVES the
//! registry, and that is the provider. So none of it crosses the
//! wire, and the caller needs no HTTP at all: it needs a store of
//! manifests and blobs indexed by digest, which is what an image is
//! once it has been saved anywhere.
//!
//! Tags never cross either. A client image is pinned by digest, and
//! every reference inside a manifest is a digest, so there is no step
//! at which a name has to be resolved.
//!
//! # No offsets, no errors
//!
//! A blob is fetched whole, once, and every range the runtime asks
//! for is served out of the provider's store: a partial blob the
//! provider held would be a blob it cannot verify. And there is no
//! error vocabulary, as on the other fetches — nothing an error could
//! say would change what the provider does next, which is not run the
//! container. What the caller sees is the run scope's own `Error`.
//!
//! This replaced a tunnel that carried the runtime's HTTP to the
//! caller verbatim. That was right about pushes and wrong about
//! pulls: a pull is two fetches by identity, and saying which is
//! being asked — the shape [`mcp`](crate::shared::mcp) took — is
//! what lets the provider cache, verify and resume.

pub mod blob;
pub mod manifest;
