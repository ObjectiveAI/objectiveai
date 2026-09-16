//! Whether the provider would supply an image: the SDK's
//! `ImageChecker`, supplied by this crate.
//!
//! A `server` image — one a container request names by repository
//! and digest and leaves the provider to produce — is, in this
//! provider, one podman already holds on the system. Nothing is
//! pulled for one and no registry is consulted, so the check is the
//! store's own answer: `podman image exists`. The name arrives
//! without a registry host and the store may hold the image under
//! one, so every configured registry's host is tried before the bare
//! name, all at once, and any yes is the answer. A yes is remembered
//! for the provider's life, since a digest is immutable and what the
//! store holds under it cannot change; a no is never remembered,
//! since the image may be loaded later. The identity asking is not
//! consulted: the configuration has no image policy per caller, and
//! every caller gets the store's answer.
//!
//! [`ImageChecker`] is the checker and [`Error`] what it fails with —
//! podman not answering, which is a failure to answer and never a no.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod image_checker;

pub use error::*;
pub use image_checker::*;
