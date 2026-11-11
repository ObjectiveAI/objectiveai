//! Image build response data.
//!
//! [`Frame`] is what comes back on channel `0` — a line of the
//! build's output, the image built, the word that the caller owns it
//! already, the word that there was no room, or a failure — and
//! [`Built`] and [`Exists`] are what the second and the third carry.

mod built;
mod exists;
mod frame;

pub use built::*;
pub use exists::*;
pub use frame::*;
