//! Whether the provider could supply an image without the caller's
//! help: the SDK's `ImageChecker`, supplied by this crate.
//!
//! An image is a name and a digest, and a run gets the bytes from
//! wherever the provider finds them — its own store, a registry it
//! uses, or the caller. A check asks the same question with the
//! caller left out, since the caller knows what it holds: is the
//! pair one the configuration lists under `containers.server_images`,
//! or does any registry the configuration lists serve it? The list
//! is a lookup; the registries are asked all at once, each with the
//! credential listed, and any yes is the answer. The identity asking
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
