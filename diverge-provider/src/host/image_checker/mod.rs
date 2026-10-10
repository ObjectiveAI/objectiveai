//! Whether the provider could supply an image without the caller's
//! help: the protocol's `ImageChecker`, supplied here.
//!
//! An image is a digest and references, and a run gets the bytes
//! from wherever the provider finds them — its own store, a
//! referenced registry it uses, or the caller. A check asks the same
//! question with the caller left out, since the caller knows what it
//! holds: is the digest in the store, under any name or none, or does
//! any referenced registry the configuration lists serve it under the
//! reference's name? The store is asked first, and an image there
//! ends the question; only then are the referenced registries asked,
//! all at once, each with the credential listed — a reference naming
//! a registry not listed is ignored — and any yes is the answer. A
//! reference whose name is not a repository path is the failure to
//! answer. The identity asking
//! is not consulted: the configuration has no image policy per
//! caller, and every caller gets the same answer.
//!
//! [`ImageChecker`] is the checker and [`Error`] what it fails with —
//! podman not starting, which is a failure to answer and never a no.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod image_checker;

pub use error::*;
pub use image_checker::*;
