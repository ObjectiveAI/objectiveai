//! Whether the provider would supply an image: the SDK's
//! `ImageChecker`, supplied by this crate.
//!
//! A `server` image — one a container request names by repository
//! and digest and leaves the provider to produce — is, in this
//! provider, one the configuration lists under
//! `containers.server_images`. The check is a lookup in that list:
//! a listed pair is available, any other is unavailable, and nothing
//! is asked of podman or of any registry. Nothing is remembered,
//! since there is nothing to learn: the list is the answer. The
//! identity asking is not consulted: the list has no policy per
//! caller, and every caller gets the same answer.
//!
//! [`ImageChecker`] is the checker and [`Error`] what it fails with,
//! which is nothing: a lookup always answers.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod image_checker;

pub use error::*;
pub use image_checker::*;
