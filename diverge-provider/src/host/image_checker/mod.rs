//! Whether the provider could supply an image without the caller's
//! help: the protocol's `ImageChecker`, supplied here.
//!
//! An image is a digest and references, and a run gets the bytes
//! from wherever the provider finds them — the caller's own images,
//! its own store, a referenced registry it uses, or the caller. A
//! check asks the same question with the caller left out, and says
//! where to look: the caller's own images, built on this provider
//! and kept in the client image store, found by digest for the
//! identity asking and for no other; and the referenced registries
//! the configuration lists, asked under each reference's name, a
//! reference naming a registry not listed ignored. The provider's
//! own cache is never asked, whatever a request says: an image it
//! pulled for one caller is no answer to another, and a check
//! reveals nothing of what the cache holds. The caller's own images,
//! when asked, are looked in first, and an image there ends the
//! question; only then are the referenced registries asked, all at once, each
//! with the credential listed, and any yes is the answer. A
//! reference whose name is not a repository path is the failure to
//! answer. The identity asking chooses only which images are its
//! own: the configuration has no image policy per caller beyond
//! that, and every caller gets the same answer of the rest.
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
