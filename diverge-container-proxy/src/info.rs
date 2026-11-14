//! The image's `/DIVERGE.md`, where the container has it.
//!
//! The proxy answers the document to nobody: what an image's
//! arguments may be is read of the image, on the provider's
//! `images::read`, and not of a container of it. The proxy reads the
//! file for one thing of its own, whether the image declares a
//! frontend, and this is where the file is.

/// Where every image puts its document: the root of its filesystem,
/// which is the root of the container's.
pub const PATH: &str = "/DIVERGE.md";
