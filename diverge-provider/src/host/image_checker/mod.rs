//! Whether the provider could supply an image without the caller's
//! help: the protocol's `ImageChecker`, supplied here.
//!
//! An image is a name and a digest, and a run gets the bytes from
//! wherever the provider finds them — its own store, a registry it
//! uses, or the caller. A check asks the same question with the
//! caller left out, since the caller knows what it holds: is the
//! digest in the store, under any name or none, or does any registry
//! the configuration lists serve the pair? The store is asked first,
//! and an image there ends the question; only then are the
//! registries asked, all at once, each with the credential listed,
//! and any yes is the answer. The identity asking
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
