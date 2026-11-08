//! Image listing response data.
//!
//! [`Frame`] is what comes back on channel `0` — an image added,
//! changed or removed, the word that the listing is whole, or a
//! failure to list — and [`Image`] is what the first three carry.

mod frame;
mod image;

pub use frame::*;
pub use image::*;
