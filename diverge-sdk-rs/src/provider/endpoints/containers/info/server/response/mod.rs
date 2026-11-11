//! The image info response.
//!
//! [`Frame`] is what the one response frame holds: the document, that
//! the image ships none, that no copy of the image was found, or a
//! failure.

mod frame;

pub use frame::*;
