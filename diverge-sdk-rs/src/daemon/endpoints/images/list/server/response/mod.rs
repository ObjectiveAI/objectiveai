//! The list response: the images, one each, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one [`Image`],
//! forbidden, or a failure. [`Image`] is what one is.

mod frame;
mod image;

pub use frame::*;
pub use image::*;
