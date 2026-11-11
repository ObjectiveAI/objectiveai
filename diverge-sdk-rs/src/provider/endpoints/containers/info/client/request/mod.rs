//! Image info request data.
//!
//! What a caller hands a provider to read an image's DIVERGE.md: the
//! [`Frame`], which names the image and nothing else.

mod frame;

pub use frame::*;
